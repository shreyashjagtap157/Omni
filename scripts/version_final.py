#!/usr/bin/env python3
"""Final comprehensive version audit from 0.0.1.0 to 0.2.0.0."""
import json
import os
import re
import subprocess

ROOT = r"C:\Users\siddh\Downloads\ABC\Omni"

def sp(s):
    """Safe print that handles Unicode."""
    try:
        # Encode to ASCII with replacement, print
        ascii_s = s.encode('ascii', errors='replace').decode('ascii')
        print(ascii_s)
    except:
        pass

# 1. BINARY_QUALIFICATION.json
print("1. BINARY_QUALIFICATION.json")
with open(os.path.join(ROOT, "release/BINARY_QUALIFICATION.json"), "r", encoding="utf-8", errors="replace") as f:
    bq = json.load(f)
sp(f"  version: {bq.get('version')}")
sp(f"  classification: {bq.get('classification')}")
sp(f"  qualification_date: {bq.get('qualification_date')}")
gates = bq.get("gates", {})
for k, v in gates.items():
    sp(f"  gate: {k} = {v}")

# 2. FINAL_OFFLINE_VALIDATION.json
print("\n2. FINAL_OFFLINE_VALIDATION.json")
with open(os.path.join(ROOT, "release/FINAL_OFFLINE_VALIDATION.json"), "r", encoding="utf-8", errors="replace") as f:
    fov = json.load(f)
sp(f"  version: {fov.get('version')}")
sp(f"  status: {fov.get('status')}")
sp(f"  reason: {fov.get('reason')[:100]}")

# 3. LINEAGE_REMEDIATION_ISSUES.json
print("\n3. LINEAGE_REMEDIATION_ISSUES.json")
with open(os.path.join(ROOT, "release/LINEAGE_REMEDIATION_ISSUES.json"), "r", encoding="utf-8", errors="replace") as f:
    lr = json.load(f)
summary = lr.get("summary", {})
sp(f"  total_issues: {summary.get('total')}")
sp(f"  fixed: {summary.get('fixed')}")
sp(f"  open_baseline_blockers: {summary.get('open_baseline_blockers')}")

# 4. RELEASE_MANIFEST.json (may not exist as standalone; version tracked in BINARY_QUALIFICATION)
print("\n4. RELEASE_MANIFEST.json check")
manifest_path = os.path.join(ROOT, "release/RELEASE_MANIFEST.json")
if os.path.exists(manifest_path):
    with open(manifest_path, "r", encoding="utf-8", errors="replace") as f:
        rm = json.load(f)
    sp(f"  version: {rm.get('version')}")
    sp(f"  classification: {rm.get('classification')}")
else:
    sp("  RELEASE_MANIFEST.json not found at release/ - version tracked in BINARY_QUALIFICATION.json instead")

# 5. CURRENT_IMPLEMENTATION_MATRIX.md - qualified core
print("\n5. CURRENT_IMPLEMENTATION_MATRIX.md (qualified core)")
with open("docs/CURRENT_IMPLEMENTATION_MATRIX.md", "r", encoding="utf-8", errors="replace") as f:
    cim = f.read()
# Find the qualified cumulative native core section
idx = cim.find("Qualified cumulative native core")
if idx >= 0:
    lines = cim.split("\n")
    count = 0
    for i in range(idx, min(idx + 60, len(lines))):
        sp(f"  [{i}] {lines[i][:80]}")
        count += 1

# 6. VERSIONING_AND_BOOTSTRAP_PLAN.md
print("\n6. VERSIONING_AND_BOOTSTRAP_PLAN.md (milestone sequence)")
with open("docs/VERSIONING_AND_BOOTSTRAP_PLAN.md", "r", encoding="utf-8", errors="replace") as f:
    vap = f.read()
# Extract milestone lines
import re
# Match lines like "- **0.1.0** — broad experimental local compiler snapshot"
milestones = re.findall(r"- \*\*([\d.]+)\*\* — .+? \*\*", vap)
sp(f"  found {len(milestones)} milestone version patterns")

# 8. MILESTONE_0.1.3
print("\n7. MILESTONE_0.1.3_NATIVE_DATA_LAYOUT_I.md achievements")
with open("docs/MILESTONE_0.1.3_NATIVE_DATA_LAYOUT_I.md", "r", encoding="utf-8", errors="replace") as f:
    m013 = f.read()
achievements = [l.strip() for l in m013.split("\n") if any(kw in l.lower() for kw in ["achievement", "qualified", "fail-closed", "deliberate"])][:15]
for a in achievements:
    sp(f"  {a}")

# 8. MILESTONE_0.1.4
print("\n8. MILESTONE_0.1.4_STRING_BYTE_VALUE_ABI_COLLECTIONS_FOUNDATION.md achievements")
with open("docs/MILESTONE_0.1.4_STRING_BYTE_VALUE_ABI_COLLECTIONS_FOUNDATION.md", "r", encoding="utf-8", errors="replace") as f:
    m014 = f.read()
achievements4 = [l.strip() for l in m014.split("\n")[:20]]
for a in achievements4:
    sp(f"  {a}")

# 9. Cargo workspace members
print("\n8. CARGO WORKSPACE MEMBERS")
with open(os.path.join(ROOT, "Cargo.toml"), "rb") as f:
    cargo_data = tomllib.load(f)
members = cargo_data.get("workspace", {}).get("members", [])
for m in members:
    path = os.path.join(ROOT, m, "Cargo.toml")
    if os.path.exists(path):
        with open(path, "rb") as pf:
            pkg = tomllib.load(pf)
        pn = pkg.get("package", {}).get("name", "?")
        pv = pkg.get("package", {}).get("version", "?")
        sp(f"  {m}: {pn} v{pv}")

# 10. Audit results
print("\n9. AUDIT RESULTS")
result = subprocess.run(["python", "scripts/audit-baseline.py", "--worktree"], capture_output=True, text=True, cwd=ROOT)
sp(f"  audit-baseline.py: returncode={result.returncode}, output_len={len(result.stdout)}")
result2 = subprocess.run(["python", "scripts/verify-source.py", "--worktree"], capture_output=True, text=True, cwd=ROOT)
sp(f"  verify-source.py: returncode={result2.returncode}, output_len={len(result2.stdout)}")

sp("\n=== AUDIT COMPLETE ===")