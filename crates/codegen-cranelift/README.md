# codegen-cranelift

Cranelift-backed codegen crate for Omni (development backend).

Features
- `run_lir_interpreter`: deterministic, dependency-free LIR interpreter (default).
- `use_cranelift` (feature): optional Cranelift JIT backend for native execution.

Quickstart

- Run interpreter tests (default):

```bash
cargo test -p codegen-cranelift
```

- Run Cranelift-backed tests (requires network to fetch Cranelift crates):

```bash
cargo test -p codegen-cranelift --features use_cranelift
```

API
- `compile_lir_stub(&lir::Module) -> String` — textual LIR renderer for debugging.
- `run_lir_interpreter(&lir::Module) -> Result<RunResult,String>` — run LIR via interpreter.
- `compile_and_run_with_jit(&lir::Module) -> Result<Vec<i64>,String>` — JIT-run via Cranelift (feature `use_cranelift`). Returns the entry function's return values as a vector (single-value entry functions are common).

Notes
- The Cranelift backend is intentionally small-surface and best-effort: it currently supports integer arithmetic, local slots, and imported `print(i64)` calls. Control-flow support is incremental.
