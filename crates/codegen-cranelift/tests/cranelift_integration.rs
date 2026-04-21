#![cfg(feature = "use_cranelift")]

use lir::example_module;

#[test]
fn cranelift_runs_example_module() {
    let m = example_module();
    let res = codegen_cranelift::compile_and_run_with_jit(&m).expect("cranelift jit failed");
    assert_eq!(res[0], 42);
}
