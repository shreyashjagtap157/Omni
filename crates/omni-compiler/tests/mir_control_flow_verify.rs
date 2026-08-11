use omni_compiler::mir::{validate_control_flow, BasicBlock, Instruction, MirFunction, MirModule};

fn module_with(instrs: Vec<Instruction>) -> MirModule {
    let mut module = MirModule::new();
    let mut function = MirFunction::new("main", false);
    let mut block = BasicBlock::new(0);
    block.instrs = instrs;
    function.blocks.push(block);
    module.functions.push(function);
    module
}

#[test]
fn rejects_missing_jump_target() {
    let module = module_with(vec![Instruction::Jump { target: 99 }]);
    let err = validate_control_flow(&module).expect_err("missing label must fail");
    assert!(
        err.contains("missing MIR label 99"),
        "unexpected error: {err}"
    );
}

#[test]
fn rejects_duplicate_labels() {
    let module = module_with(vec![
        Instruction::Label { id: 7 },
        Instruction::Label { id: 7 },
    ]);
    let err = validate_control_flow(&module).expect_err("duplicate label must fail");
    assert!(
        err.contains("duplicate MIR label 7"),
        "unexpected error: {err}"
    );
}

#[test]
fn accepts_valid_conditional_targets() {
    let module = module_with(vec![
        Instruction::ConstBool {
            dest: "c".into(),
            value: true,
        },
        Instruction::JumpIf {
            cond: "c".into(),
            target: 1,
        },
        Instruction::Jump { target: 2 },
        Instruction::Label { id: 1 },
        Instruction::Jump { target: 2 },
        Instruction::Label { id: 2 },
    ]);
    validate_control_flow(&module).expect("valid control flow");
}
