use omni_compiler::mir;
use omni_compiler::parse_file;
use omni_compiler::polonius;
use std::io::Write;

#[test]
fn use_after_linear_move_is_reported() {
    // v0.2.0.0 begins qualifying ownership-sensitive MIR. This case builds
    // an actual use-after-linear-move sequence and verifies it is rejected.
    let mut module = mir::MirModule::new();
    let mut func = mir::MirFunction::new("main", false);
    let mut block0 = mir::BasicBlock::new(0);
    block0.instrs.push(mir::Instruction::ConstInt {
        dest: "a".to_string(),
        value: 1,
    });
    block0.instrs.push(mir::Instruction::LinearMove {
        dest: "b".to_string(),
        src: "a".to_string(),
    });
    block0.instrs.push(mir::Instruction::Print {
        src: "a".to_string(),
    });
    func.blocks.push(block0);
    module.functions.push(func);
    let res = polonius::check_mir(&module);
    assert!(
        res.is_err(),
        "expected ownership check to report use-after-linear-move"
    );
}

#[test]
fn non_linear_copy_is_not_a_use_after_move() {
    // Non-linear scalar `Move` remains copy-like in the current scalar subset.
    let mut module = mir::MirModule::new();
    let mut func = mir::MirFunction::new("main", false);
    let mut block0 = mir::BasicBlock::new(0);
    block0.instrs.push(mir::Instruction::ConstInt {
        dest: "a".to_string(),
        value: 1,
    });
    block0.instrs.push(mir::Instruction::Move {
        dest: "b".to_string(),
        src: "a".to_string(),
    });
    block0.instrs.push(mir::Instruction::Print {
        src: "a".to_string(),
    });
    func.blocks.push(block0);
    module.functions.push(func);
    let res = polonius::check_mir(&module);
    assert!(
        res.is_ok(),
        "non-linear Move should be a copy, got error: {:?}",
        res
    );
}

#[test]
fn parse_then_check_arithmetic_program() {
    // Smoke test: parse, lower, and run borrow check on a simple program
    // that has no errors.
    let src = "let a = 1\nlet b = 2\nlet c = a + b\nprint c\n";
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();
    let prog = parse_file(path).expect("parse failed");
    let module = mir::lower_program_to_mir(&prog);
    let res = polonius::check_mir(&module);
    assert!(res.is_ok(), "unexpected error: {:?}", res);
}

fn linear_branch_fixture() -> mir::MirFunction {
    let mut func = mir::MirFunction::new("branchy", false);
    let mut entry = mir::BasicBlock::new(0);
    entry.instrs.push(mir::Instruction::ConstInt {
        dest: "seed".to_string(),
        value: 7,
    });
    entry.instrs.push(mir::Instruction::LinearMove {
        dest: "owned".to_string(),
        src: "seed".to_string(),
    });
    entry.instrs.push(mir::Instruction::ConstBool {
        dest: "cond".to_string(),
        value: true,
    });
    entry.instrs.push(mir::Instruction::JumpIf {
        cond: "cond".to_string(),
        target: 2,
    });
    func.blocks.push(entry);
    func
}

#[test]
fn linear_value_consumed_on_both_branches_is_ok() {
    let mut module = mir::MirModule::new();
    let mut func = linear_branch_fixture();

    let mut else_block = mir::BasicBlock::new(1);
    else_block.instrs.push(mir::Instruction::DropLinear {
        var: "owned".to_string(),
    });
    else_block.instrs.push(mir::Instruction::Jump { target: 3 });
    func.blocks.push(else_block);

    let mut then_block = mir::BasicBlock::new(2);
    then_block.instrs.push(mir::Instruction::Print {
        src: "owned".to_string(),
    });
    then_block.instrs.push(mir::Instruction::Jump { target: 3 });
    func.blocks.push(then_block);

    func.blocks.push(mir::BasicBlock::new(3));
    module.functions.push(func);

    assert!(
        polonius::check_mir(&module).is_ok(),
        "linear value consumed on every outgoing branch should be accepted"
    );
}

#[test]
fn linear_value_moved_on_only_one_branch_cannot_be_used_at_join() {
    let mut module = mir::MirModule::new();
    let mut func = linear_branch_fixture();

    let mut else_block = mir::BasicBlock::new(1);
    else_block.instrs.push(mir::Instruction::Jump { target: 3 });
    func.blocks.push(else_block);

    let mut then_block = mir::BasicBlock::new(2);
    then_block.instrs.push(mir::Instruction::Print {
        src: "owned".to_string(),
    });
    then_block.instrs.push(mir::Instruction::Jump { target: 3 });
    func.blocks.push(then_block);

    let mut join_block = mir::BasicBlock::new(3);
    join_block.instrs.push(mir::Instruction::Print {
        src: "owned".to_string(),
    });
    func.blocks.push(join_block);
    module.functions.push(func);

    let err = polonius::check_mir(&module).expect_err("conditional move must be rejected");
    assert!(
        err.contains("conditionally available"),
        "unexpected error: {err}"
    );
}

#[test]
fn linear_value_not_consumed_on_every_exit_path_is_rejected() {
    let mut module = mir::MirModule::new();
    let mut func = linear_branch_fixture();

    let mut else_block = mir::BasicBlock::new(1);
    else_block.instrs.push(mir::Instruction::Jump { target: 3 });
    func.blocks.push(else_block);

    let mut then_block = mir::BasicBlock::new(2);
    then_block.instrs.push(mir::Instruction::DropLinear {
        var: "owned".to_string(),
    });
    then_block.instrs.push(mir::Instruction::Jump { target: 3 });
    func.blocks.push(then_block);

    func.blocks.push(mir::BasicBlock::new(3));
    module.functions.push(func);

    let err = polonius::check_mir(&module).expect_err("conditional consumption must be rejected");
    assert!(
        err.contains("not consumed on every path"),
        "unexpected error: {err}"
    );
}

#[test]
fn linear_value_consumed_in_loop_is_rejected_at_loop_exit() {
    let mut module = mir::MirModule::new();
    let mut func = mir::MirFunction::new("loopy", false);

    // True preheader: `owned` is created once before entering the loop.
    let mut preheader = mir::BasicBlock::new(0);
    preheader.instrs.push(mir::Instruction::ConstInt {
        dest: "seed".to_string(),
        value: 7,
    });
    preheader.instrs.push(mir::Instruction::LinearMove {
        dest: "owned".to_string(),
        src: "seed".to_string(),
    });
    preheader.instrs.push(mir::Instruction::Jump { target: 1 });
    func.blocks.push(preheader);

    let mut header = mir::BasicBlock::new(1);
    header.instrs.push(mir::Instruction::ConstBool {
        dest: "cond".to_string(),
        value: true,
    });
    header.instrs.push(mir::Instruction::JumpIf {
        cond: "cond".to_string(),
        target: 2,
    });
    func.blocks.push(header);

    let mut body = mir::BasicBlock::new(2);
    body.instrs.push(mir::Instruction::Print {
        src: "owned".to_string(),
    });
    body.instrs.push(mir::Instruction::Jump { target: 1 });
    func.blocks.push(body);

    let mut exit = mir::BasicBlock::new(3);
    exit.instrs.push(mir::Instruction::DropLinear {
        var: "owned".to_string(),
    });
    func.blocks.push(exit);
    module.functions.push(func);

    let err = polonius::check_mir(&module).expect_err("loop consumption must poison exits");
    assert!(
        err.contains("conditionally available")
            || err.contains("not consumed on every path")
            || err.contains("moved"),
        "unexpected error: {err}"
    );
}

#[test]
fn linear_value_created_and_consumed_inside_loop_body_is_mir_valid() {
    let mut module = mir::MirModule::new();
    let mut func = mir::MirFunction::new("loop_local", false);

    let mut entry = mir::BasicBlock::new(0);
    entry.instrs.push(mir::Instruction::ConstBool {
        dest: "cond".to_string(),
        value: true,
    });
    entry.instrs.push(mir::Instruction::JumpIf {
        cond: "cond".to_string(),
        target: 1,
    });
    func.blocks.push(entry);

    let mut body = mir::BasicBlock::new(1);
    body.instrs.push(mir::Instruction::ConstInt {
        dest: "seed".to_string(),
        value: 7,
    });
    body.instrs.push(mir::Instruction::LinearMove {
        dest: "owned".to_string(),
        src: "seed".to_string(),
    });
    body.instrs.push(mir::Instruction::DropLinear {
        var: "owned".to_string(),
    });
    body.instrs.push(mir::Instruction::Jump { target: 0 });
    func.blocks.push(body);

    func.blocks.push(mir::BasicBlock::new(2));
    module.functions.push(func);

    assert!(
        polonius::check_mir(&module).is_ok(),
        "loop-local linear values consumed each iteration should be valid"
    );
}

#[test]
fn explicit_linear_double_drop_is_rejected() {
    let mut module = mir::MirModule::new();
    let mut func = mir::MirFunction::new("double_drop", false);
    let mut block = mir::BasicBlock::new(0);
    block.instrs.push(mir::Instruction::ConstInt {
        dest: "seed".into(),
        value: 1,
    });
    block.instrs.push(mir::Instruction::LinearMove {
        dest: "owned".into(),
        src: "seed".into(),
    });
    block.instrs.push(mir::Instruction::DropLinear {
        var: "owned".into(),
    });
    block.instrs.push(mir::Instruction::DropLinear {
        var: "owned".into(),
    });
    func.blocks.push(block);
    module.functions.push(func);
    let err = polonius::check_mir(&module).expect_err("explicit double drop must fail");
    assert!(
        err.contains("double-drop") || err.contains("moved linear"),
        "unexpected error: {err}"
    );
}

#[test]
fn compiler_cleanup_after_explicit_linear_consume_is_noop() {
    let mut module = mir::MirModule::new();
    let mut func = mir::MirFunction::new("cleanup_flag", false);
    let mut block = mir::BasicBlock::new(0);
    block.instrs.push(mir::Instruction::ConstInt {
        dest: "seed".into(),
        value: 1,
    });
    block.instrs.push(mir::Instruction::LinearMove {
        dest: "owned".into(),
        src: "seed".into(),
    });
    block.instrs.push(mir::Instruction::DropLinear {
        var: "owned".into(),
    });
    block.instrs.push(mir::Instruction::Drop {
        var: "owned".into(),
    });
    func.blocks.push(block);
    module.functions.push(func);
    assert!(
        polonius::check_mir(&module).is_ok(),
        "guarded cleanup after move/drop must be a no-op"
    );
}

#[test]
fn mir_mutable_reborrow_from_shared_parent_is_rejected() {
    let mut module = mir::MirModule::new();
    let mut func = mir::MirFunction::new("bad_reborrow", false);
    let mut block = mir::BasicBlock::new(0);
    block.instrs.push(mir::Instruction::ConstInt {
        dest: "x".into(),
        value: 1,
    });
    block.instrs.push(mir::Instruction::Borrow {
        dest: "parent".into(),
        place: "x".into(),
        mutable: false,
    });
    block.instrs.push(mir::Instruction::Reborrow {
        dest: "child".into(),
        parent: "parent".into(),
        mutable: true,
    });
    block.instrs.push(mir::Instruction::Deref {
        dest: "v".into(),
        reference: "child".into(),
    });
    func.blocks.push(block);
    module.functions.push(func);
    let err = polonius::check_mir(&module).expect_err("mutable reborrow from shared must fail");
    assert!(
        err.contains("mutable reborrow") || err.contains("shared reference"),
        "{err}"
    );
}

#[test]
fn mir_parent_is_suspended_while_child_reborrow_is_live() {
    let mut module = mir::MirModule::new();
    let mut func = mir::MirFunction::new("suspended_parent", false);
    let mut block = mir::BasicBlock::new(0);
    block.instrs.push(mir::Instruction::ConstInt {
        dest: "x".into(),
        value: 1,
    });
    block.instrs.push(mir::Instruction::Borrow {
        dest: "parent".into(),
        place: "x".into(),
        mutable: true,
    });
    block.instrs.push(mir::Instruction::Reborrow {
        dest: "child".into(),
        parent: "parent".into(),
        mutable: false,
    });
    block.instrs.push(mir::Instruction::Deref {
        dest: "p".into(),
        reference: "parent".into(),
    });
    block.instrs.push(mir::Instruction::Deref {
        dest: "c".into(),
        reference: "child".into(),
    });
    func.blocks.push(block);
    module.functions.push(func);
    let err = polonius::check_mir(&module).expect_err("parent use during child must fail");
    assert!(err.contains("suspended"), "{err}");
}

#[test]
fn mir_parent_is_restored_after_child_last_use() {
    let mut module = mir::MirModule::new();
    let mut func = mir::MirFunction::new("restored_parent", false);
    let mut block = mir::BasicBlock::new(0);
    block.instrs.push(mir::Instruction::ConstInt {
        dest: "x".into(),
        value: 1,
    });
    block.instrs.push(mir::Instruction::Borrow {
        dest: "parent".into(),
        place: "x".into(),
        mutable: true,
    });
    block.instrs.push(mir::Instruction::Reborrow {
        dest: "child".into(),
        parent: "parent".into(),
        mutable: false,
    });
    block.instrs.push(mir::Instruction::Deref {
        dest: "c".into(),
        reference: "child".into(),
    });
    block.instrs.push(mir::Instruction::Deref {
        dest: "p".into(),
        reference: "parent".into(),
    });
    func.blocks.push(block);
    module.functions.push(func);
    assert!(polonius::check_mir(&module).is_ok());
}

#[test]
fn mir_shared_reference_cannot_strengthen_to_mutable_parameter() {
    let mut module = mir::MirModule::new();

    let mut callee = mir::MirFunction::new("set", false);
    callee.params = vec!["r".into()];
    callee.param_types = vec![Some("&mut i64".into())];
    callee.return_type = Some("unit".into());
    callee.returns_value = false;
    callee.blocks.push(mir::BasicBlock::new(0));
    module.functions.push(callee);

    let mut caller = mir::MirFunction::new("main", false);
    let mut block = mir::BasicBlock::new(0);
    block.instrs.push(mir::Instruction::ConstInt {
        dest: "x".into(),
        value: 1,
    });
    block.instrs.push(mir::Instruction::Borrow {
        dest: "r".into(),
        place: "x".into(),
        mutable: false,
    });
    block.instrs.push(mir::Instruction::Call {
        dest: "u".into(),
        func: "set".into(),
        args: vec!["r".into()],
    });
    caller.blocks.push(block);
    module.functions.push(caller);

    let err = polonius::check_mir(&module).expect_err("shared reference must not strengthen");
    assert!(
        err.contains("shared reference") && err.contains("mutable reference"),
        "{err}"
    );
}

#[test]
fn mir_mutable_reference_may_weaken_to_shared_parameter() {
    let mut module = mir::MirModule::new();

    let mut callee = mir::MirFunction::new("read", false);
    callee.params = vec!["r".into()];
    callee.param_types = vec![Some("&i64".into())];
    callee.return_type = Some("i64".into());
    callee.returns_value = true;
    let mut callee_block = mir::BasicBlock::new(0);
    callee_block.instrs.push(mir::Instruction::Deref {
        dest: "v".into(),
        reference: "r".into(),
    });
    callee_block
        .instrs
        .push(mir::Instruction::Return { value: "v".into() });
    callee.blocks.push(callee_block);
    module.functions.push(callee);

    let mut caller = mir::MirFunction::new("main", false);
    let mut block = mir::BasicBlock::new(0);
    block.instrs.push(mir::Instruction::ConstInt {
        dest: "x".into(),
        value: 42,
    });
    block.instrs.push(mir::Instruction::Borrow {
        dest: "r".into(),
        place: "x".into(),
        mutable: true,
    });
    block.instrs.push(mir::Instruction::Call {
        dest: "v".into(),
        func: "read".into(),
        args: vec!["r".into()],
    });
    caller.blocks.push(block);
    module.functions.push(caller);

    assert!(polonius::check_mir(&module).is_ok());
}

#[test]
fn mir_safe_reference_cannot_be_passed_to_scalar_parameter() {
    let mut module = mir::MirModule::new();

    let mut callee = mir::MirFunction::new("scalar", false);
    callee.params = vec!["x".into()];
    callee.param_types = vec![Some("i64".into())];
    callee.return_type = Some("unit".into());
    callee.returns_value = false;
    callee.blocks.push(mir::BasicBlock::new(0));
    module.functions.push(callee);

    let mut caller = mir::MirFunction::new("main", false);
    let mut block = mir::BasicBlock::new(0);
    block.instrs.push(mir::Instruction::ConstInt {
        dest: "x".into(),
        value: 1,
    });
    block.instrs.push(mir::Instruction::Borrow {
        dest: "r".into(),
        place: "x".into(),
        mutable: false,
    });
    block.instrs.push(mir::Instruction::Call {
        dest: "u".into(),
        func: "scalar".into(),
        args: vec!["r".into()],
    });
    caller.blocks.push(block);
    module.functions.push(caller);

    let err = polonius::check_mir(&module).expect_err("reference-to-scalar ABI mismatch must fail");
    assert!(err.contains("non-reference parameter"), "{err}");
}

#[test]
fn mir_unresolved_call_with_safe_reference_fails_closed() {
    let mut module = mir::MirModule::new();
    let mut caller = mir::MirFunction::new("main", false);
    let mut block = mir::BasicBlock::new(0);
    block.instrs.push(mir::Instruction::ConstInt {
        dest: "x".into(),
        value: 1,
    });
    block.instrs.push(mir::Instruction::Borrow {
        dest: "r".into(),
        place: "x".into(),
        mutable: false,
    });
    block.instrs.push(mir::Instruction::Call {
        dest: "u".into(),
        func: "external".into(),
        args: vec!["r".into()],
    });
    caller.blocks.push(block);
    module.functions.push(caller);

    let err = polonius::check_mir(&module).expect_err("unresolved reference call must fail closed");
    assert!(err.contains("unresolved call"), "{err}");
}
