#!/usr/bin/env python3
"""Offline source-release verifier for Omni bootstrap milestones."""
from __future__ import annotations
from pathlib import Path
import argparse
import json
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
EXPECTED_VERSION = "0.2.0.1"
EXPECTED_CARGO_SEMVER_BASE = "0.2.0"
_args = argparse.ArgumentParser(description="Verify an Omni source tree")
_args.add_argument("--worktree", action="store_true", help="allow local target/vendor directories used for offline qualification")
ARGS = _args.parse_args()
errors: list[str] = []
notes: list[str] = []


def fail(msg: str) -> None:
    errors.append(msg)


def load_toml(path: Path):
    try:
        return tomllib.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:
        fail(f"invalid TOML {path.relative_to(ROOT)}: {exc}")
        return {}

def load_json(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:
        fail(f"invalid JSON {path.relative_to(ROOT)}: {exc}")
        return {}


for rel in (
    "RELEASE_MANIFEST.json",
    "SOURCE_QUALIFICATION.json",
    "release/BINARY_QUALIFICATION.json",
    "release/FINAL_OFFLINE_VALIDATION.json",
):
    data = load_json(ROOT / rel)
    if str(data.get("version")) != EXPECTED_VERSION:
        fail(f"stale current release metadata in {rel}: expected {EXPECTED_VERSION}, got {data.get('version')!r}")

workspace_toml = load_toml(ROOT / "Cargo.toml")
workspace_metadata = workspace_toml.get("workspace", {}).get("metadata", {}).get("omni", {})
if workspace_metadata.get("project-version") != EXPECTED_VERSION:
    fail(f"workspace Omni project-version metadata mismatch: {workspace_metadata.get('project-version')!r} != {EXPECTED_VERSION!r}")
if workspace_metadata.get("version-scheme") != "stable.major.minor.patch":
    fail("workspace Omni version-scheme metadata is missing or incorrect")
workspace = workspace_toml.get("workspace", {})
members = workspace.get("members", [])
if not members:
    fail("workspace has no members")

packages: dict[str, tuple[Path, str]] = {}
for rel in members:
    manifest = ROOT / rel / "Cargo.toml"
    if not manifest.exists():
        fail(f"missing workspace manifest: {rel}/Cargo.toml")
        continue
    data = load_toml(manifest)
    package = data.get("package", {})
    name = package.get("name")
    version = package.get("version")
    if not name or not version:
        fail(f"workspace manifest missing package name/version: {rel}")
        continue
    packages[name] = (manifest, version)
    metadata = package.get("metadata", {}).get("omni", {})
    if metadata.get("project-version") != EXPECTED_VERSION:
        fail(f"workspace package {name} missing Omni project-version {EXPECTED_VERSION} metadata")
    if metadata.get("version-scheme") != "stable.major.minor.patch":
        fail(f"workspace package {name} missing Omni four-part version-scheme metadata")
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        for dep_name, spec in data.get(section, {}).items():
            if isinstance(spec, dict) and "path" in spec:
                target = (manifest.parent / spec["path"]).resolve()
                if not target.exists():
                    fail(f"missing path dependency {dep_name}: {target}")

lock = load_toml(ROOT / "Cargo.lock")
lock_packages = {p.get("name"): p for p in lock.get("package", []) if p.get("name")}
for name, (_, version) in packages.items():
    entry = lock_packages.get(name)
    if not entry:
        fail(f"Cargo.lock missing workspace package {name}")
    elif entry.get("version") != version:
        fail(f"Cargo.lock version mismatch for {name}: {entry.get('version')} != {version}")

# Resolve top-level Rust module declarations. This catches the exact class of
# corruption present in the uploaded source snapshot.
for src in ROOT.glob("crates/*/src/**/*.rs"):
    text = src.read_text(encoding="utf-8", errors="replace")
    for match in re.finditer(r"(?m)^\s*(?:pub\s+)?mod\s+([A-Za-z_]\w*)\s*;", text):
        prefix = text[:match.start()]
        depth = prefix.count("{") - prefix.count("}")
        if depth != 0:
            continue
        name = match.group(1)
        if not (src.parent / f"{name}.rs").exists() and not (src.parent / name / "mod.rs").exists():
            fail(f"unresolved module {name} declared by {src.relative_to(ROOT)}")

# Lean source releases should not carry generated Cargo output or a vendored
# registry cache. Local offline qualification worktrees are allowed to contain
# both when --worktree is selected.
if not ARGS.worktree:
    for directory_name in ("target", "vendor"):
        directory = ROOT / directory_name
        if directory.is_dir():
            fail(f"non-source directory must not be shipped in lean source release: {directory_name}")

# Fuzz *source* is valuable and small; corpora/artifacts are disposable.
for required in (
    ROOT / "crates/fuzz_harness/src/main.rs",
    ROOT / "crates/omni-fuzz/fuzz_targets/lexer_parser.rs",
    ROOT / "crates/omni-fuzz/fuzz_targets/serialization.rs",
):
    if not required.exists():
        fail(f"missing fuzz source: {required.relative_to(ROOT)}")

native_manifest = load_toml(ROOT / "crates/codegen-native/Cargo.toml")
native_deps = set(native_manifest.get("dependencies", {}))
if native_deps != {"lir"}:
    fail(f"owned native emitter must depend only on lir at this milestone; got {sorted(native_deps)}")
native_src = (ROOT / "crates/codegen-native/src/lib.rs").read_text(encoding="utf-8")
for forbidden in ("cranelift", "inkwell", "llvm_sys", "wasmtime", "libloading"):
    if re.search(rf"\b{re.escape(forbidden)}\b", native_src, re.I):
        fail(f"owned native emitter contains forbidden backend/runtime dependency token: {forbidden}")
if "emit_elf" not in native_src or "\\x7FELF" not in native_src:
    fail("owned native emitter does not contain its ELF writer")

stage0 = (ROOT / "crates/omni-stage0/src/main.rs").read_text(encoding="utf-8")
if "Backend::Native" not in stage0:
    fail("stage0 CLI does not select the Native backend")
if '"run-jit"' not in stage0:
    fail("development JIT must be explicit as run-jit")


# Canonical installation must not drag development/oracle backends into the
# default dependency closure. This keeps clean builds materially smaller.
compiler_manifest = load_toml(ROOT / "crates/omni-compiler/Cargo.toml")
compiler_deps = compiler_manifest.get("dependencies", {})
for dep_name in ("codegen-cranelift", "codegen-wasm", "codegen-llvm"):
    spec = compiler_deps.get(dep_name)
    if not isinstance(spec, dict) or not spec.get("optional", False):
        fail(f"development backend {dep_name} must be optional in omni-compiler")
stage_manifest = load_toml(ROOT / "crates/omni-stage0/Cargo.toml")
if stage_manifest.get("features", {}).get("default", None) != []:
    fail("omni-stage0 default feature set must stay empty for the minimal native install")
gitignore = (ROOT / ".gitignore").read_text(encoding="utf-8")
if re.search(r"(?m)^/Cargo\.lock$", gitignore):
    fail("Cargo.lock must be retained for reproducible toolchain releases")


# Cumulative v0.1.2 scalar-control-flow release obligations.
control_flow = ROOT / "crates/omni-compiler/src/control_flow.rs"
if not control_flow.is_file():
    fail("missing source-level control-flow legality pass")
else:
    cf_text = control_flow.read_text(encoding="utf-8")
    for required in ("TYPE_BREAK_OUTSIDE_LOOP", "TYPE_CONTINUE_OUTSIDE_LOOP"):
        if required not in cf_text:
            fail(f"control-flow legality pass does not enforce {required}")
mir_src = (ROOT / "crates/omni-compiler/src/mir.rs").read_text(encoding="utf-8")
for required in ("LoopContext", "validate_control_flow", "continue_target", "break_target"):
    if required not in mir_src:
        fail(f"MIR control-flow hardening marker missing: {required}")
if "resolve_omni_entry" not in stage0 or "default_native_output" not in stage0:
    fail("stage0 CLI must support file/project native entry resolution")

conformance_manifest = ROOT / "conformance/native_scalar_v0_1_2/manifest.json"
try:
    suite = json.loads(conformance_manifest.read_text(encoding="utf-8"))
except Exception as exc:
    fail(f"invalid native conformance manifest: {exc}")
    suite = {}
case_names = set()
for case in suite.get("cases", []):
    name = case.get("name")
    rel = case.get("file")
    if not name or name in case_names:
        fail(f"invalid/duplicate native conformance case name: {name!r}")
    case_names.add(name)
    if not rel or not (conformance_manifest.parent / rel).exists():
        fail(f"native conformance case source is missing: {rel!r}")
for required in (
    "arithmetic", "function_call", "nested_call_alignment", "if_else",
    "while_break_continue", "project_run", "project_build", "checked_overflow",
    "break_outside_loop", "continue_outside_loop",
):
    if required not in case_names:
        fail(f"native conformance case missing: {required}")
notes.append(f"native scalar conformance cases: {len(case_names)}")

# Cumulative v0.1.3 local aggregate/data-layout obligations.
layout_manifest = ROOT / "conformance/native_layout_v0_1_3/manifest.json"
try:
    layout_suite = json.loads(layout_manifest.read_text(encoding="utf-8"))
except Exception as exc:
    fail(f"invalid v0.1.3 layout conformance manifest: {exc}")
    layout_suite = {}
layout_names = set()
for case in layout_suite.get("cases", []):
    name = case.get("name")
    rel = case.get("file")
    if not name or name in layout_names:
        fail(f"invalid/duplicate v0.1.3 layout conformance case name: {name!r}")
    layout_names.add(name)
    if not rel or not (layout_manifest.parent / rel).exists():
        fail(f"v0.1.3 layout conformance case source is missing: {rel!r}")
for required in (
    "struct_declaration_order", "tuple_index", "array_dynamic_index",
    "array_bounds_fault", "slice_local_view", "enum_payload_match",
    "enum_fieldless_match", "enum_non_exhaustive", "enum_wrong_arity",
    "aggregate_argument_abi_transition",
):
    if required not in layout_names:
        fail(f"v0.1.3 cumulative layout conformance case missing: {required}")
notes.append(f"native layout v0.1.3 conformance cases: {len(layout_names)}")

# v0.1.4.1 cross-function value ABI / String / Bytes obligations.
value_manifest = ROOT / "conformance/native_value_abi_v0_1_4/manifest.json"
try:
    value_suite = json.loads(value_manifest.read_text(encoding="utf-8"))
except Exception as exc:
    fail(f"invalid v0.1.4.1 value-ABI conformance manifest: {exc}")
    value_suite = {}
value_names = set()
for case in value_suite.get("cases", []):
    name = case.get("name")
    rel = case.get("file")
    if not name or name in value_names:
        fail(f"invalid/duplicate v0.1.4.1 value-ABI conformance case name: {name!r}")
    value_names.add(name)
    if not rel or not (value_manifest.parent / rel).exists():
        fail(f"v0.1.4.1 value-ABI conformance case source is missing: {rel!r}")
for required in (
    "struct_argument", "struct_return", "enum_roundtrip", "string_roundtrip",
    "string_runtime_print", "bytes_binary_roundtrip", "bytes_dynamic_index",
    "bytes_bounds_fault", "utf8_string_index_fail_closed",
    "aggregate_mutation_still_deferred",
):
    if required not in value_names:
        fail(f"v0.1.4.1 value-ABI conformance case missing: {required}")
notes.append(f"native value ABI v0.1.4.1 conformance cases: {len(value_names)}")

layout_lir = (ROOT / "crates/omni-compiler/src/codegen_lir.rs").read_text(encoding="utf-8")
for marker in ("AggregateStorage", "EnumInit", "SliceAccess", "ValueAbi", "abi_for_annotation", "StringRef", "BytesRef", "PrintBytes", "LoadByteIndex"):
    if marker not in layout_lir:
        fail(f"v0.1.4.1 LIR lowering marker missing: {marker}")
for marker in ("BOUNDS_FAULT_EXIT", "local_offset_disp", "LoadIndex", "LoadByteIndex"):
    if marker not in native_src:
        fail(f"v0.1.4.1 native value/layout marker missing: {marker}")
lir_src = (ROOT / "crates/lir/src/lib.rs").read_text(encoding="utf-8")
for marker in ("Ptr(u32)", "StringRef", "BytesRef", "PrintBytes", "LoadByteIndex"):
    if marker not in lir_src:
        fail(f"v0.1.4.1 typed LIR marker missing: {marker}")
for marker in ("param_types", "return_type", "ConstBytes"):
    if marker not in mir_src:
        fail(f"v0.1.4.1 typed MIR marker missing: {marker}")
stdlib_src = (ROOT / "crates/omni-stdlib/src/lib.rs").read_text(encoding="utf-8")
for marker in ("trait CellAllocator", "BootstrapCellAllocator", "OmniCellVector", "failed_collection_growth_preserves_existing_state"):
    if marker not in stdlib_src:
        fail(f"v0.1.4.1 allocator/collection foundation marker missing: {marker}")
runner_src = (ROOT / "scripts/native-conformance.py").read_text(encoding="utf-8")
for marker in ("text=False", "stdout_hex", "bytes.fromhex"):
    if marker not in runner_src:
        fail(f"binary-safe conformance runner marker missing: {marker}")
for required in (
    ROOT / "crates/omni-compiler/tests/aggregate_native_v0_1_3.rs",
    ROOT / "docs/MILESTONE_0.1.3_NATIVE_DATA_LAYOUT_I.md",
    ROOT / "crates/omni-compiler/tests/value_abi_v0_1_4.rs",
    ROOT / "docs/MILESTONE_0.1.4_STRING_BYTE_VALUE_ABI_COLLECTIONS_FOUNDATION.md",
):
    if not required.is_file():
        fail(f"missing cumulative/current layout-ABI evidence: {required.relative_to(ROOT)}")

# Detect duplicate field names in simple Rust struct bodies.
for src in ROOT.glob("crates/*/src/**/*.rs"):
    text = src.read_text(encoding="utf-8", errors="replace")
    for match in re.finditer(r"\bstruct\s+([A-Za-z_]\w*)(?:<[^{};]*>)?\s*\{", text):
        i = match.end(); depth = 1; j = i
        in_string = False; escape = False
        while j < len(text) and depth:
            ch = text[j]
            if in_string:
                if escape: escape = False
                elif ch == "\\": escape = True
                elif ch == '"': in_string = False
            else:
                if ch == '"': in_string = True
                elif ch == "{": depth += 1
                elif ch == "}": depth -= 1
            j += 1
        if depth:
            continue
        body = text[i:j-1]
        names = re.findall(r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?([A-Za-z_]\w*)\s*:", body)
        duplicates = sorted({n for n in names if names.count(n) > 1})
        if duplicates:
            fail(f"duplicate struct field(s) {duplicates} in {src.relative_to(ROOT)}::{match.group(1)}")

version_set = sorted({version for _, version in packages.values()})
if version_set != [EXPECTED_CARGO_SEMVER_BASE]:
    fail(f"workspace Cargo SemVer base set must be exactly {EXPECTED_CARGO_SEMVER_BASE}; got {version_set}")
notes.append(f"workspace members: {len(packages)}")
notes.append(f"workspace Cargo SemVer bases: {', '.join(version_set)}; Omni project version: {EXPECTED_VERSION}")
project_rust_files = [
    p for p in ROOT.rglob("*.rs")
    if p.relative_to(ROOT).parts and p.relative_to(ROOT).parts[0] not in {"target", "vendor", ".git"}
]
notes.append(f"Rust source files: {len(project_rust_files)}")
notes.append("native emitter dependency boundary: lir only")
notes.append("Cranelift/Wasm/LLVM excluded from default compiler feature closure")
notes.append("fuzz source retained; Cargo target output excluded")

toolchain = (ROOT / "rust-toolchain").read_text(encoding="utf-8")
if 'channel = "1.97.1"' not in toolchain:
    fail("rust-toolchain must pin Rust 1.97.1")
else:
    notes.append("Rust bootstrap toolchain pinned: 1.97.1")
build_script = (ROOT / "crates/omni-compiler/src/package/build_script.rs").read_text(encoding="utf-8")
if "not qualified" not in build_script.lower():
    fail("build.omni boundary must fail closed in the current release")
else:
    notes.append("build.omni remains fail-closed in the qualified baseline")
for evidence in (
    ROOT / "docs/LINEAGE_REMEDIATION_AUDIT_0.0.1_TO_0.1.2.md",
    ROOT / "release/LINEAGE_REMEDIATION_ISSUES.json",
    ROOT / "release/LINEAGE_REQUIREMENTS_0.0.1_TO_0.1.2.csv",
):
    if not evidence.is_file():
        fail(f"lineage remediation evidence missing: {evidence.relative_to(ROOT)}")
# Historical parser/CST/formatter closure must remain present in source releases.
for rel in (
    "crates/omni-compiler/tests/v0_1_2_parser_formatter_closure.rs",
    "crates/omni-compiler/tests/parser_parallel.rs",
    "crates/omni-compiler/tests/lexer_comment_edition1.rs",
    "release/HISTORICAL_MILESTONE_CLOSURE_0.0.1_TO_0.1.2.md",
    "scripts/fuzz-qualification.sh",
    "scripts/fuzz-qualification.ps1",
):
    if not (ROOT / rel).is_file():
        fail(f"historical closure file missing: {rel}")
parser_source = (ROOT / "crates/omni-compiler/src/parser.rs").read_text(encoding="utf-8")
if re.search(r"parse\(\)\.unwrap_or\((?:4|1000|4711|1000000)\)", parser_source):
    fail("parser contains a silent numeric parse default")
formatter_source = (ROOT / "crates/omni-compiler/src/formatter.rs").read_text(encoding="utf-8")
for marker in ("format_visibility", "format_contract_inline", "return {};\\n", "break;\\n"):
    if marker not in formatter_source:
        fail(f"formatter closure marker missing: {marker}")
notes.append("historical parser/CST/formatter closure present")
notes.append("lineage remediation evidence present")

print("Omni source-release verification")
for note in notes:
    print(f"  OK  {note}")
if errors:
    for error in errors:
        print(f"  ERR {error}")
    print(f"FAILED: {len(errors)} error(s)")
    sys.exit(1)
print("PASS")
