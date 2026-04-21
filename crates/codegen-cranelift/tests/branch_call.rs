#![cfg(feature = "use_cranelift")]

use codegen_cranelift::compile_and_run_with_jit;
use lir::{Function, Instr as LirInstr, Module, Type as LirType};

#[test]
fn branch_returns_expected_value() {
    // Construct a function that pushes 1, conditionally branches,
    // and returns different constants from each branch.
    // Instruction layout:
    // 0: Const(1)
    // 1: CondJump { if_true: 4, if_false: 2 }
    // 2: Const(10)
    // 3: Ret
    // 4: Const(20)
    // 5: Ret

    let body = vec![
        LirInstr::Const(1),
        LirInstr::CondJump { if_true: 4, if_false: 2 },
        LirInstr::Const(10),
        LirInstr::Ret,
        LirInstr::Const(20),
        LirInstr::Ret,
    ];

    let main = Function::new("main", vec![], LirType::I64, body);
    let mut m = Module::new();
    m.add_function(main);

    let res = compile_and_run_with_jit(&m).expect("JIT run failed");
    assert_eq!(res[0], 20, "expected true-branch to return 20");
}
