use codegen_llvm::{compile_and_run_with_llvm, LLVM_EXECUTION_QUALIFIED};
use lir::{Function, Module, Type};

#[test]
fn llvm_execution_is_explicitly_unqualified_in_v0_1_3() {
    const {
        assert!(!LLVM_EXECUTION_QUALIFIED);
    }
}

#[test]
fn llvm_execution_fails_closed_instead_of_falling_back() {
    let module = Module {
        functions: vec![Function {
            name: "main".to_string(),
            params: vec![],
            rets: vec![Type::I64],
            body: vec![],
            effects: vec![],
        }],
    };
    let error = compile_and_run_with_llvm(&module).expect_err("LLVM must be fail-closed");
    assert!(error.contains("not qualified"));
    assert!(error.contains("owned native AOT"));
}
