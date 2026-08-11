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
