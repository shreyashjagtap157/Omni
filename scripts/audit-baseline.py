#!/usr/bin/env python3
"""Adversarial offline audit for the Omni 0.0.1.0 -> 0.1.4.1 qualified lineage.

This gate intentionally checks claims that `verify-source.py` does not: the
minimal default dependency closure, fail-closed experimental boundaries,
historical compatibility corpus, active-source placeholders, and broad Rust
source structural integrity. It is not a substitute for Cargo/rustc.
"""

from __future__ import annotations

from pathlib import Path
import argparse
import json
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
_args = argparse.ArgumentParser(description="Audit the Omni v0.1.4.1 lineage baseline")
_args.add_argument("--worktree", action="store_true", help="allow local build/vendor directories while auditing active project sources")
ARGS = _args.parse_args()
EXPECTED_VERSION = "0.2.0.1"
EXPECTED_CARGO_SEMVER_BASE = "0.2.0"
errors: list[str] = []
notes: list[str] = []


def fail(message: str) -> None:
    errors.append(message)


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except Exception as exc:
        fail(f"cannot read {path.relative_to(ROOT)}: {exc}")
        return ""


def toml(path: Path) -> dict:
    try:
        return tomllib.loads(read(path))
    except Exception as exc:
        fail(f"invalid TOML {path.relative_to(ROOT)}: {exc}")
        return {}


def json_file(path: Path) -> dict:
    try:
        return json.loads(read(path))
    except Exception as exc:
        fail(f"invalid JSON {path.relative_to(ROOT)}: {exc}")
        return {}


# Current release metadata must advance with the executable milestone. Historical
# v0.1.2 evidence lives in explicitly versioned documents; unversioned release
# records always describe the current source tree.
for rel in (
    "RELEASE_MANIFEST.json",
    "SOURCE_QUALIFICATION.json",
    "release/BINARY_QUALIFICATION.json",
    "release/FINAL_OFFLINE_VALIDATION.json",
):
    data = json_file(ROOT / rel)
    if str(data.get("version")) != EXPECTED_VERSION:
        fail(f"stale current release metadata in {rel}: expected {EXPECTED_VERSION}, got {data.get('version')!r}")
release_manifest = json_file(ROOT / "RELEASE_MANIFEST.json")
workspace_toml = toml(ROOT / "Cargo.toml")
workspace_metadata = workspace_toml.get("workspace", {}).get("metadata", {}).get("omni", {})
if workspace_metadata.get("project-version") != EXPECTED_VERSION:
    fail(f"workspace Omni project-version metadata mismatch: {workspace_metadata.get('project-version')!r} != {EXPECTED_VERSION!r}")
if workspace_metadata.get("version-scheme") != "stable.major.minor.patch":
    fail("workspace Omni version-scheme metadata is missing or incorrect")
layout_record = release_manifest.get("conformance", {}).get("native_layout_v0_1_3", {})
if layout_record.get("cases") != 10 or layout_record.get("execution") not in {"PENDING", "PASS 10/10"}:
    fail("RELEASE_MANIFEST.json does not record the 10-case cumulative v0.1.3 layout corpus")
value_record = release_manifest.get("conformance", {}).get("native_value_abi_v0_1_4", {})
if value_record.get("cases") != 10 or value_record.get("execution") not in {"PENDING", "PASS 10/10"}:
    fail("RELEASE_MANIFEST.json does not record the 10-case v0.1.4.1 value-ABI corpus")


def strip_rust_noncode(text: str) -> str:
    """Replace comments/string/char bodies with spaces while preserving newlines."""
    out = list(text)
    n = len(text)
    i = 0
    block_depth = 0
    while i < n:
        if block_depth:
            if text.startswith("/*", i):
                out[i:i+2] = "  "
                block_depth += 1
                i += 2
            elif text.startswith("*/", i):
                out[i:i+2] = "  "
                block_depth -= 1
                i += 2
            else:
                if text[i] != "\n":
                    out[i] = " "
                i += 1
            continue
        if text.startswith("//", i):
            j = text.find("\n", i)
            if j < 0:
                j = n
            for k in range(i, j):
                out[k] = " "
            i = j
            continue
        if text.startswith("/*", i):
            out[i:i+2] = "  "
            block_depth = 1
            i += 2
            continue

        # Rust raw strings: r"", r#""#, br#""#, cr#""#.
        raw_match = re.match(r'(?:b|c)?r(#{0,255})"', text[i:])
        if raw_match:
            hashes = raw_match.group(1)
            opener_len = raw_match.end()
            close = '"' + hashes
            j = text.find(close, i + opener_len)
            end = n if j < 0 else j + len(close)
            for k in range(i, end):
                if text[k] != "\n":
                    out[k] = " "
            i = end
            continue

        # Normal strings / byte strings / C strings.
        prefix_len = 0
        if text.startswith('b"', i) or text.startswith('c"', i):
            prefix_len = 1
        if text[i + prefix_len:i + prefix_len + 1] == '"':
            j = i + prefix_len + 1
            escaped = False
            while j < n:
                ch = text[j]
                if escaped:
                    escaped = False
                elif ch == "\\":
                    escaped = True
                elif ch == '"':
                    j += 1
                    break
                j += 1
            for k in range(i, min(j, n)):
                if text[k] != "\n":
                    out[k] = " "
            i = j
            continue

        # Character literals have a closing quote immediately after exactly
        # one character/escape. Lifetimes such as `'a` therefore remain code.
        if text[i] == "'":
            char_match = re.match(r"'(?:\\(?:.|u\{[0-9A-Fa-f_]+\}|x[0-9A-Fa-f]{2})|[^\\'\n])'", text[i:])
            if char_match:
                j = i + char_match.end()
                for k in range(i, j):
                    out[k] = " "
                i = j
                continue
        i += 1
    if block_depth:
        fail("unterminated block comment detected while scanning Rust source")
    return "".join(out)


def check_delimiters(path: Path) -> None:
    code = strip_rust_noncode(read(path))
    pairs = {')': '(', ']': '[', '}': '{'}
    stack: list[tuple[str, int]] = []
    for i, ch in enumerate(code):
        if ch in "([{":
            stack.append((ch, i))
        elif ch in ")]}":
            if not stack or stack[-1][0] != pairs[ch]:
                line = code.count("\n", 0, i) + 1
                fail(f"delimiter mismatch in {path.relative_to(ROOT)}:{line}: unexpected {ch}")
                return
            stack.pop()
    if stack:
        ch, i = stack[-1]
        line = code.count("\n", 0, i) + 1
        fail(f"unclosed delimiter {ch} in {path.relative_to(ROOT)}:{line}")


workspace = toml(ROOT / "Cargo.toml").get("workspace", {})
members = workspace.get("members", [])
default_members = workspace.get("default-members", [])
if not members or not default_members:
    fail("workspace members/default-members must both be declared")

member_by_path: dict[Path, tuple[str, dict]] = {}
name_to_path: dict[str, Path] = {}
for rel in members:
    manifest = ROOT / rel / "Cargo.toml"
    data = toml(manifest)
    pkg = data.get("package", {})
    name = pkg.get("name")
    version = pkg.get("version")
    if not name:
        fail(f"missing package name: {rel}")
        continue
    if version != EXPECTED_CARGO_SEMVER_BASE:
        fail(f"workspace version drift: {name}={version!r}, expected {EXPECTED_CARGO_SEMVER_BASE}")
    member_by_path[manifest.parent.resolve()] = (name, data)
    name_to_path[name] = manifest.parent.resolve()

# Resolve path-dependency closure from default members.
def path_dependencies(pkg_path: Path, data: dict) -> set[Path]:
    result: set[Path] = set()
    for section in ("dependencies", "build-dependencies"):
        for _dep, spec in data.get(section, {}).items():
            if isinstance(spec, dict) and spec.get("path") and not spec.get("optional", False):
                result.add((pkg_path / spec["path"]).resolve())
    return result

closure: set[Path] = set()
queue = [(ROOT / rel).resolve() for rel in default_members]
while queue:
    pkg_path = queue.pop()
    if pkg_path in closure:
        continue
    closure.add(pkg_path)
    entry = member_by_path.get(pkg_path)
    if not entry:
        fail(f"default dependency closure reaches non-workspace path: {pkg_path}")
        continue
    _, data = entry
    queue.extend(path_dependencies(pkg_path, data) - closure)

closure_names = {member_by_path[p][0] for p in closure if p in member_by_path}
for forbidden in {
    "codegen-cranelift", "codegen-llvm", "codegen-mlir", "codegen-wasm",
    "polonius_engine_adapter", "polonius_engine_mock", "omni-selfhost", "omni-release",
}:
    if forbidden in closure_names:
        fail(f"unqualified/future package leaked into default native closure: {forbidden}")
notes.append("default native closure: " + ", ".join(sorted(closure_names)))

# No network/Git bootstrap dependency is permitted in the qualified default closure.
for pkg_path in closure:
    entry = member_by_path.get(pkg_path)
    if not entry:
        continue
    name, data = entry
    for section in ("dependencies", "build-dependencies"):
        for dep, spec in data.get(section, {}).items():
            if isinstance(spec, dict) and "git" in spec:
                fail(f"network Git dependency in default closure: {name} -> {dep}")

# Lockfile must parse and reflect every workspace package; removed historical
# heavyweight engines must not remain pinned as accidental bootstrap baggage.
lock = toml(ROOT / "Cargo.lock")
lock_entries = lock.get("package", [])
for name, pkg_path in name_to_path.items():
    # member_by_path[pkg_path] is (name, data) tuple
    stored_name, stored_data = member_by_path[pkg_path]
    version = stored_data["package"]["version"]
    if not any(p.get("name") == name and p.get("version") == version for p in lock_entries):
        fail(f"Cargo.lock missing workspace package {name} {version}")
lock_names = {p.get("name") for p in lock_entries}
for removed in ("polonius-engine", "datafrog", "inkwell", "llvm-sys"):
    if removed in lock_names:
        fail(f"archived heavy dependency still present in Cargo.lock: {removed}")
notes.append(f"Cargo.lock packages: {len(lock_entries)}")

# Original milestone closure must remain explicit so later native-hardening work
# cannot silently redefine historical version numbers.
for rel in (
    "release/HISTORICAL_MILESTONE_CLOSURE_0.0.1_TO_0.1.2.md",
    "crates/omni-compiler/tests/historical_milestone_closure.rs",
    "crates/omni-compiler/tests/parser_parallel.rs",
    "docs/adr/ADR-0001-workspace-resolver.md",
    "docs/adr/ADR-0002-workspace-structure.md",
):
    if not (ROOT / rel).exists():
        fail(f"missing historical milestone closure evidence: {rel}")

parser_text = read(ROOT / "crates/omni-compiler/src/parser.rs")
for required in ("UseScoped", "PubMod", "PubPkg", "PubCap", "PubFriend", "parse_error_set", "ContractRequires", "ContractEnsures", "ContractInvariant"):
    if required not in parser_text:
        fail(f"historical v0.1.1 parser contract missing active support marker: {required}")
formatter_text = read(ROOT / "crates/omni-compiler/src/formatter.rs")
for required in ("strict_mode", "format_program_strict", "format_visibility", "format_contract_inline"):
    if required not in formatter_text:
        fail(f"historical v0.1.2 formatter contract missing active support marker: {required}")
stage0_text = read(ROOT / "crates/omni-stage0/src/main.rs")
for required in ("--check", "--strict"):
    if required not in stage0_text:
        fail(f"historical v0.1.2 formatter CLI contract missing: {required}")

if not (ROOT / "scripts/fuzz-lexer-parser-smoke.py").exists():
    fail("missing historical v0.1.0 60-second lexer/parser smoke-fuzz gate")
for rel in ("scripts/fuzz-qualification.sh", "scripts/fuzz-qualification.ps1"):
    if not (ROOT / rel).exists():
        fail(f"missing cargo-fuzz historical qualification gate: {rel}")

# Original repository-foundation deliverables must exist in the cumulative tree.
for rel in (
    ".github/workflows/ci.yml", ".devcontainer/devcontainer.json",
    "CONTRIBUTING.md", "CODE_OF_CONDUCT.md", "SECURITY.md",
    "IMPLEMENTATION_STATUS.md", "ROADMAP.md", "docs/QUICK_START.md",
):
    if not (ROOT / rel).exists():
        fail(f"missing historical foundation deliverable: {rel}")

# v0.1.1 parser closure: current brace syntax, historical compatible spellings,
# contracts, strict numeric syntax, parallel parsing, and fail-closed malformed input.
for required in (
    "skip_parser_trivia", "ContractInvariant",
    "Expected impl body", "Expected trait body", "Expected error set body",
    "Invalid executor thread count", "Invalid deterministic task limit",
    "Invalid tensor dimension", "Invalid SIMD width", "Invalid debug port",
    "Unsupported function contract attribute",
):
    if required not in parser_text:
        fail(f"v0.1.1 parser remediation marker missing: {required}")
parser_utils_text = read(ROOT / "crates/omni-compiler/src/parser_utils.rs")
if "parse_files_parallel" not in parser_utils_text:
    fail("v0.1.1 deterministic parallel file parsing API is missing")
if re.search(r"parse\(\)\.unwrap_or\((?:4|1000|4711|1000000)\)", parser_text):
    fail("v0.1.1 parser still contains silent numeric parse defaults")

# v0.1.2 CST/formatter closure: exact source recovery, semantic round-trip
# tests, Edition-1 explicit semicolons on canonical AST statements, and no
# known AST-changing assignment/while-in formatting.
cst_text = read(ROOT / "crates/omni-compiler/src/cst.rs")
for required in ("source_text: Option<String>", "build_cst_from_source", "recover_source"):
    if required not in cst_text:
        fail(f"lossless CST marker missing: {required}")
formatter_closure = read(ROOT / "crates/omni-compiler/tests/v0_1_2_parser_formatter_closure.rs")
for required in (
    "assignment_formatting_does_not_turn_mutation_into_declaration",
    "while_in_formatting_preserves_while_in_ast",
    "formatter_preserves_visibility_async_and_contracts",
    "formatter_roundtrips_brace_impl_and_trait",
    "historical_linear_struct_spelling_is_preserved_semantically",
):
    if required not in formatter_closure:
        fail(f"formatter closure regression missing: {required}")
for required in (r'return {};\n', r'break;\n', r'continue;\n', r'= {};\n'):
    if required not in formatter_text:
        fail(f"canonical formatter semicolon marker missing: {required}")

# Scalar type checking must not turn malformed annotations/unification failures
# into fresh variables or silently discard unification errors.
type_checker_text = read(ROOT / "crates/omni-compiler/src/type_checker.rs")
if re.search(r"parse_type_annotation\([^\n]+\)\.unwrap_or_else", type_checker_text):
    fail("type checker still hides invalid type annotations behind fresh variables")
if "let _ = ctx.unify" in type_checker_text:
    fail("type checker still discards unification errors")
for q in ("scripts/qualify-release.sh", "scripts/qualify-release.ps1"):
    text = read(ROOT / q)
    if ("fuzz-lexer-parser-smoke.py" not in text
            or "fuzz-qualification" not in text
            or "build --workspace --locked" not in text):
        fail(f"historical qualification gates missing from {q}")

# Historical 0.0.1, v0.1.2 scalar, v0.1.3 layout, and current v0.1.4.1 ABI corpora must be real files.
for rel, required in (
    ("conformance/historical_v0_0_1/manifest.json", {"exit42", "hello", "checked_overflow", "undefined_name", "duplicate_definition"}),
    ("conformance/native_scalar_v0_1_2/manifest.json", {"arithmetic", "function_call", "nested_call_alignment", "if_else", "while_break_continue", "project_run", "project_build", "checked_overflow", "break_outside_loop", "continue_outside_loop"}),
    ("conformance/native_layout_v0_1_3/manifest.json", {"struct_declaration_order", "tuple_index", "array_dynamic_index", "array_bounds_fault", "slice_local_view", "enum_payload_match", "enum_fieldless_match", "enum_non_exhaustive", "enum_wrong_arity", "aggregate_argument_abi_transition"}),
    ("conformance/native_value_abi_v0_1_4/manifest.json", {"struct_argument", "struct_return", "enum_roundtrip", "string_roundtrip", "string_runtime_print", "bytes_binary_roundtrip", "bytes_dynamic_index", "bytes_bounds_fault", "utf8_string_index_fail_closed", "aggregate_mutation_still_deferred"}),
):
    manifest_path = ROOT / rel
    manifest = json_file(manifest_path)
    cases = manifest.get("cases", [])
    names = [c.get("name") for c in cases]
    if len(names) != len(set(names)):
        fail(f"duplicate conformance case names in {rel}")
    missing = required - set(names)
    if missing:
        fail(f"missing required conformance cases in {rel}: {sorted(missing)}")
    for case in cases:
        target = manifest_path.parent / str(case.get("file", ""))
        if not target.exists():
            fail(f"missing conformance input {target.relative_to(ROOT)}")
    notes.append(f"{manifest.get('suite', rel)} cases: {len(cases)}")

# Experimental backend names must fail closed at this release rather than alias
# another implementation.
for rel, marker in (
    ("crates/codegen-cranelift/src/lib.rs", "Cranelift JIT execution is not qualified"),
    ("crates/codegen-llvm/src/lib.rs", "LLVM execution is not qualified"),
    ("crates/omni-compiler/src/codegen_rust.rs", "not qualified"),
):
    # Only check if file exists at the default path; archived crates are excluded
    if (ROOT / rel).exists() and marker.lower() not in read(ROOT / rel).lower():
        fail(f"experimental boundary does not visibly fail closed: {rel}")

# Canonical native backend must stay independent of foreign code generators.
native_manifest = toml(ROOT / "crates/codegen-native/Cargo.toml")
if set(native_manifest.get("dependencies", {})) != {"lir"}:
    fail("codegen-native dependency boundary must be exactly {lir}")
native_src = read(ROOT / "crates/codegen-native/src/lib.rs")
for marker in ("BOUNDS_FAULT_EXIT", "LoadOffset", "StoreOffset", "LoadIndex", "LoadByteIndex", "local_offset_disp"):
    if marker not in native_src:
        fail(f"current native value/layout marker missing: {marker}")
layout_lir = read(ROOT / "crates/omni-compiler/src/codegen_lir.rs")
for marker in ("AggregateStorage", "EnumInit", "SliceAccess", "ValueAbi", "abi_for_annotation", "StringRef", "BytesRef", "PrintBytes", "LoadByteIndex"):
    if marker not in layout_lir:
        fail(f"v0.1.4.1 value-ABI lowering marker missing: {marker}")
lir_src = read(ROOT / "crates/lir/src/lib.rs")
for marker in ("Ptr(u32)", "StringRef", "BytesRef", "PrintBytes", "LoadByteIndex"):
    if marker not in lir_src:
        fail(f"v0.1.4.1 LIR value-class marker missing: {marker}")
mir_src_current = read(ROOT / "crates/omni-compiler/src/mir.rs")
for marker in ("param_types", "return_type", "ConstBytes"):
    if marker not in mir_src_current:
        fail(f"v0.1.4.1 typed MIR marker missing: {marker}")
stdlib_src = read(ROOT / "crates/omni-stdlib/src/lib.rs")
for marker in ("trait CellAllocator", "BootstrapCellAllocator", "OmniCellVector", "failed_collection_growth_preserves_existing_state"):
    if marker not in stdlib_src:
        fail(f"v0.1.4.1 allocator/collection foundation marker missing: {marker}")
runner_src = read(ROOT / "scripts/native-conformance.py")
for marker in ("text=False", "stdout_hex", "bytes.fromhex"):
    if marker not in runner_src:
        fail(f"binary-safe v0.1.4.1 conformance runner marker missing: {marker}")
for required in (
    ROOT / "crates/omni-compiler/tests/aggregate_native_v0_1_3.rs",
    ROOT / "docs/MILESTONE_0.1.3_NATIVE_DATA_LAYOUT_I.md",
    ROOT / "crates/omni-compiler/tests/value_abi_v0_1_4.rs",
    ROOT / "docs/MILESTONE_0.1.4_STRING_BYTE_VALUE_ABI_COLLECTIONS_FOUNDATION.md",
):
    if not required.is_file():
        fail(f"missing cumulative/current layout-ABI evidence: {required.relative_to(ROOT)}")
for token in ("cranelift", "inkwell", "llvm_sys", "wasmtime", "libloading"):
    if re.search(rf"\b{re.escape(token)}\b", native_src, re.I):
        fail(f"foreign runtime/codegen token in owned native backend: {token}")

# Active Omni-source stdlib prototypes are intentionally quarantined until their
# semantics are qualified. The bootstrap collection bridge remains Rust-only in v0.1.4.1.
active_stdlib_sources = list((ROOT / "omni/stdlib").glob("*.omni"))
if active_stdlib_sources:
    fail("unqualified .omni stdlib prototypes remain active: " + ", ".join(p.name for p in active_stdlib_sources))

# Fuzz target source is source code, not disposable build output.
for rel in (
    "crates/fuzz_harness/src/main.rs",
    "crates/omni-fuzz/fuzz_targets/lexer_parser.rs",
    "crates/omni-fuzz/fuzz_targets/serialization.rs",
):
    if not (ROOT / rel).is_file():
        fail(f"missing fuzz source: {rel}")

# Lean source packages must not ship generated build output or the offline vendor
# cache. A local qualification worktree may contain both and audits them only as
# build inputs, not as active Omni project sources.
if not ARGS.worktree:
    for directory_name in ("target", "vendor"):
        directory = ROOT / directory_name
        if directory.is_dir():
            fail(f"non-source directory shipped in lean source package: {directory_name}")

# Production Rust source must not contain executable placeholder macros or
# unresolved TODO/FIXME markers. Tests may intentionally panic to assert errors.
active_src = [p for p in ROOT.glob("crates/*/src/**/*.rs") if "docs/archive" not in str(p)]
for src in active_src:
    text = read(src)
    for pattern, label in (
        (r"\btodo!\s*\(", "todo!"),
        (r"\bunimplemented!\s*\(", "unimplemented!"),
        (r"\bTODO\b", "TODO"),
        (r"\bFIXME\b", "FIXME"),
    ):
        if re.search(pattern, text):
            fail(f"active source contains {label}: {src.relative_to(ROOT)}")
    check_delimiters(src)

# Regression signatures for silent-success defects already found in the lineage.
interpreter = read(ROOT / "crates/omni-compiler/src/interpreter.rs")
for bad in (
    "if *i < 0 { 0usize }",
    "if idx < 0 { 0usize }",
    "new_vec.pop().unwrap()",
    "Value::Int(n.abs())",
    "Value::Int(a.pow(",
):
    if bad in interpreter:
        fail(f"legacy interpreter silent/panic fallback regressed: {bad}")
macro_src = read(ROOT / "crates/omni-compiler/src/macros.rs")
for bad in ("ai + bi", "ai - bi", "ai * bi", "ComptimeValue::Int(-n)", "_ => 8,"):
    if bad in macro_src:
        fail(f"macro comptime unchecked/fabricated semantic regression: {bad}")

# Exact duplicate active files are suspicious; archives and the
# conformance corpus are excluded because historical/golden copies are intentional.
import hashlib
seen_hashes: dict[str, Path] = {}
for candidate in ROOT.rglob("*"):
    if not candidate.is_file() or "archive" in candidate.parts or "conformance" in candidate.parts:
        continue
    relative_parts = candidate.relative_to(ROOT).parts
    if relative_parts and relative_parts[0] in {"target", "vendor", ".git"}:
        continue
    if candidate.name in {"SHA256SUMS", "Cargo.lock"}:
        continue
    if candidate.suffix.lower() not in {".rs", ".md", ".omni", ".toml", ".json", ".yaml", ".yml", ".py", ".sh", ".ps1"}:
        continue
    data = candidate.read_bytes()
    if len(data) < 20:
        continue
    digest = hashlib.sha256(data).hexdigest()
    prior = seen_hashes.get(digest)
    if prior is not None:
        fail(f"exact duplicate active files: {prior.relative_to(ROOT)} and {candidate.relative_to(ROOT)}")
    else:
        seen_hashes[digest] = candidate

# The remediation register itself must be closed and internally consistent.
issue_registry = json_file(ROOT / "release/LINEAGE_REMEDIATION_ISSUES.json")
issues = issue_registry.get("issues", [])
summary = issue_registry.get("summary", {})
if not issues:
    fail("lineage remediation issue register is empty")
open_issues = [i for i in issues if i.get("status") != "fixed"]
if open_issues:
    fail("lineage remediation register contains unresolved findings: " + ", ".join(str(i.get("id")) for i in open_issues))
if summary.get("total") != len(issues) or summary.get("fixed") != len(issues):
    fail("lineage remediation summary counts do not match the issue register")
if summary.get("open_baseline_blockers") != 0:
    fail("lineage remediation summary still reports baseline blockers")
notes.append(f"closed lineage findings: {len(issues)}")

notes.append(f"active production Rust files structurally scanned: {len(active_src)}")
notes.append("historical/future semantic surfaces are fail-closed or archived")

print("Omni lineage baseline audit (0.0.1.0 -> 0.1.4.1)")
for note in notes:
    print(f"  OK  {note}")
if errors:
    for error in errors:
        print(f"  ERR {error}")
    print(f"FAILED: {len(errors)} issue(s)")
    sys.exit(1)
print("PASS")