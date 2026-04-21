use lir::Module;

#[cfg(not(feature = "use_llvm"))]
use codegen_cranelift::compile_and_run_with_jit;
#[cfg(feature = "use_llvm")]
use codegen_llvm::compile_and_run_with_llvm;

/// Compile the given LIR module using the available codegen backend and run it.
/// Currently this forwards to the Cranelift JIT backend. When the LLVM
/// backend is implemented, this function will select the appropriate backend
/// based on Cargo features.
pub fn compile_and_run(module: &Module) -> Result<Vec<i64>, String> {
    #[cfg(feature = "use_llvm")]
    {
        return compile_and_run_with_llvm(module);
    }

    #[cfg(not(feature = "use_llvm"))]
    {
        // The Cranelift backend is the default backend for development builds.
        compile_and_run_with_jit(module)
    }
}
