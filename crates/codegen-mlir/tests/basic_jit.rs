use codegen_mlir::{compile_and_run_with_mlir, compile_and_run_with_mlir_jit};

#[test]
fn mlir_execution_is_not_faked_by_another_backend() {
    let module = lir::example_module();
    assert!(compile_and_run_with_mlir(&module).is_err());
    assert!(compile_and_run_with_mlir_jit(&module).is_err());
}
