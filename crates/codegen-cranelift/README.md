# codegen-cranelift

Historical Cranelift development backend boundary.

In the remediated Omni v0.1.4 baseline, canonical execution is the owned native AOT backend.
The crate keeps a deterministic scalar LIR reference executor and textual renderer, but
`compile_and_run_with_jit` deliberately returns an error because the archived JIT did not yet
preserve every required checked-arithmetic and pointer semantic.

The previous JIT implementation is preserved under `docs/archive/unqualified-backends/` for
future differential re-qualification.

Public helpers:

- `render_lir_text(&lir::Module) -> String` — deterministic textual LIR renderer.
- `run_lir_interpreter(&lir::Module)` — scalar reference executor.
- `compile_and_run_with_jit(&lir::Module)` — fail-closed until re-qualified.
