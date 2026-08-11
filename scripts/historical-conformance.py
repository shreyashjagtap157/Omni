#!/usr/bin/env python3
"""Run the historical v0.0.1 compatibility corpus against the current compiler."""
from __future__ import annotations
import argparse
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_MANIFEST = ROOT / "conformance/historical_v0_0_1/manifest.json"


def invoke(cmd: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=ROOT, text=True, capture_output=True, check=False)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--omni", default="omni", help="path/name of the Omni CLI")
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--allow-non-native-host", action="store_true")
    args = parser.parse_args()

    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    omni = shutil.which(args.omni) if os.sep not in args.omni else args.omni
    if not omni or not Path(omni).exists():
        print(f"ERROR: Omni executable not found: {args.omni}", file=sys.stderr)
        return 2

    host_native = platform.system().lower() == "linux" and platform.machine().lower() in {"x86_64", "amd64"}
    failures = 0
    executed = 0
    skipped = 0
    print(f"Omni conformance suite: {manifest['suite']} ({manifest['version']})")
    print(f"compiler: {omni}")
    print(f"host: {platform.machine()}-{platform.system().lower()}")

    for case in manifest["cases"]:
        path = args.manifest.parent / case["file"]
        mode = case["mode"]
        if mode == "run" and not host_native and not args.allow_non_native_host:
            print(f"SKIP {case['name']}: owned native backend requires x86-64 Linux/WSL")
            skipped += 1
            continue
        if mode == "run":
            if path.is_dir():
                with tempfile.TemporaryDirectory(prefix="omni-conformance-run-") as td:
                    copied = Path(td) / "project"
                    shutil.copytree(path, copied)
                    proc = invoke([str(omni), "run", str(copied)])
            else:
                proc = invoke([str(omni), "run", str(path)])
            ok = proc.returncode == case["exit"] and proc.stdout == case["stdout"]
            if not ok:
                print(f"FAIL {case['name']}: exit={proc.returncode} stdout={proc.stdout!r} stderr={proc.stderr!r}")
                failures += 1
            else:
                print(f"PASS {case['name']}")
            executed += 1
        elif mode == "build_run":
            if not host_native and not args.allow_non_native_host:
                print(f"SKIP {case['name']}: owned native backend requires x86-64 Linux/WSL")
                skipped += 1
                continue
            with tempfile.TemporaryDirectory(prefix="omni-conformance-") as td:
                temp_root = Path(td)
                project = temp_root / "project"
                if path.is_dir():
                    shutil.copytree(path, project)
                    build_input = project
                else:
                    build_input = path
                artifact = temp_root / "app"
                build = invoke([str(omni), "build", str(build_input), "-o", str(artifact)])
                if build.returncode != 0 or not artifact.is_file():
                    print(f"FAIL {case['name']}: build exit={build.returncode} stdout={build.stdout!r} stderr={build.stderr!r}")
                    failures += 1
                else:
                    proc = invoke([str(artifact)])
                    ok = proc.returncode == case["exit"] and proc.stdout == case["stdout"]
                    if not ok:
                        print(f"FAIL {case['name']}: exit={proc.returncode} stdout={proc.stdout!r} stderr={proc.stderr!r}")
                        failures += 1
                    else:
                        print(f"PASS {case['name']}")
                executed += 1
        elif mode == "check_fail":
            proc = invoke([str(omni), "check", str(path)])
            needle = case["stderr_contains"]
            ok = proc.returncode != 0 and needle in proc.stderr
            if not ok:
                print(f"FAIL {case['name']}: exit={proc.returncode} expected stderr to contain {needle!r}; stderr={proc.stderr!r}")
                failures += 1
            else:
                print(f"PASS {case['name']}")
            executed += 1
        else:
            print(f"FAIL {case['name']}: unknown mode {mode!r}")
            failures += 1
            executed += 1

    print(f"result: {executed - failures} passed; {failures} failed; {skipped} skipped")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
