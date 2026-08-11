//! LLVM development-oracle boundary.
//!
//! Omni v0.1.4 does not qualify LLVM as an execution backend. The historical
//! implementation is archived under `docs/archive/unqualified-backends/`.
//! Canonical execution uses the owned native AOT backend.

use lir::Module;

/// Whether this release qualifies LLVM execution semantics.
pub const LLVM_EXECUTION_QUALIFIED: bool = false;

/// LLVM execution is deliberately fail-closed in v0.1.4.
pub fn compile_and_run_with_llvm(_module: &Module) -> Result<Vec<i64>, String> {
    Err(
        "LLVM execution is not qualified in Omni v0.1.4; use the owned native AOT backend"
            .to_string(),
    )
}
