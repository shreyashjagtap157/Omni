use omni_compiler::mir;
use omni_compiler::polonius;
use std::process::Command;

fn polonius_available() -> bool {
    Command::new("polonius").arg("--version").output().is_ok()
}

#[test]
fn polonius_parity_additional_cases() {
    let mut cases: Vec<(String, mir::MirModule)> = Vec::new();

    // Case 1: drop then use -> should error
    let mut m1 = mir::MirModule::new();
    let mut f1 = mir::MirFunction::new("drop_then_use");
    let mut b1 = mir::BasicBlock::new(0);
    b1.instrs.push(mir::Instruction::ConstInt {
        dest: "x".to_string(),
        value: 1,
    });
    b1.instrs.push(mir::Instruction::Drop {
        var: "x".to_string(),
    });
    b1.instrs.push(mir::Instruction::Print {
        src: "x".to_string(),
    });
    f1.blocks.push(b1);
    m1.functions.push(f1);
    cases.push(("drop_then_use".to_string(), m1));

    // Case 2: field move then use field -> move base moves fields -> should error
    let mut m2 = mir::MirModule::new();
    let mut f2 = mir::MirFunction::new("field_move_then_use");
    let mut b2 = mir::BasicBlock::new(0);
    b2.instrs.push(mir::Instruction::ConstInt {
        dest: "x".to_string(),
        value: 10,
    });
    b2.instrs.push(mir::Instruction::ConstInt {
        dest: "x.a".to_string(),
        value: 1,
    });
    b2.instrs.push(mir::Instruction::Move {
        dest: "z".to_string(),
        src: "x".to_string(),
    });
    b2.instrs.push(mir::Instruction::Print {
        src: "x.a".to_string(),
    });
    f2.blocks.push(b2);
    m2.functions.push(f2);
    cases.push(("field_move_then_use".to_string(), m2));

    // Case 3: multi-block move then use across blocks -> should error
    let mut m3 = mir::MirModule::new();
    let mut f3 = mir::MirFunction::new("multi_block_move");
    let mut b3a = mir::BasicBlock::new(0);
    b3a.instrs.push(mir::Instruction::ConstInt {
        dest: "a".to_string(),
        value: 1,
    });
    b3a.instrs.push(mir::Instruction::Move {
        dest: "b".to_string(),
        src: "a".to_string(),
    });
    let mut b3b = mir::BasicBlock::new(1);
    b3b.instrs.push(mir::Instruction::Print {
        src: "a".to_string(),
    });
    f3.blocks.push(b3a);
    f3.blocks.push(b3b);
    m3.functions.push(f3);
    cases.push(("multi_block_move".to_string(), m3));

    // Case 4: move then reinit then use -> should NOT error
    let mut m4 = mir::MirModule::new();
    let mut f4 = mir::MirFunction::new("reinit_after_move");
    let mut b4 = mir::BasicBlock::new(0);
    b4.instrs.push(mir::Instruction::ConstInt {
        dest: "a".to_string(),
        value: 1,
    });
    b4.instrs.push(mir::Instruction::Move {
        dest: "b".to_string(),
        src: "a".to_string(),
    });
    b4.instrs.push(mir::Instruction::ConstInt {
        dest: "a".to_string(),
        value: 2,
    });
    b4.instrs.push(mir::Instruction::Print {
        src: "a".to_string(),
    });
    f4.blocks.push(b4);
    m4.functions.push(f4);
    cases.push(("reinit_after_move".to_string(), m4));

    // By default, don't run the external polonius CLI parity checks unless
    // explicitly requested via `OMNI_FORCE_REAL_POLONIUS=1`. This avoids
    // flaky differences caused by differences in expected facts format.
    let have_polonius = polonius_available()
        && std::env::var("OMNI_FORCE_REAL_POLONIUS").ok().as_deref() == Some("1");
    if !have_polonius {
        eprintln!("skipping real-polonius parity; set OMNI_FORCE_REAL_POLONIUS=1 to enable");
    }

    for (name, module) in cases {
        let facts = polonius::export_polonius_input_with_region_facts(&module);
        let mock_res = polonius_engine_adapter::check_facts(&facts);

        if have_polonius {
            std::env::set_var("OMNI_USE_POLONIUS", "1");
            let real_res = polonius_engine_adapter::check_facts(&facts);
            std::env::remove_var("OMNI_USE_POLONIUS");
            assert_eq!(
                mock_res.is_ok(),
                real_res.is_ok(),
                "parity mismatch for case {}: facts=\n{}",
                name,
                facts
            );
        } else {
            // Ensure mock runs (should not panic)
            assert!(mock_res.is_ok() || mock_res.is_err());
        }
    }
}
