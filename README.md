# Omni

Omni is a native-first, self-hosting-targeted systems/general-purpose programming language.
The canonical execution path compiles Omni source ahead of time to owned target machine
code; released native programs do not require a VM, interpreter, mandatory JIT, or
foreign-language runtime.

## Current milestone: v0.2.0.0 — Ownership, Borrowing & Safe References

v0.2.0.0 preserves the qualified scalar/control-flow, local-layout, and value-ABI lineage through
v0.1.4.1, qualifying the full ownership, borrowing, and safe-reference execution subset on owned x86-64 Linux AOT:

- safe references (`&x`, `&mut x`), reborrowing, dereferencing (`*r`, `*r = expr`), and multi-block loan tracking;
- linear moves, partial field moves, and linear field reinitialization;
- nominal struct field mutation (`p.x = expr`) for `let mut` and linear bindings;
- MIR borrow checking with Polonius and CFG linear consumption verification;
- deterministic linear resource consumption and drop checks along CFG paths;
- fail-closed diagnostics for escaping borrows, storing references into aggregates, and unproven borrows.

The canonical pipeline remains:

```text
Omni source
  -> lexer / parser / static checks
  -> typed MIR + Polonius borrow checking & linear validation
  -> typed LIR + stack / CFG / local-layout / bounded-pointer verification
  -> owned x86-64 encoder
  -> ELF64 executable
  -> Linux loader -> CPU
```

Cranelift/LLVM execution remains fail closed; MLIR/Wasm remain noncanonical experiments.
They do not define Omni language semantics.

## Versioning & Remote Sync

Omni uses an automated four-part project version identity:
`stable.major.minor.patch` (e.g. `0.2.0.0`, `0.2.0.1111`, `1.0.0.0`).

- **Automatic Patch Increment**: Every commit containing code or implementation changes triggers
  the Git pre-commit hook (`.githooks/pre-commit` -> `scripts/auto-version-hook.py`), which automatically
  bumps the 4th component (`0.2.0.0` -> `0.2.0.1`) and stages all manifests in the commit without manual effort.
- **Milestone Promotions**: `scripts/bump-version.py` advances `patch`, `minor`, `major`, or `stable` releases.
- **Remote Synchronization**: `scripts/sync-github.ps1` or `scripts/sync-github.sh` automates committing,
  creating release tags (`v<x.y.z.w>`), verifying audit gates, and pushing to the configured GitHub repository.

## Quick start

Prerequisites: Rust 1.97.1 + Cargo (pinned by `rust-toolchain`) and Python 3.

```bash
python3 scripts/verify-source.py --worktree
cargo build --release --locked -p omni-stage0
cargo install --path crates/omni-stage0 --locked --force
omni --version
omni doctor
omni check examples/native_edition1.omni
omni run examples/native_edition1.omni
```

Expected compiler identity is `omni 0.2.0.0`. The example prints `42` and exits with 42 on
x86-64 Linux/WSL2.

## Conformance

```bash
python3 scripts/historical-conformance.py --omni omni
python3 scripts/native-conformance.py --omni omni
python3 scripts/native-conformance.py --omni omni \
  --manifest conformance/native_layout_v0_1_3/manifest.json
python3 scripts/native-conformance.py --omni omni \
  --manifest conformance/native_value_abi_v0_1_4/manifest.json
```

The v0.1.4.1.1 corpus covers aggregate arguments/returns, enum round-trips, String
pass/return/runtime printing, arbitrary non-UTF-8 Bytes output, dynamic byte indexing and
bounds faults, UTF-8 String indexing refusal, and the still-deferred aggregate-mutation
boundary.

Full release qualification:

```bash
./scripts/qualify-release.sh
```

## Repository map

- `crates/omni-stage0` — bootstrap CLI (`omni`).
- `crates/omni-compiler` — frontend, semantic passes, MIR and tooling integration.
- `crates/lir` — typed target-neutral LIR including bounded pointer/value ABI operations.
- `crates/codegen-native` — owned x86-64 Linux AOT backend; canonical deployment path.
- `crates/omni-stdlib` — Rust bootstrap allocator/scalar-cell collection foundation; not yet the normative Omni-source stdlib.
- `crates/codegen-cranelift`, `crates/codegen-llvm` — development/archive boundaries; execution fails closed.
- `crates/codegen-mlir`, `crates/codegen-wasm` — experimental artifact paths only.
- `crates/polonius_engine_*` — ownership research; not the production v0.2 soundness engine.
- `conformance/native_scalar_v0_1_2` — cumulative scalar/native corpus.
- `conformance/native_layout_v0_1_3` — cumulative local aggregate/layout corpus.
- `conformance/native_value_abi_v0_1_4` — current value-ABI/bytes corpus.
- `spec/` — Edition-1 candidate snapshot and implementation notes.

## Project direction

- [Current implementation matrix](docs/CURRENT_IMPLEMENTATION_MATRIX.md)
- [v0.1.4.1.1 milestone report](docs/MILESTONE_0.1.4.1_STRING_BYTE_VALUE_ABI_COLLECTIONS_FOUNDATION.md)
- [v0.1.3 native-data-layout milestone](docs/MILESTONE_0.1.3_NATIVE_DATA_LAYOUT_I.md)
- [Versioning/self-hosting plan](docs/VERSIONING_AND_BOOTSTRAP_PLAN.md)
- [Installation and qualification](INSTALL.md)
- [Specification baseline](spec/README.md)

Rust remains the bootstrap implementation language through the planned 2.0 line. The 2.x
transition ports the compiler to Omni; 3.0 remains the target for a fully self-hosted
canonical toolchain.
