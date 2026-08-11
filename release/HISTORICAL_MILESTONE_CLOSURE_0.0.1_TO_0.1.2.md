# Omni historical milestone closure — 0.0.1 through 0.1.2

Status: **source-closed and binary-qualified on x86-64 Linux with Rust 1.97.1**  
Baseline: Omni v0.1.2-r2  
Canonical specification target: `spec/edition1/`.

This document reconciles the original early roadmap with the later native-hardening work. The original version meanings remain cumulative; native AOT/control-flow hardening added later does not redefine them.

## 0.0.1 — workspace foundation

Closed source requirements:
- Cargo workspace/resolver-v2 structure exists.
- CI workflow, devcontainer, contributor documentation and ADR-0001/ADR-0002 exist.
- Every workspace member has version 0.1.2 in the remediated cumulative tree.

Host gate still required: `cargo build --workspace --locked` for historical whole-workspace compatibility. Research crates are not part of the semantic native-release closure, so failure in an explicitly archived/unqualified backend must not be mistaken for native semantic failure; such crates must nevertheless compile if retained as live workspace members.

## 0.0.2 — documentation foundation

Closed source requirements:
- specification snapshot, decision records, security/conduct/contribution policy, implementation status, quick start and roadmap are present;
- stale plans are archived and are not authoritative;
- current docs point to the Edition-1 snapshot and current implementation matrix.

## 0.1.0 — lexer

Closed source requirements:
- complete lexer/token inventory, indentation compatibility tokens, comments, interpolation tokenization and debug/inspection support exist;
- lexer/parser fuzz target source is retained under `crates/omni-fuzz/fuzz_targets/`.

Host gate still required: run the lexer/parser fuzz target for at least 60 seconds with the pinned Rust toolchain/cargo-fuzz (or the release-approved equivalent harness) without a panic/crash.

## 0.1.1 — parser

Closed source requirements:
- recursive-descent/Pratt expression parser and recovery exist;
- generic function syntax and transitional historical syntax remain parseable where compatible;
- deterministic parallel independent-file parsing exists;
- scoped imports exist;
- `pub`, `pub(mod)`, `pub(pkg)`, `pub(cap: X)`, and `pub(friend: X)` are represented rather than discarded;
- effect annotations are represented;
- `@requires`, `@ensures`, and `@invariant` contracts are represented;
- error-set declarations are represented and now preserve visibility.

Regression source: `crates/omni-compiler/tests/historical_milestone_closure.rs` and `parser_parallel.rs`.

## 0.1.2 — CST + formatter, plus native scalar hardening

Closed source requirements:
- lossless CST path exists for tooling;
- AST formatter is deterministic/idempotent over its qualified surface;
- strict formatting sorts imports deterministically;
- effect annotations/contracts/visibility/error sets are preserved by AST formatting;
- CLI exposes `omni fmt <file> [--check] [--strict]`;
- the later native scalar hardening remains cumulative: owned x86-64 ELF AOT, scalar calls/arithmetic/control flow, `break`/`continue`, MIR/LIR verification and fail-closed unsupported semantics.

## Promotion gates

The source tree SHALL NOT be called fully build-qualified until all of the following pass on x86-64 Linux/WSL2 using the pinned Rust 1.97.1 toolchain:

```bash
python3 scripts/audit-baseline.py
python3 scripts/verify-source.py
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked -p omni-stage0
cargo install --path crates/omni-stage0 --locked --force
python3 scripts/historical-conformance.py --omni omni
python3 scripts/native-conformance.py --omni omni
```

Additionally, historical foundation compatibility SHALL be checked with `cargo build --workspace --locked` and the lexer/parser fuzz target SHALL run for >=60 seconds without crash. These two historical compatibility gates are separate from the default native semantic closure.
