use codegen_cranelift::run_lir_interpreter;
use lir::{Function, Instr, Module, Type};

fn module_with(body: Vec<Instr>) -> Module {
    let mut module = Module::new();
    module.add_function(Function::new("main", vec![], Type::I64, body, vec![]));
    module
}

#[test]
fn interpreter_rejects_integer_overflow() {
    let module = module_with(vec![
        Instr::Const(i64::MAX),
        Instr::Const(1),
        Instr::Add,
        Instr::Ret,
    ]);
    let err = match run_lir_interpreter(&module) {
        Ok(_) => panic!("expected overflow error"),
        Err(e) => e,
    };
    assert!(err.contains("overflow"));
}

#[test]
fn interpreter_rejects_division_by_zero() {
    let module = module_with(vec![
        Instr::Const(1),
        Instr::Const(0),
        Instr::Div,
        Instr::Ret,
    ]);
    let err = match run_lir_interpreter(&module) {
        Ok(_) => panic!("expected division error"),
        Err(e) => e,
    };
    assert!(err.contains("division by zero"));
}
