//! Step 1-7 integration tests: parse -> typecheck -> MIR -> borrow check -> LIR -> native codegen -> runtime validation.

use omni_compiler::{parse_file, check_file, emit_mir_file, emit_lir_file, run_native_file, check_mir_file};
use std::io::Write;

#[test]
fn check_mir_file_reports_move_error() {
    let src = "let a = 1\nlet b = a\nprint a\n";
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();
    let res = check_mir_file(path);
    assert!(res.is_err(), "expected check_mir_file to report use-after-move");
}

#[test]
fn run_native_hello_world() {
    let src = "let a = 1\nlet b = 2\nlet c = a + b\nprint c\n";
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();
    run_native_file(path).expect("expected native run to succeed");
}

#[test]
fn run_native_with_printed_string() {
    let src = "let s = \"hello\"\nprint s\n";
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();
    run_native_file(path).expect("expected native run to succeed for string output");
}

#[test]
fn step1_7_pipeline_smoke() {
    let src = "let a = 1\n";
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();

    // Step 1: Parse
    let _program = parse_file(path).expect("parse failed");

    // Step 2: Type-check
    check_file(path).expect("type check failed");

    // Step 3: Emit MIR
    let mir_text = emit_mir_file(path).expect("emit MIR failed");
    assert!(!mir_text.is_empty(), "MIR output should not be empty");

    // Step 4: MIR borrow check (Polonius integration)
    check_mir_file(path).expect("MIR borrow check failed");

    // Step 5: Emit LIR / codegen intermediate
    let lir_text = emit_lir_file(path).expect("emit LIR failed");
    assert!(!lir_text.is_empty(), "LIR output should not be empty");

    // Step 6: Run native compiled output
    run_native_file(path).expect("native run failed");

    // Step 7: Full pipeline semantics are validated by successful native execution.
    // This confirms the end-to-end compile/run path completes without crashing.
}
