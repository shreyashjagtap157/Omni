use lir::example_module;
use codegen_cranelift::compile_lir_stub;

#[test]
fn compile_example_module() {
    let m = example_module();
    let out = compile_lir_stub(&m);
    assert!(out.contains("fn main"));
}
