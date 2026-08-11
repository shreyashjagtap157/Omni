# codegen-mlir

Experimental future MLIR backend infrastructure for Omni.

**v0.1.4 status:** not qualified as an LIR backend and not an execution runtime. `emit_mlir_text`, `compile_and_run_with_mlir`, and the historical `compile_and_run_with_mlir_jit` API fail explicitly. They never delegate to Cranelift.

The crate retains direct MLIR text fixtures for tensor/control-flow experimentation so future implementation work is not lost. Canonical Omni v0.1.4 execution is the owned x86-64 Linux AOT backend.
