//! Historical Rust-translation backend boundary.
//!
//! Translating Omni MIR into Rust and invoking `rustc` is incompatible with the
//! canonical native-execution architecture and was never a qualified v0.1.4
//! backend. The prior experiment is archived under
//! `docs/archive/unqualified-backends/`.

use crate::mir::MirModule;

pub fn compile_and_run(_module: &MirModule) -> Result<(), String> {
    Err("Rust translation execution is not qualified in Omni v0.1.4; canonical execution uses the owned native AOT backend".to_string())
}
