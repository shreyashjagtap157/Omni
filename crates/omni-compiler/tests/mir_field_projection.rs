use omni_compiler::mir::{BasicBlock, Instruction, MirFunction, MirModule};
use omni_compiler::polonius;

#[test]
fn field_move_then_use_reports_error() {
    let mut module = MirModule::new();
    let mut f = MirFunction::new("main");
    let mut b = BasicBlock::new(0);
    // initialize x.a
    b.instrs.push(Instruction::ConstInt {
        dest: "x.a".to_string(),
        value: 1,
    });
    // move x.a into y
    b.instrs.push(Instruction::Move {
        dest: "y".to_string(),
        src: "x.a".to_string(),
    });
    // use x.a after move -> should error
    b.instrs.push(Instruction::Print {
        src: "x.a".to_string(),
    });
    f.blocks.push(b);
    module.functions.push(f);

    // Ensure tests use the in-repo mock solver regardless of external env.
    std::env::remove_var("OMNI_USE_POLONIUS");
    let res = polonius::check_mir(&module);
    assert!(res.is_err(), "expected error for use-after-move of field");
}

#[test]
fn moving_base_moves_fields() {
    let mut module = MirModule::new();
    let mut f = MirFunction::new("main");
    let mut b = BasicBlock::new(0);
    // init base and a field explicit
    b.instrs.push(Instruction::ConstInt {
        dest: "x".to_string(),
        value: 10,
    });
    b.instrs.push(Instruction::ConstInt {
        dest: "x.a".to_string(),
        value: 1,
    });
    // move base x to z -> should mark x and x.a moved
    b.instrs.push(Instruction::Move {
        dest: "z".to_string(),
        src: "x".to_string(),
    });
    // using x.a after moving base should be error
    b.instrs.push(Instruction::Print {
        src: "x.a".to_string(),
    });
    f.blocks.push(b);
    module.functions.push(f);

    // Ensure tests use the in-repo mock solver regardless of external env.
    std::env::remove_var("OMNI_USE_POLONIUS");
    let res = polonius::check_mir(&module);
    assert!(
        res.is_err(),
        "expected error for using field after moving base"
    );
}
