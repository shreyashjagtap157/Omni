use omni_compiler::mir;
use omni_compiler::polonius;

#[test]
fn export_nonempty_for_basic_module() {
    let mut module = mir::MirModule::new();
    let mut func = mir::MirFunction::new("main");
    func.blocks.push(mir::BasicBlock::new(0));
    module.functions.push(func);
    let facts = polonius::export_polonius_input_with_region_facts(&module);
    assert!(!facts.is_empty(), "expected exported facts to be non-empty");
}

#[test]
fn adapter_delegates_ok_for_trivial_instrs() {
    let mut module = mir::MirModule::new();
    let mut func = mir::MirFunction::new("main");
    let mut block = mir::BasicBlock::new(0);
    block.instrs.push(mir::Instruction::ConstInt {
        dest: "x".to_string(),
        value: 1,
    });
    block.instrs.push(mir::Instruction::Print {
        src: "x".to_string(),
    });
    func.blocks.push(block);
    module.functions.push(func);
    // Ensure adapter uses the mock solver for test determinism.
    std::env::remove_var("OMNI_USE_POLONIUS");
    let res = polonius::check_mir(&module);
    assert!(res.is_ok(), "expected adapter to accept trivial module");
}
