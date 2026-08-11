# Audit documents

The pre-remediation audits were moved to `docs/archive/historical-audits/` because they
describe source states that no longer match the current tree.

The current audit is:

- `../LINEAGE_REMEDIATION_AUDIT_0.0.1_TO_0.1.2.md`

Run `python3 scripts/audit-baseline.py` for the machine-checkable baseline audit and
`scripts/qualify-release.sh` (or `.ps1`) for full Rust-host qualification.
