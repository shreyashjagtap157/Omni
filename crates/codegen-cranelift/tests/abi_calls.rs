#![cfg(feature = "use_cranelift")]

use codegen_cranelift::compile_and_run_with_jit;
use lir::{Function, Instr as LirInstr, Module, Type as LirType};

#[test]
fn function_call_params_and_return() {
    // add(a, b) -> a + b
    let add = Function::new(
        "add",
        vec![LirType::I64, LirType::I64],
        LirType::I64,
        vec![LirInstr::Add, LirInstr::Ret],
    );

    let main = Function::new(
        "main",
        vec![],
        LirType::I64,
        vec![
            LirInstr::Const(2),
            LirInstr::Const(3),
            LirInstr::Call("add".to_string()),
            LirInstr::Ret,
        ],
    );

    let mut m = Module::new();
    m.add_function(add);
    m.add_function(main);

    let res = compile_and_run_with_jit(&m).expect("JIT run failed");
    assert_eq!(res[0], 5);
}

#[test]
fn nested_calls_double() {
    // add(a, b) -> a + b
    let add = Function::new(
        "add",
        vec![LirType::I64, LirType::I64],
        LirType::I64,
        vec![LirInstr::Add, LirInstr::Ret],
    );

    // double(x) -> store x into slot0; load slot0; load slot0; call add; ret
    let double = Function::new(
        "double",
        vec![LirType::I64],
        LirType::I64,
        vec![
            LirInstr::Store(0),
            LirInstr::Load(0),
            LirInstr::Load(0),
            LirInstr::Call("add".to_string()),
            LirInstr::Ret,
        ],
    );

    let main = Function::new(
        "main",
        vec![],
        LirType::I64,
        vec![LirInstr::Const(7), LirInstr::Call("double".to_string()), LirInstr::Ret],
    );

    let mut m = Module::new();
    m.add_function(add);
    m.add_function(double);
    m.add_function(main);

    let res = compile_and_run_with_jit(&m).expect("JIT run failed");
    assert_eq!(res[0], 14);
}
