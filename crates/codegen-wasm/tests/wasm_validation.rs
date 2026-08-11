use codegen_wasm::emit_wasm_bytes;
use lir::{Function, Instr, Module, Type};
use wasmparser::Validator;

#[test]
fn qualified_wasm_bytes_are_valid() {
    let bytes = emit_wasm_bytes(&lir::example_module()).expect("emit failed");
    Validator::new()
        .validate_all(&bytes)
        .expect("generated Wasm must validate");
}

#[test]
fn unsupported_memory_ops_fail_closed() {
    let mut module = Module::new();
    module.add_function(Function::new(
        "main",
        vec![],
        Type::I64,
        vec![Instr::GetAddr(0), Instr::Ret],
        vec![],
    ));
    assert!(emit_wasm_bytes(&module).is_err());
}
