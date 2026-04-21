use omni_compiler::lexer::TokenKind;
use omni_compiler::mir::{BasicBlock, Instruction, MirFunction, MirModule};
use omni_compiler::mir_optimize;

fn single_block_function(name: &str, instrs: Vec<Instruction>) -> MirFunction {
    let mut function = MirFunction::new(name);
    let mut block = BasicBlock::new(0);
    block.instrs = instrs;
    function.blocks.push(block);
    function
}

#[test]
fn constant_folding_rewrites_binary_ops() {
    let mut module = MirModule::new();
    module.functions.push(single_block_function(
        "main",
        vec![
            Instruction::ConstInt {
                dest: "a".to_string(),
                value: 40,
            },
            Instruction::ConstInt {
                dest: "b".to_string(),
                value: 2,
            },
            Instruction::BinaryOp {
                dest: "c".to_string(),
                op: TokenKind::Plus,
                left: "a".to_string(),
                right: "b".to_string(),
            },
            Instruction::Return {
                value: "c".to_string(),
            },
        ],
    ));

    mir_optimize::run_mir_optimizations(&mut module);

    let block = &module.functions[0].blocks[0];
    assert!(block
        .instrs
        .iter()
        .any(|instr| matches!(instr, Instruction::ConstInt { dest, value } if dest == "c" && *value == 42)));
    assert!(!block
        .instrs
        .iter()
        .any(|instr| matches!(instr, Instruction::BinaryOp { dest, .. } if dest == "c")));
}

#[test]
fn dead_code_elimination_drops_unused_defs() {
    let mut module = MirModule::new();
    module.functions.push(single_block_function(
        "main",
        vec![
            Instruction::ConstInt {
                dest: "dead".to_string(),
                value: 1,
            },
            Instruction::ConstInt {
                dest: "live".to_string(),
                value: 2,
            },
            Instruction::Return {
                value: "live".to_string(),
            },
        ],
    ));

    mir_optimize::run_mir_optimizations(&mut module);

    let block = &module.functions[0].blocks[0];
    assert!(!block
        .instrs
        .iter()
        .any(|instr| matches!(instr, Instruction::ConstInt { dest, .. } if dest == "dead")));
    assert!(block
        .instrs
        .iter()
        .any(|instr| matches!(instr, Instruction::ConstInt { dest, value } if dest == "live" && *value == 2)));
}

#[test]
fn simple_constant_functions_inline_across_calls() {
    let mut module = MirModule::new();
    module.functions.push(single_block_function(
        "helper",
        vec![
            Instruction::ConstInt {
                dest: "value".to_string(),
                value: 7,
            },
            Instruction::Return {
                value: "value".to_string(),
            },
        ],
    ));
    module.functions.push(single_block_function(
        "main",
        vec![
            Instruction::Call {
                dest: "answer".to_string(),
                func: "helper".to_string(),
                args: vec![],
            },
            Instruction::Return {
                value: "answer".to_string(),
            },
        ],
    ));

    mir_optimize::run_mir_optimizations(&mut module);

    let main_block = &module.functions[1].blocks[0];
    assert!(main_block
        .instrs
        .iter()
        .any(|instr| matches!(instr, Instruction::ConstInt { dest, value } if dest == "answer" && *value == 7)));
    assert!(!main_block
        .instrs
        .iter()
        .any(|instr| matches!(instr, Instruction::Call { dest, func, .. } if dest == "answer" && func == "helper")));
}