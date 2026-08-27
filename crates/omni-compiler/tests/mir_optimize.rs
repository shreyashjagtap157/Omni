use omni_compiler::complete_lexer::TokenKind;
use omni_compiler::mir::{BasicBlock, Instruction, MirFunction, MirModule};
use omni_compiler::mir_optimize;

fn single_block_function(name: &str, instrs: Vec<Instruction>) -> MirFunction {
    let mut function = MirFunction::new(name, false);
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

#[test]
fn constant_folding_preserves_division_faults() {
    let mut module = MirModule::new();
    module.functions.push(single_block_function(
        "main",
        vec![
            Instruction::ConstInt {
                dest: "a".into(),
                value: 7,
            },
            Instruction::ConstInt {
                dest: "b".into(),
                value: 0,
            },
            Instruction::BinaryOp {
                dest: "c".into(),
                op: TokenKind::Slash,
                left: "a".into(),
                right: "b".into(),
            },
            Instruction::Return { value: "c".into() },
        ],
    ));

    mir_optimize::constant_fold_module(&mut module);
    assert!(module.functions[0].blocks[0]
        .instrs
        .iter()
        .any(|instr| matches!(instr, Instruction::BinaryOp { dest, op: TokenKind::Slash, .. } if dest == "c")));
}

#[test]
fn constant_folding_preserves_integer_overflow_faults() {
    let mut module = MirModule::new();
    module.functions.push(single_block_function(
        "main",
        vec![
            Instruction::ConstInt {
                dest: "a".into(),
                value: i64::MAX,
            },
            Instruction::ConstInt {
                dest: "b".into(),
                value: 1,
            },
            Instruction::BinaryOp {
                dest: "c".into(),
                op: TokenKind::Plus,
                left: "a".into(),
                right: "b".into(),
            },
            Instruction::Return { value: "c".into() },
        ],
    ));

    mir_optimize::constant_fold_module(&mut module);
    assert!(module.functions[0].blocks[0]
        .instrs
        .iter()
        .any(|instr| matches!(instr, Instruction::BinaryOp { dest, op: TokenKind::Plus, .. } if dest == "c")));
}

#[test]
fn dce_preserves_unused_calls_because_calls_can_have_effects() {
    let mut module = MirModule::new();
    module.functions.push(single_block_function(
        "main",
        vec![
            Instruction::Call {
                dest: "unused".into(),
                func: "side_effect".into(),
                args: vec![],
            },
            Instruction::ConstInt {
                dest: "answer".into(),
                value: 0,
            },
            Instruction::Return {
                value: "answer".into(),
            },
        ],
    ));

    mir_optimize::dce_module(&mut module);
    assert!(module.functions[0].blocks[0]
        .instrs
        .iter()
        .any(|instr| matches!(instr, Instruction::Call { func, .. } if func == "side_effect")));
}

#[test]
fn constant_facts_do_not_cross_calls_that_can_mutate_through_references() {
    let mut module = MirModule::new();
    module.functions.push(single_block_function(
        "main",
        vec![
            Instruction::ConstInt {
                dest: "value".into(),
                value: 1,
            },
            Instruction::Borrow {
                dest: "reference".into(),
                place: "value".into(),
                mutable: true,
            },
            Instruction::Call {
                dest: "unit".into(),
                func: "mutate".into(),
                args: vec!["reference".into()],
            },
            Instruction::ConstInt {
                dest: "one".into(),
                value: 1,
            },
            Instruction::BinaryOp {
                dest: "observed".into(),
                op: TokenKind::Plus,
                left: "value".into(),
                right: "one".into(),
            },
            Instruction::Return {
                value: "observed".into(),
            },
        ],
    ));

    mir_optimize::constant_fold_module(&mut module);

    let instrs = &module.functions[0].blocks[0].instrs;
    let borrow = instrs
        .iter()
        .position(|instr| matches!(instr, Instruction::Borrow { .. }))
        .expect("mutable borrow must remain observable in MIR");
    let call = instrs
        .iter()
        .position(
            |instr| matches!(instr, Instruction::Call { func, .. } if func == "mutate"),
        )
        .expect("effectful call must remain observable in MIR");
    let observation = instrs
        .iter()
        .position(
            |instr| matches!(instr, Instruction::BinaryOp { dest, .. } if dest == "observed"),
        )
        .expect("post-call read must not fold from a stale constant");
    assert!(borrow < call && call < observation);
}

#[test]
fn constant_facts_do_not_cross_indirect_or_subobject_writes() {
    let mut indirect = BasicBlock::new(0);
    indirect.instrs = vec![
        Instruction::ConstInt {
            dest: "value".into(),
            value: 20,
        },
        Instruction::ConstInt {
            dest: "replacement".into(),
            value: 21,
        },
        Instruction::DerefAssign {
            reference: "reference".into(),
            src: "replacement".into(),
        },
        Instruction::BinaryOp {
            dest: "observed".into(),
            op: TokenKind::Plus,
            left: "value".into(),
            right: "replacement".into(),
        },
    ];
    mir_optimize::constant_fold_block(&mut indirect);
    assert!(matches!(
        indirect.instrs.last(),
        Some(Instruction::BinaryOp { dest, .. }) if dest == "observed"
    ));

    let mut subobject = BasicBlock::new(0);
    subobject.instrs = vec![
        Instruction::ConstInt {
            dest: "value".into(),
            value: 20,
        },
        Instruction::ConstInt {
            dest: "replacement".into(),
            value: 21,
        },
        Instruction::FieldAssign {
            base: "aggregate".into(),
            field: "field".into(),
            src: "replacement".into(),
        },
        Instruction::BinaryOp {
            dest: "observed".into(),
            op: TokenKind::Plus,
            left: "value".into(),
            right: "replacement".into(),
        },
    ];
    mir_optimize::constant_fold_block(&mut subobject);
    assert!(matches!(
        subobject.instrs.last(),
        Some(Instruction::BinaryOp { dest, .. }) if dest == "observed"
    ));
}

#[test]
fn constant_facts_do_not_cross_spawn() {
    let mut block = BasicBlock::new(0);
    block.instrs = vec![
        Instruction::ConstInt {
            dest: "value".into(),
            value: 20,
        },
        Instruction::Spawn {
            func: "observer".into(),
            args: vec!["reference".into()],
        },
        Instruction::BinaryOp {
            dest: "observed".into(),
            op: TokenKind::Plus,
            left: "value".into(),
            right: "value".into(),
        },
    ];

    mir_optimize::constant_fold_block(&mut block);
    assert!(matches!(
        block.instrs.last(),
        Some(Instruction::BinaryOp { dest, .. }) if dest == "observed"
    ));
}

#[test]
fn inliner_does_not_erase_side_effects_from_constant_returning_function() {
    let mut module = MirModule::new();
    module.functions.push(single_block_function(
        "helper",
        vec![
            Instruction::ConstInt {
                dest: "printed".into(),
                value: 1,
            },
            Instruction::Print {
                src: "printed".into(),
            },
            Instruction::ConstInt {
                dest: "value".into(),
                value: 7,
            },
            Instruction::Return {
                value: "value".into(),
            },
        ],
    ));
    module.functions.push(single_block_function(
        "main",
        vec![
            Instruction::Call {
                dest: "answer".into(),
                func: "helper".into(),
                args: vec![],
            },
            Instruction::Return {
                value: "answer".into(),
            },
        ],
    ));

    mir_optimize::inline_simple_functions(&mut module);
    assert!(module.functions[1].blocks[0]
        .instrs
        .iter()
        .any(|instr| matches!(instr, Instruction::Call { func, .. } if func == "helper")));
}
