#!/usr/bin/env python3
"""Re-audit every version from 0.0.1.0 to 0.2.0.0."""
import json
import os
import re
import subprocess

ROOT = r"C:\Users\siddh\Downloads\ABC\Omni"

def safe_print(text):
    try:
        print(text)
    except:
        pass

def read_file(path, encoding="utf-8"):
    try:
        with open(path, encoding=encoding, errors="replace") as f:
            return f.read()
    except Exception as e:
        return f"ERROR:{e}"

def safe_json(path):
    try:
        with open(path, encoding="utf-8", errors="replace") as f:
            return json.load(f)
    except Exception as e:
        return {"error": str(e)}

# 1. BINARY_QUALIFICATION.json
print_section = lambda title, content: print(f"\n{'='*60}\n{title}\n{'='*60}") or print(content) if isinstance(content, str) else print(content)

# 1. BINARY_QUALIFICATION.json
print_section("1. BINARY_QUALIFICATION.json", safe_json(os.path.join(ROOT, "release/BINARY_QUALIFICATION.json")))

# 2. FINAL_OFFLINE_VALIDATION.json
print_section("2. FINAL_OFFLINE_VALIDATION.json", safe_json(os.path.join(ROOT, "release/FINAL_OFFLINE_VALIDATION.json")))

# 3. LINEAGE_REMEDIATION_ISSUES.json
print_section("3. LINEAGE_REMEDIATION_ISSUES.json", safe_json(os.path.join(ROOT, "release/LINEAGE_REMEDIATION_ISSUES.json")))

# 4. RELEASE_MANIFEST.json
print_section("4. RELEASE_MANIFEST.json", safe_json(os.path.join(ROOT, "release/RELEASE_MANIFEST.json")))

# 5. CURRENT_IMPLEMENTATION_MATRIX.md
cim = read_file("docs/CURRENT_IMPLEMENTATION_MATRIX.md")
print_section("5. CURRENT_IMPLEMENTATION_MATRIX.md (qualified core)", 
    re.search(r'## v3.4 reconciliation matrix(.*?)##', cim, re.DOTALL).group(1)[:1500] if re.search(r'## v3.4 reconciliation matrix(.*?)##', cim, re.DOTALL) else "not found")

# 6. VERSIONING_AND_BOOTSTRAP_PLAN.md
vap = read_file("docs/VERSIONING_AND_BOOTSTRAP_PLAN.md")
print_section("6. VERSIONING_AND_BOOTSTRAP_PLAN.md (milestone sequence)",
    re.findall(r'\*\*- .+? \*\*-', vap)[:20])

# 7. MILESTONE_0.1.3
m013 = read_file("docs/MILESTONE_0.1.3_NATIVE_DATA_LAYOUT_I.md")
print_section("7. MILESTONE_0.1.3 ACHIEVEMENTS",
    [l for l in m013.split("\n") if any(kw in l.lower() for kw in ['achievement', 'qualified', 'fail-closed', 'deliberate'])][:15])

# 8. MILESTONE_0.1.4
m014 = read_file("docs/MILESTONE_0.1.4_STRING_BYTE_VALUE_ABI_COLLECTIONS_FOUNDATION.md")
print_section("8. MILESTONE_0.1.4 ACHIEVEMENTS",
    [l for l in m014.split("\n")[:20]])

# 9. Cargo workspace members
import tomllib
with open(os.path.join(ROOT, "Cargo.toml"), 'rb') as f:
    cargo_data = tomllib.load(f)
members = cargo_data.get("workspace", {}).get("members", [])
print_section("9. CARGO WORKSPACE MEMBERS",
    [f"{m}: {tomllib.load(os.path.join(ROOT, m, 'Cargo.toml'))['package']['name']} v{tomllib.load(os.path.join(ROOT, m, 'Cargo.toml'))['package']['version']}" for m in members])

# 9. Audit results
result = subprocess.run(["python", "scripts/audit-baseline.py", "--worktree"], 
                       capture_output=True, text=True, cwd=ROOT)
print_section("9. AUDIT-BASELINE.py RESULT", result.stdout[-300:] if result.stdout else "no output")

result2 = subprocess.run(["python", "scripts/verify-source.py", "--worktree"], 
                        capture_output=True, text=True, cwd=ROOT)
print_section("10. VERIFY-SOURCE.py RESULT", result2.stdout[-300:] if result2.stdout else "no output")