use codegen_mlir::emit_control_flow_demo_mlir_text;

#[test]
fn control_flow_demo_text_covers_cf_ops() {
    let text = emit_control_flow_demo_mlir_text();

    assert!(text.contains("cf.cond_br"));
    assert!(text.contains("cf.br"));
    assert!(text.contains("func.return"));
}