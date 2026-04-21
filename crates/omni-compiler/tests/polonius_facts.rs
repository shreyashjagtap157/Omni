use omni_compiler::mir::{MirModule, MirFunction, BasicBlock, Instruction};
use omni_compiler::polonius;

#[test]
fn basic_defs_and_moves_emit_facts() {
    let mut module = MirModule::new();
    let mut f = MirFunction::new("main");
    let mut b = BasicBlock::new(0);
    b.instrs.push(Instruction::ConstInt { dest: "a".to_string(), value: 1 });
    b.instrs.push(Instruction::Move { dest: "b".to_string(), src: "a".to_string() });
    b.instrs.push(Instruction::Print { src: "a".to_string() });
    f.blocks.push(b);
    module.functions.push(f);

    let facts = polonius::generate_region_loan_facts(&module);
    assert!(facts.iter().any(|s| s == "def main 0 0 a"), "expected def fact for a");
    assert!(facts.iter().any(|s| s == "move main 0 1 a b"), "expected move fact for a->b");
    assert!(facts.iter().any(|s| s == "use main 0 2 a"), "expected use fact for a");
}

#[test]
fn base_and_field_definitions_and_move() {
    let mut module = MirModule::new();
    let mut f = MirFunction::new("main");
    let mut b = BasicBlock::new(0);
    b.instrs.push(Instruction::ConstInt { dest: "x".to_string(), value: 10 });
    b.instrs.push(Instruction::ConstInt { dest: "x.a".to_string(), value: 1 });
    b.instrs.push(Instruction::Move { dest: "z".to_string(), src: "x".to_string() });
    b.instrs.push(Instruction::Print { src: "x.a".to_string() });
    f.blocks.push(b);
    module.functions.push(f);

    let facts = polonius::generate_region_loan_facts(&module);
    assert!(facts.iter().any(|s| s == "def main 0 0 x"), "expected def fact for x");
    assert!(facts.iter().any(|s| s == "def main 0 1 x.a"), "expected def fact for x.a");
    assert!(facts.iter().any(|s| s == "move main 0 2 x z"), "expected move fact for x->z");
}
