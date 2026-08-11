use codegen_llvm::compile_and_run_with_llvm;
use lir::example_module;

#[test]
fn llvm_execution_is_fail_closed_until_requalified() {
    let err =
        compile_and_run_with_llvm(&example_module()).expect_err("LLVM must remain unqualified");
    assert!(err.contains("not qualified"));
}
