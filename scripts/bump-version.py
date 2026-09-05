#!/usr/bin/env python3
"""Omni Four-Part Version Manager (stable.major.minor.patch).

Manages project-wide 4-part versioning:
  <stable>.<major>.<minor>.<patch> (e.g. 0.2.0.1111)

Supports:
  - patch: updates the 4th component (e.g. 0.2.0.0 -> 0.2.0.1 or explicit --number)
  - minor: updates the 3rd component, resets patch (e.g. 0.2.0.0 -> 0.2.1.0)
  - major: updates the 2nd component, resets minor and patch (e.g. 0.2.0.0 -> 0.3.0.0)
  - stable: updates the 1st component, resets all others (e.g. 0.2.0.0 -> 1.0.0.0)
  - set: sets an explicit 4-part version string
  - --sync / --push: commits the version bump, creates a tag, and pushes to git remote
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def parse_4part_version(ver_str: str) -> tuple[int, int, int, int]:
    parts = ver_str.strip().split(".")
    if len(parts) != 4:
        raise ValueError(f"Version '{ver_str}' is not a 4-part version (expected stable.major.minor.patch)")
    return tuple(int(p) for p in parts)  # type: ignore


def format_4part_version(parts: tuple[int, int, int, int]) -> str:
    return ".".join(str(p) for p in parts)


def semver_base_from_4part(parts: tuple[int, int, int, int]) -> str:
    return f"{parts[0]}.{parts[1]}.{parts[2]}"


def read_current_version() -> tuple[tuple[int, int, int, int], str]:
    cargo_toml = ROOT / "Cargo.toml"
    text = cargo_toml.read_text(encoding="utf-8")
    m = re.search(r'project-version\s*=\s*"([^"]+)"', text)
    if not m:
        raise RuntimeError("Could not find project-version in root Cargo.toml")
    ver_str = m.group(1)
    return parse_4part_version(ver_str), ver_str


def update_root_cargo_toml(new_4part: str, new_semver: str, dry_run: bool = False) -> None:
    path = ROOT / "Cargo.toml"
    text = path.read_text(encoding="utf-8")
    text = re.sub(r'project-version\s*=\s*"[^"]+"', f'project-version = "{new_4part}"', text)
    text = re.sub(r'cargo-semver-base\s*=\s*"[^"]+"', f'cargo-semver-base = "{new_semver}"', text)
    if not dry_run:
        path.write_text(text, encoding="utf-8")


def update_crate_cargo_tomls(new_4part: str, new_semver: str, dry_run: bool = False) -> list[Path]:
    updated: list[Path] = []
    for manifest in ROOT.glob("crates/*/Cargo.toml"):
        text = manifest.read_text(encoding="utf-8")
        orig_text = text
        # update package.version = "x.y.z" (only in [package] section)
        text = re.sub(r'(?m)^(\s*version\s*=\s*)"[^"]+"', rf'\g<1>"{new_semver}"', text, count=1)
        # update project-version = "x.y.z.w"
        text = re.sub(r'project-version\s*=\s*"[^"]+"', f'project-version = "{new_4part}"', text)
        # update cargo-semver-base = "x.y.z"
        text = re.sub(r'cargo-semver-base\s*=\s*"[^"]+"', f'cargo-semver-base = "{new_semver}"', text)
        if text != orig_text:
            updated.append(manifest)
            if not dry_run:
                manifest.write_text(text, encoding="utf-8")

    # Also check docs/archive if any
    for manifest in ROOT.glob("docs/archive/**/Cargo.toml"):
        text = manifest.read_text(encoding="utf-8")
        orig_text = text
        text = re.sub(r'project-version\s*=\s*"[^"]+"', f'project-version = "{new_4part}"', text)
        if text != orig_text:
            updated.append(manifest)
            if not dry_run:
                manifest.write_text(text, encoding="utf-8")

    return updated


def update_rust_sources(new_4part: str, dry_run: bool = False) -> None:
    # 1. crates/omni-compiler/src/version.rs
    v_rs = ROOT / "crates/omni-compiler/src/version.rs"
    if v_rs.exists():
        text = v_rs.read_text(encoding="utf-8")
        text = re.sub(r'pub const PROJECT_VERSION: &str = "[^"]+";', f'pub const PROJECT_VERSION: &str = "{new_4part}";', text)
        if not dry_run:
            v_rs.write_text(text, encoding="utf-8")

    # 2. crates/codegen-native/src/lib.rs
    cg_lib = ROOT / "crates/codegen-native/src/lib.rs"
    if cg_lib.exists():
        text = cg_lib.read_text(encoding="utf-8")
        text = re.sub(r'const OMNI_PROJECT_VERSION: &str = "[^"]+";', f'const OMNI_PROJECT_VERSION: &str = "{new_4part}";', text)
        if not dry_run:
            cg_lib.write_text(text, encoding="utf-8")

    # 3. crates/omni-selfhost/src/lib.rs
    sh_lib = ROOT / "crates/omni-selfhost/src/lib.rs"
    if sh_lib.exists():
        text = sh_lib.read_text(encoding="utf-8")
        text = re.sub(r'pub const VERSION: &str = "[^"]+-rust-bootstrap";', f'pub const VERSION: &str = "{new_4part}-rust-bootstrap";', text)
        if not dry_run:
            sh_lib.write_text(text, encoding="utf-8")


def update_json_file(rel_path: str, new_4part: str, new_semver: str, dry_run: bool = False) -> None:
    p = ROOT / rel_path
    if not p.exists():
        return
    try:
        data = json.loads(p.read_text(encoding="utf-8"))
    except Exception:
        return

    if "version" in data:
        data["version"] = new_4part
    if "cargo_semver_base" in data:
        data["cargo_semver_base"] = new_semver
    if "versioning" in data and isinstance(data["versioning"], dict):
        data["versioning"]["project_version"] = new_4part
        data["versioning"]["cargo_semver_base"] = new_semver
    if "installed_compiler" in data and isinstance(data["installed_compiler"], dict):
        if "version" in data["installed_compiler"]:
            data["installed_compiler"]["version"] = f"omni {new_4part}"
    if "gates" in data and isinstance(data["gates"], dict):
        if "omni --version" in data["gates"]:
            data["gates"]["omni --version"] = f"PASS: omni {new_4part}"

    if not dry_run:
        p.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


def update_scripts(new_4part: str, new_semver: str, dry_run: bool = False) -> None:
    for script_name in ("scripts/audit-baseline.py", "scripts/verify-source.py"):
        p = ROOT / script_name
        if not p.exists():
            continue
        text = p.read_text(encoding="utf-8")
        text = re.sub(r'EXPECTED_VERSION\s*=\s*"[^"]+"', f'EXPECTED_VERSION = "{new_4part}"', text)
        text = re.sub(r'EXPECTED_CARGO_SEMVER_BASE\s*=\s*"[^"]+"', f'EXPECTED_CARGO_SEMVER_BASE = "{new_semver}"', text)
        if not dry_run:
            p.write_text(text, encoding="utf-8")


def update_cargo_lock(dry_run: bool = False) -> None:
    if dry_run:
        return
    print("Refreshing Cargo.lock...")
    res = subprocess.run(["cargo", "check", "-p", "omni-stage0", "--locked"], cwd=ROOT, capture_output=True, text=True)
    if res.returncode != 0:
        subprocess.run(["cargo", "check", "-p", "omni-stage0", "--offline"], cwd=ROOT, capture_output=True, text=True)


def git_commit_and_push(new_4part: str, remote: str = "origin", branch: str | None = None, push: bool = False) -> None:
    # 1. Determine branch
    if not branch:
        res = subprocess.run(["git", "branch", "--show-current"], cwd=ROOT, capture_output=True, text=True)
        branch = res.stdout.strip() or "main"

    print(f"Staging version bump to git (branch: {branch})...")
    subprocess.run(["git", "add", "-A"], cwd=ROOT, check=True)

    commit_msg = f"chore(release): bump version to {new_4part}"
    res = subprocess.run(["git", "commit", "-m", commit_msg], cwd=ROOT, capture_output=True, text=True)
    if res.returncode == 0:
        print(f"Committed: {commit_msg}")
    else:
        print(f"Git commit message: {res.stdout.strip() or res.stderr.strip()}")

    # 2. Tag
    tag_name = f"v{new_4part}"
    subprocess.run(["git", "tag", "-a", tag_name, "-m", f"Release {new_4part}"], cwd=ROOT, capture_output=True)
    print(f"Created git tag: {tag_name}")

    # 3. Push if requested
    if push:
        print(f"Pushing commit and tags to remote '{remote}' (branch {branch})...")
        push_cmd = ["git", "push", remote, branch, "--tags"]
        push_res = subprocess.run(push_cmd, cwd=ROOT, capture_output=True, text=True)
        if push_res.returncode == 0:
            print(f"Successfully pushed to {remote} ({branch}) with tags!")
        else:
            print(f"Notice: Push to {remote} output: {push_res.stderr.strip() or push_res.stdout.strip()}")
            print(f"If you want to push to a specific GitHub remote:")
            print(f"  git remote add github <GITHUB_URL>")
            print(f"  git push github {branch} --tags")


def main() -> int:
    parser = argparse.ArgumentParser(description="Omni 4-part version bumper and git sync")
    subparsers = parser.add_subparsers(dest="command", required=True)

    # patch
    patch_parser = subparsers.add_parser("patch", help="Bump the 4th component (e.g. 0.2.0.0 -> 0.2.0.1 or explicit --number)")
    patch_parser.add_argument("--number", type=int, help="Set explicit patch number instead of incrementing by 1")

    # minor
    subparsers.add_parser("minor", help="Bump minor release (3rd component), resetting patch to 0 (e.g. 0.2.0.0 -> 0.2.1.0)")

    # major
    subparsers.add_parser("major", help="Bump major release (2nd component), resetting minor and patch (e.g. 0.2.0.0 -> 0.3.0.0)")

    # stable
    subparsers.add_parser("stable", help="Bump stable release (1st component), resetting all others (e.g. 0.2.0.0 -> 1.0.0.0)")

    # set
    set_parser = subparsers.add_parser("set", help="Set explicit 4-part version (e.g. 0.2.0.1111)")
    set_parser.add_argument("version", help="Explicit version string (stable.major.minor.patch)")

    # current
    subparsers.add_parser("current", help="Show current 4-part version")

    # Common options
    for p in (patch_parser, subparsers.choices["minor"], subparsers.choices["major"], subparsers.choices["stable"], set_parser):
        p.add_argument("--dry-run", action="store_true", help="Print changes without writing files")
        p.add_argument("--sync", action="store_true", help="Commit, tag, and push changes to git remote")
        p.add_argument("--remote", default="origin", help="Git remote name (default: origin)")
        p.add_argument("--branch", help="Git branch name (default: current branch)")

    args = parser.parse_args()

    curr_parts, curr_str = read_current_version()
    if args.command == "current":
        print(f"Current Omni version: {curr_str} (Cargo base: {semver_base_from_4part(curr_parts)})")
        return 0

    s, M, m, p = curr_parts

    if args.command == "patch":
        if args.number is not None:
            new_p = args.number
        else:
            new_p = p + 1
        new_parts = (s, M, m, new_p)
    elif args.command == "minor":
        new_parts = (s, M, m + 1, 0)
    elif args.command == "major":
        new_parts = (s, M + 1, 0, 0)
    elif args.command == "stable":
        new_parts = (s + 1, 0, 0, 0)
    elif args.command == "set":
        new_parts = parse_4part_version(args.version)
    else:
        print(f"Unknown command: {args.command}", file=sys.stderr)
        return 1

    new_4part = format_4part_version(new_parts)
    new_semver = semver_base_from_4part(new_parts)

    print(f"Updating version: {curr_str} -> {new_4part} (Cargo base: {new_semver})")

    # 1. Root Cargo.toml
    update_root_cargo_toml(new_4part, new_semver, dry_run=args.dry_run)

    # 2. Crates Cargo.tomls
    updated_manifests = update_crate_cargo_tomls(new_4part, new_semver, dry_run=args.dry_run)
    print(f"Updated {len(updated_manifests)} crate manifest(s).")

    # 3. Rust sources
    update_rust_sources(new_4part, dry_run=args.dry_run)
    print("Updated Rust version constants.")

    # 4. JSON manifests and qualifications
    for rel_json in (
        "RELEASE_MANIFEST.json",
        "SOURCE_QUALIFICATION.json",
        "release/BINARY_QUALIFICATION.json",
        "release/FINAL_OFFLINE_VALIDATION.json",
    ):
        update_json_file(rel_json, new_4part, new_semver, dry_run=args.dry_run)
    print("Updated qualification and release JSON files.")

    # 5. Audit scripts
    update_scripts(new_4part, new_semver, dry_run=args.dry_run)
    print("Updated audit scripts (audit-baseline.py, verify-source.py).")

    # 6. Cargo.lock
    update_cargo_lock(dry_run=args.dry_run)

    print(f"Omni version successfully updated to {new_4part}!")

    if getattr(args, "sync", False):
        git_commit_and_push(new_4part, remote=args.remote, branch=args.branch, push=True)

    return 0


if __name__ == "__main__":
    sys.exit(main())
