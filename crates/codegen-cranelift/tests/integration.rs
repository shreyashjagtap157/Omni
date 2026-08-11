use codegen_cranelift::render_lir_text;
use lir::example_module;

#[test]
fn compile_example_module() {
    let m = example_module();
    let out = render_lir_text(&m);
    assert!(out.contains("fn main"));
}
