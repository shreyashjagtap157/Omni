# Contributing to Omni

Omni is native-first. A feature is not implemented merely because it parses: its claimed
milestone must have static semantics, verified IR/lowering where applicable, positive and
negative tests, and canonical native evidence when the milestone promises execution.

## Read first

1. `spec/README.md` and `spec/edition1/` — normative language target.
2. `docs/CURRENT_IMPLEMENTATION_MATRIX.md` — qualified implementation surface.
3. `docs/VERSIONING_AND_BOOTSTRAP_PLAN.md` — forward milestone sequence.
4. `release/HISTORICAL_MILESTONE_CLOSURE_0.0.1_TO_0.1.2.md` — reconciled early-version contract.
5. `docs/LINEAGE_REMEDIATION_AUDIT_0.0.1_TO_0.1.2.md` — remediation evidence.

Historical plans under `docs/archive/` are evidence only. When they conflict with the
current Edition-1 standard, the current standard wins and the obsolete requirement must
be recorded as superseded rather than implemented as a conflicting extension.

## Required baseline checks

```bash
python3 scripts/audit-baseline.py
python3 scripts/verify-source.py
cargo fmt --all -- --check
cargo clippy --workspace --locked --all-targets -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
```

On x86-64 Linux/WSL2, release candidates also run:

```bash
python3 scripts/historical-conformance.py --omni omni
python3 scripts/native-conformance.py --omni omni
python3 scripts/native-conformance.py --omni omni --manifest conformance/native_layout_v0_1_3/manifest.json
```

The full promotion gate is `scripts/qualify-release.sh` or `scripts/qualify-release.ps1`.

## Fail-closed rule

Unsupported behavior must produce an explicit diagnostic/error. Never substitute zero,
empty data, `Nop`, a different backend, mock proof success, or a guessed implementation.

## Repository hygiene

Do not commit Cargo `target/`, LLVM SDK/build trees, fuzz artifacts/corpora, Python caches,
or generated local executables. Fuzz *source* stays versioned.
