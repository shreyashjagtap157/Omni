use codegen_mlir::emit_mlir_text;

#[test]
fn mlir_text_emit_works() {
    let module = lir::example_module();
    let text = emit_mlir_text(&module);

    assert!(!text.is_empty());
    assert!(text.contains("module"));
    assert!(text.contains("func.func"));
}

#[test]
fn mlir_has_required_dialects() {
    let module = lir::example_module();
    let text = emit_mlir_text(&module);

    assert!(text.contains("arith") || text.contains("func"));
}
