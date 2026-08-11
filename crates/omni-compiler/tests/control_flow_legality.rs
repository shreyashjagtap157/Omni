use omni_compiler::diagnostics::error_codes;
use omni_compiler::driver::{Backend, Compiler};

#[test]
fn break_outside_loop_is_static_error() {
    let source = "fn main() -> i64 { break; return 0; }";
    let result = Compiler::new(source, Backend::Native).compile();
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == error_codes::TYPE_BREAK_OUTSIDE_LOOP));
    assert!(result.mir.is_none());
}

#[test]
fn continue_outside_loop_is_static_error() {
    let source = "fn main() -> i64 { continue; return 0; }";
    let result = Compiler::new(source, Backend::Native).compile();
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == error_codes::TYPE_CONTINUE_OUTSIDE_LOOP));
    assert!(result.mir.is_none());
}
