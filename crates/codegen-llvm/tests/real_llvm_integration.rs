#[cfg(feature = "real_llvm")]
use codegen_llvm::compile_and_run_with_llvm;
#[cfg(feature = "real_llvm")]
use lir::example_module;

#[cfg(feature = "real_llvm")]
#[test]
fn real_llvm_runs_example_module() {
    let m = example_module();
    let res = compile_and_run_with_llvm(&m).expect("real_llvm run failed");
    assert_eq!(res, vec![42]);
}
