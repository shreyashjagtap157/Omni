use omni_compiler::parse_file;
use omni_compiler::type_checker::type_check_program;
use std::io::Write;

#[test]
fn linear_type_basic_parse() {
    // Test that linear let parses correctly
    let src = "linear a = 1\n";
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();
    let prog = parse_file(path).expect("parse failed");
    let result = type_check_program(&prog);
    // Should succeed - linear type defined and used once
    assert!(result.is_ok(), "linear type should work");
}

#[test]
fn linear_type_moved_error() {
    // Linear type moved to another variable should error if used again
    let src = "linear a = 1\nlet b = a\nprint a\n";
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();
    let prog = parse_file(path).expect("parse failed");
    let result = type_check_program(&prog);
    assert!(result.is_err(), "expected error for using moved linear value");
}

#[test]
fn linear_type_proper_use() {
    // Linear type moved once should be fine
    let src = "linear a = 1\nlet b = a\nprint b\n";
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();
    let prog = parse_file(path).expect("parse failed");
    let result = type_check_program(&prog);
    assert!(result.is_ok(), "linear type moved once should be valid");
}
