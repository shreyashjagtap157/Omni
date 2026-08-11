use codegen_cranelift::compile_and_run_with_jit;
use lir::example_module;

#[test]
fn jit_execution_is_fail_closed_until_requalified() {
    let err = compile_and_run_with_jit(&example_module()).expect_err("JIT must remain unqualified");
    assert!(err.contains("not qualified"));
}
