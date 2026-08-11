# Archived / Unqualified Tooling

These scripts are preserved for historical or future-backend work, but they are
**not part of the qualified Omni v0.1.2-r2 toolchain**.

- LLVM setup/download scripts belong to the later LLVM oracle/backend track.
- `trigger-pr.ps1` targeted an obsolete LLVM CI workflow.
- `test_fmt.rs` used an older standalone formatter harness rather than the
  qualified Cargo test/CLI path.
- `compare_reproducible_build.ps1` assumes historical Windows/PE parity output;
  reproducible-release qualification will be reintroduced against the owned
  native artifact model in its dedicated milestone.

Keeping them under `docs/archive` preserves useful history without presenting
unqualified utilities as current release commands.
