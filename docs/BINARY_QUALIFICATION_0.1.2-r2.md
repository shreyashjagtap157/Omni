# Omni v0.1.2-r2 Binary Qualification

Status: **PASS** for the cumulative implementation surface claimed by versions 0.0.1 through 0.1.2 on x86-64 Linux.

## Bootstrap environment

- Rust `1.97.1`
- Cargo `1.97.1`
- rustfmt `1.9.0-stable`
- Clippy `0.1.97`
- Canonical emitted artifact: owned x86-64 Linux ELF64 native AOT executable

## Required Rust gates

All requested gates pass on the final source:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --workspace --locked
cargo build --release --locked -p omni-stage0
```

Additional whole-workspace gates also pass:

```bash
cargo clippy --workspace --locked --all-targets -- -D warnings
cargo test --workspace --locked
```

The workspace registers 517 tests. Seven tests are intentionally ignored because they are explicit post-v0.1.2 aggregate/trait semantic qualification cases; they are retained so later milestones cannot forget them.

## Runtime/conformance evidence

- Historical v0.0.1 compatibility: **5/5 PASS**.
- Native scalar v0.1.2 conformance: **23/23 PASS**.
- Lexer/parser fuzzing against the final parser: **22,080 generated cases across 60 cumulative seconds**, four independent deterministic seeds, zero crashes/signals/timeouts.
- Installed compiler SHA-256: `d54e36d31ebc9d9e0d80a545862b0472f3ae9e1dc25309f2509162021118080f`.

## Scope boundary

This qualification does **not** claim later-roadmap features are complete. Aggregate layout, production ownership soundness, full trait semantics/monomorphization, qualified FFI/ABI expansion, concurrency/async, non-x86-64 native emission, LLVM/JIT qualification, and other later milestones remain outside v0.1.2 and must fail closed where applicable.

Machine-readable evidence is in `release/BINARY_QUALIFICATION.json`; detailed command logs are under `release/qualification-logs/`.
