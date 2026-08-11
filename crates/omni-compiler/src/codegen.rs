use crate::driver::Backend;
use lir::Module;

pub fn compile_and_run(_module: &Module, backend: Backend) -> Result<Vec<i64>, String> {
    match backend {
        Backend::Cranelift => {
            #[cfg(feature = "dev-jit")]
            {
                codegen_cranelift::compile_and_run_with_jit(_module)
            }
            #[cfg(not(feature = "dev-jit"))]
            {
                Err("Cranelift JIT backend is disabled; install/build with feature 'dev-jit'".to_string())
            }
        }
        Backend::Native => Err(
            "the native backend emits an executable artifact; use compile_to_aot or compile_and_run_aot"
                .to_string(),
        ),
        #[cfg(feature = "use_llvm")]
        Backend::LLVM => codegen_llvm::compile_and_run_with_llvm(_module),
        #[cfg(not(feature = "use_llvm"))]
        Backend::LLVM => Err("LLVM backend not available (enable use_llvm feature)".to_string()),
        Backend::Wasm => Err(
            "the Wasm backend emits an optional artifact and is not an Omni execution runtime; use the emit-wasm command when the feature is enabled"
                .to_string(),
        ),
        Backend::Rust => Err(
            "the historical Rust translation backend is not qualified in v0.1.4; canonical execution is native AOT"
                .to_string(),
        ),
    }
}

/// Compile LIR directly to a platform-native executable.
///
/// The canonical owned backend currently emits x86-64 Linux ELF64 machine
/// code directly. It does not invoke a VM, JIT, C compiler, assembler, or
/// linker. More target formats are added behind the same API.
pub fn compile_to_aot(
    module: &Module,
    output_path: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    codegen_native::compile_to_native(module, output_path)
}

/// Compile a LIR module with the owned AOT backend, execute the resulting
/// native process, and capture its process output.
pub fn compile_and_run_aot(module: &Module) -> Result<codegen_native::NativeRunResult, String> {
    codegen_native::compile_and_run_native(module)
}
