# codegen-mlir

Placeholder MLIR backend crate for Omni.

This crate provides a stable API for a future MLIR integration. Currently it
offers a fallback that delegates to the existing Cranelift backend so the
multi-backend plumbing can be exercised in CI and tests without an MLIR
toolchain.

Run the crate tests with:

```bash
cargo test -p codegen-mlir
```

When `use_mlir` is implemented, enable the corresponding feature and ensure
the MLIR/LLVM toolchain is installed and available on the PATH.
