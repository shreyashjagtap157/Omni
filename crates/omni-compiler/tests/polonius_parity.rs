use omni_compiler::mir;
use omni_compiler::polonius;
use std::process::Command;

fn polonius_available() -> bool {
    Command::new("polonius").arg("--version").output().is_ok()
}

#[test]
fn polonius_parity_mock_vs_real() {
    let mut module_ok = mir::MirModule::new();
    let mut func_ok = mir::MirFunction::new("main");
    let mut block_ok = mir::BasicBlock::new(0);
    block_ok.instrs.push(mir::Instruction::ConstInt {
        dest: "x".to_string(),
        value: 1,
    });
    block_ok.instrs.push(mir::Instruction::Print {
        src: "x".to_string(),
    });
    func_ok.blocks.push(block_ok);
    module_ok.functions.push(func_ok);

    let mut module_err = mir::MirModule::new();
    let mut func_err = mir::MirFunction::new("main");
    let mut block_err = mir::BasicBlock::new(0);
    block_err.instrs.push(mir::Instruction::ConstInt {
        dest: "a".to_string(),
        value: 1,
    });
    block_err.instrs.push(mir::Instruction::Move {
        dest: "b".to_string(),
        src: "a".to_string(),
    });
    block_err.instrs.push(mir::Instruction::Print {
        src: "a".to_string(),
    });
    func_err.blocks.push(block_err);
    module_err.functions.push(func_err);

    let facts_ok = polonius::export_polonius_input_with_region_facts(&module_ok);
    let facts_err = polonius::export_polonius_input_with_region_facts(&module_err);

    let mock_ok = polonius_engine_adapter::check_facts(&facts_ok);
    let mock_err = polonius_engine_adapter::check_facts(&facts_err);

    assert!(mock_ok.is_ok(), "mock should accept trivial module");
    assert!(mock_err.is_err(), "mock should reject use-after-move");

    let have_polonius = polonius_available()
        && std::env::var("OMNI_FORCE_REAL_POLONIUS").ok().as_deref() == Some("1");
    if !have_polonius {
        eprintln!("skipping real-polonius parity; set OMNI_FORCE_REAL_POLONIUS=1 to enable");
        return;
    }

    std::env::set_var("OMNI_USE_POLONIUS", "1");
    let real_ok = polonius_engine_adapter::check_facts(&facts_ok);
    let real_err = polonius_engine_adapter::check_facts(&facts_err);
    std::env::remove_var("OMNI_USE_POLONIUS");

    assert_eq!(
        mock_ok.is_ok(),
        real_ok.is_ok(),
        "parity mismatch for trivial module"
    );
    assert_eq!(
        mock_err.is_ok(),
        real_err.is_ok(),
        "parity mismatch for use-after-move module"
    );
}
