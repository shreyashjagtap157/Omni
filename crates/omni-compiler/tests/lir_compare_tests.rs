use omni_compiler::codegen_lir;
use omni_compiler::complete_lexer::TokenKind;
use omni_compiler::mir;

fn build_module_with_binary_ops(ops: &[(TokenKind, &str)]) -> mir::MirModule {
    let mut module = mir::MirModule::new();
    let mut func = mir::MirFunction::new("main", false);
    let mut block0 = mir::BasicBlock::new(0);
    block0.instrs.push(mir::Instruction::ConstInt {
        dest: "a".to_string(),
        value: 5,
    });
    block0.instrs.push(mir::Instruction::ConstInt {
        dest: "b".to_string(),
        value: 10,
    });
    for (op, dest) in ops {
        block0.instrs.push(mir::Instruction::BinaryOp {
            dest: (*dest).to_string(),
            op: op.clone(),
            left: "a".to_string(),
            right: "b".to_string(),
        });
    }
    block0.instrs.push(mir::Instruction::Return {
        value: "r1".to_string(),
    });
    func.blocks.push(block0);
    module.functions.push(func);
    module
}

#[test]
fn lower_lt_to_lir_emits_lt_text() {
    let module = build_module_with_binary_ops(&[(TokenKind::Lt, "r1")]);
    let lir = codegen_lir::lower_mir_to_lir(&module).expect("LIR lowering");
    let out = codegen_lir::compile_lir_module_text(&lir);
    assert!(
        out.contains("fn main"),
        "expected compiled output to include main header, got: {}",
        out
    );
    assert!(
        out.contains("Lt"),
        "expected Lt instruction in LIR text, got: {}",
        out
    );
}

#[test]
fn lower_all_comparison_ops_to_lir() {
    let ops = vec![
        (TokenKind::Lt, "r1"),
        (TokenKind::Gt, "r2"),
        (TokenKind::LtEq, "r3"),
        (TokenKind::GtEq, "r4"),
        (TokenKind::EqEq, "r5"),
        (TokenKind::NotEq, "r6"),
        (TokenKind::Percent, "r7"),
    ];
    let module = build_module_with_binary_ops(&ops);
    let lir = codegen_lir::lower_mir_to_lir(&module).expect("LIR lowering");
    let out = codegen_lir::compile_lir_module_text(&lir);
    for expected in &["Lt", "Gt", "Le", "Ge", "Eq", "Ne", "Mod"] {
        assert!(
            out.contains(expected),
            "expected {} in LIR text, got: {}",
            expected,
            out
        );
    }
}

#[test]
fn lower_eqeq_emits_eq() {
    let module = build_module_with_binary_ops(&[(TokenKind::EqEq, "r1")]);
    let lir = codegen_lir::lower_mir_to_lir(&module).expect("LIR lowering");
    let out = codegen_lir::compile_lir_module_text(&lir);
    assert!(
        out.contains("Eq"),
        "expected Eq in LIR text for ==, got: {}",
        out
    );
    assert!(
        !out.contains("Nop"),
        "should not emit Nop for comparison, got: {}",
        out
    );
}

#[test]
fn lower_percent_emits_mod() {
    let module = build_module_with_binary_ops(&[(TokenKind::Percent, "r1")]);
    let lir = codegen_lir::lower_mir_to_lir(&module).expect("LIR lowering");
    let out = codegen_lir::compile_lir_module_text(&lir);
    assert!(
        out.contains("Mod"),
        "expected Mod in LIR text for %, got: {}",
        out
    );
    assert!(
        !out.contains("Nop"),
        "should not emit Nop for modulo, got: {}",
        out
    );
}

#[test]
fn aggregate_struct_access_lowers_to_checked_offsets() {
    let mut module = mir::MirModule::new();
    let mut func = mir::MirFunction::new("main", false);
    let mut block0 = mir::BasicBlock::new(0);
    block0.instrs.push(mir::Instruction::StructDef {
        name: "Point".to_string(),
        fields: vec![
            ("x".to_string(), "int".to_string()),
            ("y".to_string(), "int".to_string()),
        ],
        is_linear: false,
    });
    block0.instrs.push(mir::Instruction::ConstInt {
        dest: "xv".to_string(),
        value: 42,
    });
    block0.instrs.push(mir::Instruction::ConstInt {
        dest: "yv".to_string(),
        value: 7,
    });
    block0.instrs.push(mir::Instruction::AggregateInit {
        dest: "p".to_string(),
        type_name: "Point".to_string(),
        fields: vec![
            ("y".to_string(), "yv".to_string()),
            ("x".to_string(), "xv".to_string()),
        ],
    });
    block0.instrs.push(mir::Instruction::FieldAccess {
        dest: "px".to_string(),
        base: "p".to_string(),
        field: "x".to_string(),
        linear: false,
    });
    block0.instrs.push(mir::Instruction::Return {
        value: "px".to_string(),
    });
    func.blocks.push(block0);
    module.functions.push(func);

    let lir = codegen_lir::lower_mir_to_lir(&module).expect("aggregate LIR lowering");
    let out = codegen_lir::compile_lir_module_text(&lir);
    assert!(
        out.contains("StoreOffset"),
        "expected aggregate stores: {out}"
    );
    assert!(
        out.contains("LoadOffset"),
        "expected aggregate field load: {out}"
    );
}

#[test]
fn tuple_constant_index_lowers_with_bounds_check_at_compile_time() {
    let mut module = mir::MirModule::new();
    let mut func = mir::MirFunction::new("main", false);
    let mut block0 = mir::BasicBlock::new(0);
    block0.instrs.push(mir::Instruction::ConstInt {
        dest: "a".to_string(),
        value: 7,
    });
    block0.instrs.push(mir::Instruction::ConstInt {
        dest: "b".to_string(),
        value: 42,
    });
    block0.instrs.push(mir::Instruction::AggregateInit {
        dest: "pair".to_string(),
        type_name: "Tuple".to_string(),
        fields: vec![
            ("0".to_string(), "a".to_string()),
            ("1".to_string(), "b".to_string()),
        ],
    });
    block0.instrs.push(mir::Instruction::ConstInt {
        dest: "i".to_string(),
        value: 1,
    });
    block0.instrs.push(mir::Instruction::IndexAccess {
        dest: "elem".to_string(),
        base: "pair".to_string(),
        index: "i".to_string(),
    });
    block0.instrs.push(mir::Instruction::Return {
        value: "elem".to_string(),
    });
    func.blocks.push(block0);
    module.functions.push(func);

    let lir = codegen_lir::lower_mir_to_lir(&module).expect("tuple LIR lowering");
    let out = codegen_lir::compile_lir_module_text(&lir);
    assert!(
        out.contains("LoadOffset"),
        "expected tuple indexed load: {out}"
    );
}

#[test]
fn tuple_constant_index_out_of_bounds_fails_closed() {
    let mut module = mir::MirModule::new();
    let mut func = mir::MirFunction::new("main", false);
    let mut block0 = mir::BasicBlock::new(0);
    block0.instrs.push(mir::Instruction::ConstInt {
        dest: "a".into(),
        value: 1,
    });
    block0.instrs.push(mir::Instruction::AggregateInit {
        dest: "pair".into(),
        type_name: "Tuple".into(),
        fields: vec![("0".into(), "a".into())],
    });
    block0.instrs.push(mir::Instruction::ConstInt {
        dest: "i".into(),
        value: 1,
    });
    block0.instrs.push(mir::Instruction::IndexAccess {
        dest: "elem".into(),
        base: "pair".into(),
        index: "i".into(),
    });
    block0.instrs.push(mir::Instruction::Return {
        value: "elem".into(),
    });
    func.blocks.push(block0);
    module.functions.push(func);
    let err =
        codegen_lir::lower_mir_to_lir(&module).expect_err("out-of-bounds tuple index must fail");
    assert!(err.contains("out of bounds"), "unexpected error: {err}");
}

#[test]
fn lower_linear_move_emits_load() {
    let mut module = mir::MirModule::new();
    let mut func = mir::MirFunction::new("main", false);
    let mut block0 = mir::BasicBlock::new(0);
    block0.instrs.push(mir::Instruction::ConstInt {
        dest: "src".to_string(),
        value: 42,
    });
    block0.instrs.push(mir::Instruction::LinearMove {
        dest: "dst".to_string(),
        src: "src".to_string(),
    });
    block0.instrs.push(mir::Instruction::Return {
        value: "dst".to_string(),
    });
    func.blocks.push(block0);
    module.functions.push(func);

    let lir = codegen_lir::lower_mir_to_lir(&module).expect("LIR lowering");
    let out = codegen_lir::compile_lir_module_text(&lir);
    assert!(
        !out.contains("Nop"),
        "LinearMove should not be a Nop, got: {}",
        out
    );
    assert!(out.contains("Load"), "expected Load, got: {}", out);
}

#[test]
fn lower_const_str_emits_runtime_string_descriptor_print() {
    let mut module = mir::MirModule::new();
    let mut func = mir::MirFunction::new("main", false);
    let mut block0 = mir::BasicBlock::new(0);
    block0.instrs.push(mir::Instruction::ConstStr {
        dest: "s".to_string(),
        value: "hello".to_string(),
    });
    block0.instrs.push(mir::Instruction::Print {
        src: "s".to_string(),
    });
    block0.instrs.push(mir::Instruction::Return {
        value: "0".to_string(),
    });
    func.blocks.push(block0);
    module.functions.push(func);

    let lir = codegen_lir::lower_mir_to_lir(&module).expect("LIR lowering");
    let out = codegen_lir::compile_lir_module_text(&lir);
    assert!(
        out.contains("StringRef") && out.contains("PrintBytes"),
        "expected String descriptor + byte-counted print, got: {}",
        out
    );
    assert!(
        out.contains("hello"),
        "expected literal 'hello' in LIR, got: {}",
        out
    );
}
