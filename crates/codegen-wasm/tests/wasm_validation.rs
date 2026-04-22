use codegen_wasm::emit_wasm_bytes;
use wasmparser::Validator;

#[test]
fn wasm_bytes_are_valid() {
    let module = lir::example_module();
    let bytes = emit_wasm_bytes(&module).expect("emit failed");

    let mut validator = Validator::new();
    if let Err(e) = validator.validate_all(&bytes) {
        eprintln!("Validation error: {:?}", e);
        panic!("WASM validation failed");
    }
}

#[test]
fn wasm_bytes_not_empty() {
    let module = lir::example_module();
    let bytes = emit_wasm_bytes(&module).expect("emit failed");

    assert!(!bytes.is_empty());
}
