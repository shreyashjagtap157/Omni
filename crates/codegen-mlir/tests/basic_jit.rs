use codegen_mlir::compile_and_run_with_mlir_jit;
use lir::example_module;

#[test]
fn mlir_jit_runs_example() {
    let m = example_module();
    let res = compile_and_run_with_mlir_jit(&m).expect("jit run failed");
    assert_eq!(res, vec![42]);
}