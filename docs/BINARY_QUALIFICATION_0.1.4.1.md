# Omni v0.1.4.1 qualification report

Status: **PASS** for the four-part versioning migration patch.

## Versioning result

Omni project releases now use:

```text
stableRelease.majorRelease.minorRelease.patch
```

The current project release identity is **0.1.4.1**. Cargo crate versions remain at the SemVer-compatible base **0.1.4**, with every workspace package carrying `package.metadata.omni.project-version = "0.1.4.1"`.

## Build and source gates

- `python3 scripts/verify-source.py --worktree` — PASS
- `python3 scripts/audit-baseline.py --worktree` — PASS
- `cargo fmt --all -- --check` — PASS
- `cargo clippy --workspace --locked --all-targets -- -D warnings` — PASS
- `cargo test --workspace --locked` — PASS

The workspace remains at 14 members with Cargo SemVer base `0.1.4`. The same five v0.3.0.0 trait/type-system semantic tests are intentionally ignored; no claimed v0.1.4.1 surface test is ignored.

## Installed compiler evidence

- `omni --version` — `omni 0.1.4.1`
- `omni doctor` — PASS
- Installed compiler SHA-256 — `06063fcc3036c3916d1294022addf19d08ad81f3f62bdea3827105f6f8d12e9d`

## Executable conformance

- Historical v0.0.1 corpus: **5/5 PASS**
- Native scalar v0.1.2 corpus: **23/23 PASS**
- Native layout v0.1.3 corpus: **10/10 PASS**
- Native value ABI v0.1.4 corpus: **10/10 PASS**

## Fuzz evidence

Lexer/parser smoke fuzzing completed **60 cumulative seconds** over four deterministic 15-second shards, seeds 660100–660103, with **22,055 generated cases** and zero failures.

## Scope boundary

This is a patch release. It does not expand the v0.1.4.0 semantic surface. Ownership, borrowing, moves, initialization/drop state, and regions remain the next major milestone: **0.2.0.0**.
