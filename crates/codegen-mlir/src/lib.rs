use lir::Module;

/// Placeholder MLIR backend shim.
///
/// This crate provides a stable API for the future MLIR backend. For now
/// it delegates to the Cranelift backend as a safe fallback so higher-level
/// code can call into an MLIR API without requiring an actual MLIR toolchain.

pub fn compile_and_run_with_mlir(module: &Module) -> Result<Vec<i64>, String> {
    // In future: attempt to lower to MLIR dialects, invoke MLIR pipeline,
    // and run on CPU/GPU/TPU. For now, return an error directing users to
    // the fallback API.
    Err("MLIR backend not implemented in this workspace; call the fallback API instead".to_string())
}

/// Fallback that uses the Cranelift backend when MLIR is unavailable.
pub fn compile_and_run_with_mlir_fallback(module: &Module) -> Result<Vec<i64>, String> {
    // Delegate to the existing Cranelift integration so tests and CI can exercise
    // the multi-backend plumbing without requiring an MLIR toolchain.
    codegen_cranelift::compile_and_run_with_jit(module)
}
