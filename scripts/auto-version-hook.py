#!/usr/bin/env python3
"""Omni Git Pre-Commit Hook logic.

Automatically bumps the patch version (x.y.z.w -> x.y.z.(w+1)) whenever
staged files contain code, tests, or docs changes, without requiring
manual version updates.
"""

from __future__ import annotations

from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    # Check what files are staged for commit
    res = subprocess.run(["git", "diff", "--cached", "--name-only"], cwd=ROOT, capture_output=True, text=True)
    if res.returncode != 0:
        return 0

    staged_files = [f.strip() for f in res.stdout.splitlines() if f.strip()]
    if not staged_files:
        return 0

    # If the only staged files are already version bump files or hook files, do not re-bump
    version_files = {
        "Cargo.toml",
        "Cargo.lock",
        "RELEASE_MANIFEST.json",
        "SOURCE_QUALIFICATION.json",
        "release/BINARY_QUALIFICATION.json",
        "release/FINAL_OFFLINE_VALIDATION.json",
        "scripts/audit-baseline.py",
        "scripts/verify-source.py",
        "scripts/bump-version.py",
        "scripts/auto-version-hook.py",
        "scripts/sync-github.ps1",
        "scripts/sync-github.sh",
        ".githooks/pre-commit",
        "crates/omni-compiler/src/version.rs",
        "crates/codegen-native/src/lib.rs",
        "crates/omni-selfhost/src/lib.rs",
    }
    non_version_staged = [f for f in staged_files if f not in version_files and not f.endswith("Cargo.toml")]

    if not non_version_staged:
        # Commit is already purely a version bump, hook, or metadata update
        return 0

    # Auto-bump patch version
    print("\n[auto-version] Implementation/code change detected in staged files.")
    print("[auto-version] Automatically incrementing Omni patch version...")
    bump_res = subprocess.run([sys.executable, str(ROOT / "scripts/bump-version.py"), "patch"], cwd=ROOT)
    if bump_res.returncode != 0:
        print("[auto-version] Error: Automatic version bump failed!", file=sys.stderr)
        return bump_res.returncode

    # Stage the newly updated version files into the current commit
    for vf in version_files:
        p = ROOT / vf
        if p.exists():
            subprocess.run(["git", "add", vf], cwd=ROOT)

    for manifest in ROOT.glob("crates/*/Cargo.toml"):
        subprocess.run(["git", "add", str(manifest.relative_to(ROOT))], cwd=ROOT)

    print("[auto-version] Version files successfully incremented and staged into commit.\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
