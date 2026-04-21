# codegen-wasm

Minimal WebAssembly backend for the LIR subset used by the Stage0 toolchain.
It currently emits parseable `.wasm` bytes for the simple arithmetic subset
covered by `lir::example_module()` and exports each function from the module.
