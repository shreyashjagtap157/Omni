#![cfg(feature = "use_cranelift")]

use codegen_cranelift::compile_and_run_with_jit;
use lir::{Function, Instr as LirInstr, Module, Type as LirType};

#[test]
fn entry_multi_return_wrapper() {
    let main = Function::new_multi(
        "main",
        vec![],
        vec![LirType::I64, LirType::I64],
        vec![LirInstr::Const(3), LirInstr::Const(7), LirInstr::Ret],
    );

    let mut m = Module::new();
    m.add_function(main);

    let res = compile_and_run_with_jit(&m).expect("JIT run failed");
    assert_eq!(res, vec![3, 7]);
}
