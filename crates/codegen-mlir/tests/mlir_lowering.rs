use codegen_mlir::emit_mlir_text;

#[test]
fn general_lir_lowering_fails_closed_until_qualified() {
    let module = lir::example_module();
    let err = emit_mlir_text(&module).expect_err("v0.1.4 MLIR lowering must be unavailable");
    assert!(err.contains("not qualified"));
}
