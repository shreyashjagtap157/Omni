use lir::example_module;
use codegen_llvm::compile_and_run_with_llvm;

#[test]
fn llvm_stub_fallback_runs_example() {
    let m = example_module();
    let res = compile_and_run_with_llvm(&m).expect("LLVM stub run failed");
    assert_eq!(res, vec![42]);
}
