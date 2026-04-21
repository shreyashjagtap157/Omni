use omni_compiler::mir;
use omni_compiler::codegen_lir;

#[test]
fn lower_simple_mir_to_lir_and_compile() {
    let mut module = mir::MirModule::new();
    let mut func = mir::MirFunction::new("main");

    let mut block0 = mir::BasicBlock::new(0);
    block0.instrs.push(mir::Instruction::ConstInt { dest: "a".to_string(), value: 40 });
    block0.instrs.push(mir::Instruction::ConstInt { dest: "b".to_string(), value: 2 });
    block0.instrs.push(mir::Instruction::BinaryOp { dest: "c".to_string(), op: omni_compiler::lexer::TokenKind::Plus, left: "a".to_string(), right: "b".to_string() });
    block0.instrs.push(mir::Instruction::Return { value: "c".to_string() });

    func.blocks.push(block0);
    module.functions.push(func);

    let lir = codegen_lir::lower_mir_to_lir(&module);
    let out = codegen_lir::compile_lir_module_text(&lir);
    assert!(out.contains("fn main"), "expected compiled output to include function header");
    assert!(out.contains("Add") || out.contains("Add"), "expected an Add instruction in output");
}
