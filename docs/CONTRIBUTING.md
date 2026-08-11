# Contributing to Omni

Omni is native-first. The current release-qualified implementation is deliberately
narrower than the parser/AST surface, so contributions must preserve the distinction
between **recognized**, **checked**, and **native executable** semantics.

## Before changing code

Read:

1. `../spec/README.md`;
2. `CURRENT_IMPLEMENTATION_MATRIX.md`;
3. `MILESTONE_0.1.3_NATIVE_DATA_LAYOUT_I.md`;
4. `VERSIONING_AND_BOOTSTRAP_PLAN.md`;
5. `LINEAGE_REMEDIATION_AUDIT_0.0.1_TO_0.1.2.md`;
6. relevant ADRs/spec modules.

Historical plans under `archive/` are not current requirements.

## Baseline checks

Run:

```bash
python3 scripts/audit-baseline.py --worktree
python3 scripts/verify-source.py --worktree
cargo fmt --all -- --check
cargo clippy --workspace --locked --all-targets -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
```

For a release candidate, run `scripts/qualify-release.sh` (or the PowerShell variant).
The whole-workspace checks are integrity gates for retained research crates; they do not
qualify those crates as semantic backends. The semantic claim is still limited to the
explicit native conformance corpora and `CURRENT_IMPLEMENTATION_MATRIX.md`.

## Feature completion rule

A language feature is complete only when its semantics exist end-to-end:

1. grammar/source mapping;
2. resolution and static checks;
3. MIR representation and verifier invariants;
4. lowering/code generation where required;
5. canonical native execution;
6. positive, negative, and regression conformance cases;
7. documentation/status matrix update.

Do not mark a feature complete because its AST node exists.

## Fail-closed rule

Unsupported behavior must return a stable diagnostic/error. Do not synthesize zero,
empty strings, fake success, `Nop`, slot-zero variables, an unrelated backend, or a
mock proof result.

## Pull-request checklist

- [ ] Change is scoped to the current milestone or explicitly experimental.
- [ ] New semantics are represented consistently in source checks, MIR and native path.
- [ ] Unsupported paths fail closed.
- [ ] Positive and negative tests/conformance cases cover the change.
- [ ] `CURRENT_IMPLEMENTATION_MATRIX.md` is updated if capability changed.
- [ ] `CHANGELOG.md` is updated for user-visible behavior.
- [ ] Source audit/verifier pass.
- [ ] Rust format, warning-denied workspace Clippy/tests/build pass.
- [ ] No generated `target/`, vendored offline registry, LLVM SDK/cache or fuzz corpus is committed.

See `CODE_OF_CONDUCT.md` and `SECURITY.md` for project conduct and vulnerability handling.
