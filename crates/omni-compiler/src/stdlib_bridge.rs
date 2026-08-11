//! Bridge between `omni-compiler` and the `omni-stdlib` Rust crate.
//!
//! This module re-exports the public surface of `omni-stdlib` so that
//! downstream users of the compiler library (e.g. the VM, interpreter, and
//! future runtime) can reach standard-library types — `Gen<T>`, `Arena`,
//! `SlotMap`, `Vec`, `HashMap` — without depending on the stdlib crate
//! directly. The compiler doesn't have a `.omni` source loader yet, so the
//! `.omni` stdlib sources under `omni-stdlib/` aren't picked up at
//! compile time; this Rust-level bridge is what actually unblocks Rust
//! callers.

pub use omni_stdlib::*;

pub const STDLIB_VERSION: &str = crate::version::PROJECT_VERSION;
