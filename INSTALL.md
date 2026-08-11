# Installing the Omni v0.1.4.1.1 bootstrap compiler

## Supported canonical host

The qualified native AOT target for v0.1.4.1.1 is x86-64 Linux, including WSL2. Other hosts may
build frontend/tooling components, but they are outside the canonical value-ABI claim.

## Prerequisites

- Rust 1.97.1 and Cargo (`rust-toolchain`).
- Python 3 for audits/conformance.
- Standard x86-64 Linux userspace for owned ELF64 execution.
- LLVM is not required by the default compiler path.

## Build and install

```bash
cargo build --release --locked -p omni-stage0
cargo install --path crates/omni-stage0 --locked --force
omni --version
omni doctor
```

Expected identity: `omni 0.1.4.1`.

## Smoke and conformance

```bash
omni check examples/native_edition1.omni
omni run examples/native_edition1.omni
python3 scripts/historical-conformance.py --omni omni
python3 scripts/native-conformance.py --omni omni
python3 scripts/native-conformance.py --omni omni --manifest conformance/native_layout_v0_1_3/manifest.json
python3 scripts/native-conformance.py --omni omni --manifest conformance/native_value_abi_v0_1_4/manifest.json
```

The current value-ABI manifest has 10 CLI-level cases and includes binary stdout checks.

## Full qualification

Linux/WSL2:

```bash
./scripts/qualify-release.sh
```

PowerShell:

```powershell
.\scripts\qualify-release.ps1
```

The release gate includes source audits, fmt, workspace-wide Clippy/tests/build, release
installation, doctor/smoke checks, all cumulative native corpora, and the inherited
60-second lexer/parser fuzz requirement.

## Offline builds

A separate offline bundle may carry a vendored Cargo dependency tree. `vendor/`, Cargo
`target/`, fuzz corpora and toolchain caches are not part of the lean source release.

## Semantic boundary

Read `docs/MILESTONE_0.1.4.1_STRING_BYTE_VALUE_ABI_COLLECTIONS_FOUNDATION.md` for the exact
qualified value ABI. Ownership-sensitive mutation, nontrivial destruction, mutable
heap-owning String/Bytes and source-level generic collections remain later work. Cranelift
JIT and LLVM execution remain deliberately unqualified; owned x86-64 Linux AOT is canonical.
