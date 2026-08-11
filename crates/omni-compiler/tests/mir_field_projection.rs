use omni_compiler::mir::{BasicBlock, Instruction, MirFunction, MirModule};
use omni_compiler::polonius;

#[test]
fn field_move_then_use_reports_error() {
    let mut module = MirModule::new();
    let mut f = MirFunction::new("main", false);
    let mut b = BasicBlock::new(0);
    // initialize x.a
    b.instrs.push(Instruction::ConstInt {
        dest: "x.a".to_string(),
        value: 1,
    });
    // linear-move x.a into y (consuming)
    b.instrs.push(Instruction::LinearMove {
        dest: "y".to_string(),
        src: "x.a".to_string(),
    });
    // use x.a after move -> should error
    b.instrs.push(Instruction::Print {
        src: "x.a".to_string(),
    });
    f.blocks.push(b);
    module.functions.push(f);

    let res = polonius::check_mir(&module);
    assert!(res.is_err(), "expected error for use-after-move of field");
}

#[test]
fn moving_base_moves_fields() {
    let mut module = MirModule::new();
    let mut f = MirFunction::new("main", false);
    let mut b = BasicBlock::new(0);
    // init a linear base and a linear field explicitly
    b.instrs.push(Instruction::ConstInt {
        dest: "seed".to_string(),
        value: 10,
    });
    b.instrs.push(Instruction::LinearMove {
        dest: "x".to_string(),
        src: "seed".to_string(),
    });
    b.instrs.push(Instruction::ConstInt {
        dest: "x.a".to_string(),
        value: 1,
    });
    b.instrs.push(Instruction::LinearMove {
        dest: "x.a".to_string(),
        src: "x.a".to_string(),
    });
    // linear-move base x to z -> marks x and x.a moved
    b.instrs.push(Instruction::LinearMove {
        dest: "z".to_string(),
        src: "x".to_string(),
    });
    // Consume z so the assertion isolates hierarchical base->field invalidation.
    b.instrs.push(Instruction::DropLinear {
        var: "z".to_string(),
    });
    // using x.a after moving base should be error
    b.instrs.push(Instruction::Print {
        src: "x.a".to_string(),
    });
    f.blocks.push(b);
    module.functions.push(f);

    let res = polonius::check_mir(&module);
    assert!(
        res.is_err(),
        "expected error for using field after moving base"
    );
}

#[test]
fn moving_one_field_does_not_move_sibling_field() {
    let mut module = MirModule::new();
    let mut f = MirFunction::new("main", false);
    let mut b = BasicBlock::new(0);
    b.instrs.push(Instruction::ConstInt {
        dest: "x.a".to_string(),
        value: 1,
    });
    b.instrs.push(Instruction::ConstInt {
        dest: "x.b".to_string(),
        value: 2,
    });
    b.instrs.push(Instruction::LinearMove {
        dest: "y".to_string(),
        src: "x.a".to_string(),
    });
    b.instrs.push(Instruction::DropLinear {
        var: "y".to_string(),
    });
    b.instrs.push(Instruction::Print {
        src: "x.b".to_string(),
    });
    f.blocks.push(b);
    module.functions.push(f);

    let res = polonius::check_mir(&module);
    assert!(
        res.is_ok(),
        "moving x.a must not poison sibling x.b: {:?}",
        res
    );
}

#[test]
fn moving_linear_field_keeps_sibling_observable_but_blocks_root() {
    let mut module = MirModule::new();
    let mut f = MirFunction::new("partial", false);
    let mut b = BasicBlock::new(0);
    b.instrs.push(Instruction::ConstInt {
        dest: "seed".into(),
        value: 1,
    });
    b.instrs.push(Instruction::LinearMove {
        dest: "p".into(),
        src: "seed".into(),
    });
    b.instrs.push(Instruction::LinearMove {
        dest: "x".into(),
        src: "p.x".into(),
    });
    b.instrs.push(Instruction::Print { src: "p.y".into() });
    b.instrs.push(Instruction::Print { src: "p".into() });
    f.blocks.push(b);
    module.functions.push(f);
    let err = polonius::check_mir(&module).expect_err("whole root use after field move must fail");
    assert!(err.contains("partially moved"), "unexpected error: {err}");
}

#[test]
fn moving_same_linear_field_twice_is_rejected() {
    let mut module = MirModule::new();
    let mut f = MirFunction::new("double_field", false);
    let mut b = BasicBlock::new(0);
    b.instrs.push(Instruction::ConstInt {
        dest: "seed".into(),
        value: 1,
    });
    b.instrs.push(Instruction::LinearMove {
        dest: "p".into(),
        src: "seed".into(),
    });
    b.instrs.push(Instruction::LinearMove {
        dest: "x".into(),
        src: "p.x".into(),
    });
    b.instrs.push(Instruction::LinearMove {
        dest: "x2".into(),
        src: "p.x".into(),
    });
    f.blocks.push(b);
    module.functions.push(f);
    let err = polonius::check_mir(&module).expect_err("field double move must fail");
    assert!(
        err.contains("moved linear value") || err.contains("moved field"),
        "unexpected error: {err}"
    );
}

#[test]
fn different_projected_moves_across_branches_become_conditional() {
    let mut module = MirModule::new();
    let mut f = MirFunction::new("partial_join", false);
    let mut entry = BasicBlock::new(0);
    entry.instrs.push(Instruction::ConstInt {
        dest: "seed".into(),
        value: 1,
    });
    entry.instrs.push(Instruction::LinearMove {
        dest: "p".into(),
        src: "seed".into(),
    });
    entry.instrs.push(Instruction::ConstBool {
        dest: "cond".into(),
        value: true,
    });
    entry.instrs.push(Instruction::JumpIf {
        cond: "cond".into(),
        target: 2,
    });
    f.blocks.push(entry);
    let mut left = BasicBlock::new(1);
    left.instrs.push(Instruction::LinearMove {
        dest: "lx".into(),
        src: "p.x".into(),
    });
    left.instrs
        .push(Instruction::DropLinear { var: "lx".into() });
    left.instrs.push(Instruction::Jump { target: 3 });
    f.blocks.push(left);
    let mut right = BasicBlock::new(2);
    right.instrs.push(Instruction::LinearMove {
        dest: "ry".into(),
        src: "p.y".into(),
    });
    right
        .instrs
        .push(Instruction::DropLinear { var: "ry".into() });
    right.instrs.push(Instruction::Jump { target: 3 });
    f.blocks.push(right);
    let mut join = BasicBlock::new(3);
    join.instrs.push(Instruction::Print { src: "p.x".into() });
    f.blocks.push(join);
    module.functions.push(f);
    let err =
        polonius::check_mir(&module).expect_err("projected branch mismatch must be conditional");
    assert!(
        err.contains("conditionally available"),
        "unexpected error: {err}"
    );
}

#[test]
fn moved_linear_field_can_be_reinitialized_in_mir() {
    let mut module = MirModule::new();
    let mut f = MirFunction::new("reinit", false);
    let mut b = BasicBlock::new(0);
    b.instrs.push(Instruction::ConstInt {
        dest: "seed".into(),
        value: 1,
    });
    b.instrs.push(Instruction::LinearMove {
        dest: "p".into(),
        src: "seed".into(),
    });
    b.instrs.push(Instruction::FieldAccess {
        dest: "old_x".into(),
        base: "p".into(),
        field: "x".into(),
        linear: true,
    });
    b.instrs.push(Instruction::DropLinear {
        var: "old_x".into(),
    });
    b.instrs.push(Instruction::ConstInt {
        dest: "replacement".into(),
        value: 9,
    });
    b.instrs.push(Instruction::FieldAssign {
        base: "p".into(),
        field: "x".into(),
        src: "replacement".into(),
    });
    b.instrs.push(Instruction::DropLinear { var: "p".into() });
    f.blocks.push(b);
    module.functions.push(f);
    assert!(
        polonius::check_mir(&module).is_ok(),
        "reinitialized aggregate should be fully available"
    );
}

#[test]
fn live_linear_field_assignment_is_rejected_in_mir() {
    let mut module = MirModule::new();
    let mut f = MirFunction::new("bad_mutation", false);
    let mut b = BasicBlock::new(0);
    b.instrs.push(Instruction::ConstInt {
        dest: "seed".into(),
        value: 1,
    });
    b.instrs.push(Instruction::LinearMove {
        dest: "p".into(),
        src: "seed".into(),
    });
    b.instrs.push(Instruction::ConstInt {
        dest: "replacement".into(),
        value: 9,
    });
    b.instrs.push(Instruction::FieldAssign {
        base: "p".into(),
        field: "x".into(),
        src: "replacement".into(),
    });
    f.blocks.push(b);
    module.functions.push(f);
    let err = polonius::check_mir(&module).expect_err("live-field mutation must fail closed");
    assert!(
        err.contains("still initialized") || err.contains("mutation"),
        "unexpected error: {err}"
    );
}
