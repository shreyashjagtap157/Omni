# Omni v0.1.1 — Native Core Baseline

> **Historical note (2026-08-08):** This document records the v0.1.1 achievement boundary. The current v0.1.4 promotion gate uses whole-workspace Rust integrity checks while keeping semantic qualification restricted to explicit native conformance corpora. Use `scripts/qualify-release.sh` and `docs/CURRENT_IMPLEMENTATION_MATRIX.md` for current qualification.

## Achievement

v0.1.1 is the first milestone in this repository whose canonical CLI path is explicitly
owned AOT native emission rather than an in-process JIT being labelled “native.”

The purpose of this milestone is **trustworthy narrowing**: retain the broad frontend,
but execute only the subset whose MIR/LIR/native semantics are implemented, and fail
closed elsewhere.

## Deliverables

- repaired source-module integrity;
- workspace version `0.1.1` and synchronized lockfile;
- `codegen-native` owned backend crate;
- direct x86-64 Linux ELF64 output;
- native scalar locals, arithmetic, comparison, branching, calls and return;
- Linux syscall-based integer/string output and process exit;
- checked integer fault path;
- function parameter ABI flow through MIR/LIR;
- native `omni run`, `omni run-native`, and `omni build`;
- explicit `omni run-jit` development path;
- fixed build `-o/--output` parsing;
- optimizer correctness hardening;
- fail-closed markers for incomplete advanced semantics;
- source-release verifier;
- Unix and PowerShell installer scripts;
- Edition-1 brace/semicolon parser regression test;
- native backend regression tests, LIR evaluation-stack/CFG validation and SysV call alignment;
- minimal default dependency closure: Cranelift/Wasm/LLVM backends are opt-in;
- `omni doctor` capability report;
- updated project status/native policy/versioning documentation.

## Supported host for owned AOT

`x86_64-linux`, including a normal Linux environment inside WSL2.

Other hosts may build frontend/tooling code, but v0.1.1 intentionally refuses to claim
owned native artifact support for them yet.

## Qualification

Offline structural verification in the packaging environment:

```text
python3 scripts/verify-source.py
PASS
```

The packaging environment has no Rust toolchain, so the following remain mandatory on
a Rust-equipped qualification host before promoting the tag:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --workspace --locked
cargo install --path crates/omni-stage0 --locked --force
omni run examples/native_edition1.omni
```

Expected `native_edition1.omni` behavior: print `42` and exit with process status 42.
