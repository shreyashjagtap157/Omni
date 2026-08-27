use omni_compiler::{emit_lir_file, parse_file};
use std::io::Write;

#[test]
fn check_mir_file_reports_move_error() {
    let src = "let a = 1\nlet b = a\nprint a\n";
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let _path = tmp.path();
    let res =
        omni_compiler::driver::Compiler::new(src, omni_compiler::driver::Backend::Native).compile();
    // Polonius borrow check is feature-gated; when disabled, no move errors expected
    #[cfg(feature = "use_polonius")]
    assert!(
        !res.diagnostics.is_empty(),
        "expected compilation to report use-after-move"
    );
    #[cfg(not(feature = "use_polonius"))]
    assert!(res.program.is_some());
}

#[test]
fn run_native_hello_world() {
    let src = "let a = 1\nlet b = 2\nlet c = a + b\nprint c\n";
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let _path = tmp.path();
    omni_compiler::driver::Compiler::new(src, omni_compiler::driver::Backend::Native).compile();
}

#[test]
fn run_native_with_printed_string() {
    let src = "let s = \"hello\"\nprint s\n";
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let _path = tmp.path();
    omni_compiler::driver::Compiler::new(src, omni_compiler::driver::Backend::Native).compile();
}

#[test]
fn step1_7_pipeline_smoke() {
    let src = "let a = 1\n";
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();

    // Step 1: Parse
    let _program = parse_file(tmp.path()).expect("parse failed");

    // Step 2-7: Full pipeline via Compiler
    let result =
        omni_compiler::driver::Compiler::new(src, omni_compiler::driver::Backend::Native).compile();
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:?}",
        result.diagnostics
    );
    assert!(result.program.is_some());

    // Step 5: Emit LIR / codegen intermediate
    let lir_text = emit_lir_file(path).expect("emit LIR failed");
    assert!(!lir_text.is_empty(), "LIR output should not be empty");
}

#[test]
fn run_native_hello_example_file() {
    let src = "print \"Hello, Omni!\"";
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let _path = tmp.path();
    omni_compiler::driver::Compiler::new(src, omni_compiler::driver::Backend::Native).compile();
}

#[test]
fn test_compiler_pipeline_success() {
    let source = "fn compute() -> int:\n    return 42\nfn run():\n    compute()\n";
    let compiler =
        omni_compiler::driver::Compiler::new(source, omni_compiler::driver::Backend::Native);
    let result = compiler.compile();

    assert!(
        result.diagnostics.is_empty(),
        "Expected no diagnostics, got: {:?}",
        result.diagnostics
    );
    assert!(result.program.is_some());
    assert!(result.resolve_result.is_some());
}

#[test]
fn test_compiler_pipeline_parser_error() {
    // An unterminated brace-delimited body is malformed source and must fail closed.
    let source = "fn main() {";
    let compiler =
        omni_compiler::driver::Compiler::new(source, omni_compiler::driver::Backend::Native);
    let result = compiler.compile();

    assert!(result.program.is_none());
    assert!(result.diagnostics.iter().any(|diagnostic| matches!(
        diagnostic.severity,
        omni_compiler::diagnostics::Severity::Error
    )));
}

#[test]
fn test_compiler_pipeline_resolver_error() {
    let source = "fn main() -> int:\n    return x";
    let compiler =
        omni_compiler::driver::Compiler::new(source, omni_compiler::driver::Backend::Native);
    let result = compiler.compile();

    assert!(!result.diagnostics.is_empty());
    assert!(result.resolve_result.is_none());
}

#[test]
fn test_compiler_pipeline_type_error() {
    let source = "fn main() -> int:\n    return \"hello\"";
    let compiler =
        omni_compiler::driver::Compiler::new(source, omni_compiler::driver::Backend::Native);
    let result = compiler.compile();

    assert!(!result.diagnostics.is_empty());
}


#[test]
fn lower_mir_preserves_source_order_for_nested_call_arguments() {
    let source = r#"
fn left():
    print 1
    return 10

fn right():
    print 2
    return 20

fn combine(a, b):
    a + b

fn main():
    print combine(left(), right())
"#;

    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", source).unwrap();
    let path = tmp.path();
    let program = parse_file(path).expect("parse failed");
    let mir = omni_compiler::mir::lower_program_to_mir(&program);
    let main = mir
        .functions
        .iter()
        .find(|func| func.name == "main")
        .expect("expected main function");
    let rendered = omni_compiler::mir::format_mir(&mir);
    let main_text_start = rendered
        .find("fn main")
        .expect("expected rendered main function");
    let main_text = &rendered[main_text_start..];
    let left_pos = main_text.find("call left").expect("expected left() call in MIR");
    let right_pos = main_text.find("call right").expect("expected right() call in MIR");
    let combine_pos = main_text
        .find("call combine")
        .expect("expected combine() call in MIR");
    assert!(left_pos < right_pos, "left() must be lowered before right():\n{main_text}");
    assert!(right_pos < combine_pos, "argument calls must precede the enclosing call:\n{main_text}");
    assert!(!main.blocks.is_empty(), "expected MIR blocks for main");
}

#[cfg(feature = "wasm-backend")]
#[test]
fn test_compiler_pipeline_wasm() {
    let source = "fn compute() -> int:\n    return 42\n";
    let compiler =
        omni_compiler::driver::Compiler::new(source, omni_compiler::driver::Backend::Wasm);
    let result = compiler.compile();

    assert!(
        result.diagnostics.is_empty(),
        "Expected no diagnostics, got: {:?}",
        result.diagnostics
    );
    assert!(result.program.is_some());
    assert!(result.wasm_output.is_some());
    let wasm = result.wasm_output.unwrap();
    assert!(!wasm.is_empty());
}

#[cfg(not(feature = "wasm-backend"))]
#[test]
fn test_compiler_pipeline_wasm_fails_closed_when_unqualified() {
    let source = "fn compute() -> int:\n    return 42\n";
    let compiler =
        omni_compiler::driver::Compiler::new(source, omni_compiler::driver::Backend::Wasm);
    let result = compiler.compile();

    assert!(result.wasm_output.is_none());
    assert!(result.diagnostics.iter().any(|diagnostic| {
        diagnostic.code.0 == "7001"
            && matches!(
                diagnostic.severity,
                omni_compiler::diagnostics::Severity::Error
            )
    }));
}
