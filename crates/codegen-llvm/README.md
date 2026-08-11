# codegen-llvm

This crate is an explicit **unqualified backend boundary** in Omni v0.1.4.

The former LLVM experiment is preserved in `docs/archive/unqualified-backends/`.
It is not used for canonical compilation and no local LLVM installation is required for the
v0.1.4 baseline. `compile_and_run_with_llvm` fails explicitly until a later milestone adds a
real LLVM integration with differential semantic tests.
