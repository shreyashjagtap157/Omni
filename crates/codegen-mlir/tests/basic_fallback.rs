use lir::example_module;
use codegen_mlir::compile_and_run_with_mlir_fallback;

#[test]
fn mlir_fallback_runs_example() {
    let m = example_module();
    let res = compile_and_run_with_mlir_fallback(&m).expect("fallback run failed");
    assert_eq!(res, vec![42]);
}
