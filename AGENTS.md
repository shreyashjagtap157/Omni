# Agent Guide for Omni

Omni is a native-first programming-language implementation. The Rust code in this
repository is a **bootstrap compiler**, not a runtime that hosted Omni programs depend
on. The qualified v0.1.3 execution path emits owned x86-64 Linux ELF64 machine code.

## Sources of truth

Use these in this order:

1. `spec/README.md` and `spec/edition1/` — language-definition target.
2. `docs/CURRENT_IMPLEMENTATION_MATRIX.md` — actual implementation coverage.
3. `docs/MILESTONE_0.1.3_NATIVE_DATA_LAYOUT_I.md` — current milestone contract.
4. `docs/VERSIONING_AND_BOOTSTRAP_PLAN.md` — version/milestone sequence.
5. `docs/LINEAGE_REMEDIATION_AUDIT_0.0.1_TO_0.1.2.md` — repaired historical debt.
6. `scripts/audit-baseline.py` — structural/lineage gate.
7. `scripts/qualify-release.sh` or `.ps1` — executable promotion gate.

Anything under `docs/archive/` is historical evidence only. `minimax_plan.md` is a
redirect, not an authoritative plan.

## Canonical execution contract

```text
Omni source
  -> lexer/parser/resolution/static semantics
  -> MIR + verification
  -> LIR + verification
  -> owned native backend
  -> ELF/PE/Mach-O/freestanding image as targets mature
  -> OS/firmware loader
  -> processor
```

No VM, bytecode interpreter, JIT, LLVM runtime, Cranelift runtime, or Rust runtime is
part of the canonical execution semantics. At v0.1.3 only the owned x86-64 Linux ELF64
path is qualified.

## Canonical semantic closure versus workspace integrity

The canonical compiler dependency closure intentionally stays small:

- `omni-stage0`
- `omni-compiler`
- `codegen-native`
- `lir`
- `omni-stdlib`

Cranelift, LLVM, MLIR, Wasm, upstream-Polonius experiments, release tooling, fuzz tools,
and self-host scaffolding remain outside the **semantic qualification claim** unless a
later milestone explicitly qualifies them. They are nevertheless required to compile,
pass warning-denied Clippy, and pass their applicable Rust tests in the v0.1.3
whole-workspace integrity gate. Workspace compilation never makes an experimental
backend a semantic oracle.

## Build and qualification

On a Rust-equipped x86-64 Linux/WSL2 host:

```bash
python3 scripts/audit-baseline.py --worktree
python3 scripts/verify-source.py --worktree
cargo fmt --all -- --check
cargo clippy --workspace --locked --all-targets -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
cargo build --release --locked -p omni-stage0
cargo install --path crates/omni-stage0 --locked --force
python3 scripts/historical-conformance.py --omni omni
python3 scripts/native-conformance.py --omni omni
python3 scripts/native-conformance.py --omni omni \
  --manifest conformance/native_layout_v0_1_3/manifest.json
```

Prefer the bundled one-command gate:

```bash
./scripts/qualify-release.sh
```

The whole-workspace checks are Rust-integrity gates. The canonical semantic claim is
still determined only by the explicitly qualified native conformance corpora.

## Testing compiler changes

Use the canonical native compiler path (`Backend::Native`) for current pipeline tests.
Do not make Cranelift or LLVM the semantic oracle. A future backend becomes a valid
differential oracle only after it independently passes the same semantic conformance
cases as the owned native path.

A feature counts as implemented only when it is complete through the whole wedge:

```text
source grammar
-> resolution/static checking
-> MIR semantics + verifier
-> LIR/lowering when applicable
-> native target lowering
-> positive and negative conformance
```

Parser/AST recognition by itself is not implementation completion.

## v0.1.3 dependable native subset

The owned x86-64 Linux path retains the v0.1.2 scalar/control-flow subset and qualifies
the first local aggregate-layout wedge documented in
`docs/CURRENT_IMPLEMENTATION_MATRIX.md`:

- scalar integer/boolean locals, checked arithmetic and comparisons;
- `if`/`while`/`loop`, `break`/`continue`, direct scalar calls/returns;
- nominal local structs with declaration-order scalar-cell layout and field reads;
- structural scalar-cell tuples and fixed arrays;
- bounds-checked dynamic array/local-slice indexing;
- non-escaping constant-range slice views;
- nominal local tagged enums with fieldless/scalar payloads and exhaustive variant match;
- pre-emission aggregate alignment/frame validation and runtime bounds faults.

Aggregate arguments/returns/escaping slices and the stable value ABI remain v0.1.4.1.1.
Ownership-sensitive aggregate writes and nontrivial destruction remain v0.2.0.
Anything outside the qualified subset must fail closed rather than fabricate a value,
silently emit `Nop`, route through another backend, or accept an unsound ownership result.

## Ownership boundary

Full Edition-1 ownership/borrowing/regions are a v0.2.0 milestone. The v0.1.3 canonical
path does **not** claim production borrow-checker soundness. Ownership-sensitive MIR
that requires the future checker must be rejected. The old experimental Polonius source
is retained only as research/compatibility infrastructure and is not a soundness proof.

## Heavy/optional components

- `codegen-native`: canonical owned backend; default dependency is only `lir`.
- `codegen-cranelift`: unqualified/fail-closed execution boundary in v0.1.3.
- `codegen-llvm`: unqualified/fail-closed execution boundary in v0.1.3.
- `codegen-mlir`: experimental artifact representation only.
- `codegen-wasm`: optional constrained artifact experiment; not native execution.
- `polonius_engine_*`: experimental/future ownership work; not current soundness proof.
- `omni-selfhost`: future bootstrap-transition scaffolding; not self-host completion.

Local LLVM SDKs/build trees, Cargo `target/`, vendored registries used only for offline
qualification, and generated fuzz corpora are not lean source artifacts. Keep fuzz
**source**, not large generated fuzz output.

## Error discipline

Never implement an unsupported semantic operation by:

- returning zero/empty/unit merely to continue;
- mapping an unknown variable to slot 0;
- lowering an unknown operation to `Nop`;
- silently selecting a different backend;
- treating a mock/prototype checker as proof;
- swallowing malformed LSP/package/build inputs;
- changing checked arithmetic behavior in optimization.

Return a stable diagnostic/error instead.

## Version rule

0.0.1 through 1.0.0 build the dependable Rust-bootstrap Omni Core. 1.0.1 through 2.0.0
complete the broader specified language/profile set in Rust. The 2.x transition then
ports compiler components to Omni; 3.0.0 is the target for the fully self-hosted
canonical toolchain. See `docs/VERSIONING_AND_BOOTSTRAP_PLAN.md` for exact gates.
