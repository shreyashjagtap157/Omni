use omni_compiler::mir;
use omni_compiler::polonius;

#[test]
fn use_after_move_across_blocks_reports_error() {
    let mut module = mir::MirModule::new();
    let mut func = mir::MirFunction::new("main");

    let mut block0 = mir::BasicBlock::new(0);
    block0.instrs.push(mir::Instruction::ConstInt { dest: "a".to_string(), value: 1 });
    block0.instrs.push(mir::Instruction::Move { dest: "b".to_string(), src: "a".to_string() });

    let mut block1 = mir::BasicBlock::new(1);
    block1.instrs.push(mir::Instruction::Print { src: "a".to_string() });

    func.blocks.push(block0);
    func.blocks.push(block1);
    module.functions.push(func);

    // Ensure tests use the in-repo mock solver regardless of external env.
    std::env::remove_var("OMNI_USE_POLONIUS");
    let res = polonius::check_mir(&module);
    assert!(res.is_err(), "expected solver to report use-after-move across blocks");
}

#[test]
fn reinit_between_blocks_allows_use() {
    let mut module = mir::MirModule::new();
    let mut func = mir::MirFunction::new("main");

    let mut block0 = mir::BasicBlock::new(0);
    block0.instrs.push(mir::Instruction::ConstInt { dest: "a".to_string(), value: 1 });
    block0.instrs.push(mir::Instruction::Move { dest: "b".to_string(), src: "a".to_string() });

    let mut block1 = mir::BasicBlock::new(1);
    block1.instrs.push(mir::Instruction::ConstInt { dest: "a".to_string(), value: 2 });
    block1.instrs.push(mir::Instruction::Print { src: "a".to_string() });

    func.blocks.push(block0);
    func.blocks.push(block1);
    module.functions.push(func);

    // Ensure tests use the in-repo mock solver regardless of external env.
    std::env::remove_var("OMNI_USE_POLONIUS");
    let res = polonius::check_mir(&module);
    assert!(res.is_ok(), "expected solver to accept reinitialized var across blocks");
}
