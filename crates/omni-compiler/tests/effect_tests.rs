use omni_compiler::complete_lexer::tokenize_complete;
use omni_compiler::driver::{Backend, Compiler};
use omni_compiler::interpreter;
use omni_compiler::parser::Parser;

fn run_interpreter(source: &str) -> Result<(), String> {
    let tokens = tokenize_complete(source).map_err(|e| format!("Lexer failed: {}", e))?;
    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .map_err(|e| format!("Parser failed: {}", e))?;
    interpreter::run_program(&program)
}

#[test]
fn test_basic_effect_propagation() {
    let source = r#"
io fn foo():
    bar()

io fn bar():
    print "hello"
"#;
    let compiler = Compiler::new(source, Backend::Native);
    let result = compiler.compile();
    assert!(result.program.is_some());
    assert!(result.effect_resolver.is_some());
}

#[test]
fn test_unhandled_effect() {
    let source = r#"
pure fn main():
    io_func()

io fn io_func():
    print "hello"
"#;
    let compiler = Compiler::new(source, Backend::Native);
    let result = compiler.compile();
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("effects not included in declared")
                || d.message.contains("unhandled effects")),
        "expected effect mismatch diagnostic, got: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_pure_main_passes() {
    let source = r#"
pure fn main():
    print "hello"
"#;
    let compiler = Compiler::new(source, Backend::Native);
    let result = compiler.compile();
    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| !d.message.contains("unhandled effects")),
        "expected no unhandled effects diagnostic, got: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_interpreter_basic_program() {
    let result = run_interpreter("print 42\n");
    assert!(result.is_ok(), "interpreter failed: {:?}", result);
}

#[test]
fn test_interpreter_variables_and_arithmetic() {
    let result = run_interpreter("let x = 10\nlet y = 20\nprint x + y\n");
    assert!(result.is_ok(), "interpreter failed: {:?}", result);
}

#[test]
fn test_interpreter_conditionals() {
    let source =
        "fn main():\n    let x = 15\n    if x > 10\n        print 1\n    else\n        print 0\n";
    let result = run_interpreter(source);
    assert!(result.is_ok(), "interpreter failed: {:?}", result);
}

#[test]
fn test_interpreter_function_call() {
    let source = "fn add(a, b):\n    a + b\n\nfn main():\n    print add(3, 4)\n";
    let result = run_interpreter(source);
    assert!(result.is_ok(), "interpreter failed: {:?}", result);
}

#[test]
fn test_interpreter_string_ops() {
    let source = "fn main():\n    let s = string_concat(\"hello\", \" world\")\n    print s\n";
    let result = run_interpreter(source);
    assert!(result.is_ok(), "interpreter failed: {:?}", result);
}

#[test]
fn test_interpreter_vector_ops() {
    let source =
        "fn main():\n    let v = vector_new()\n    let n = vector_push(v, 42)\n    print n\n";
    let result = run_interpreter(source);
    assert!(result.is_ok(), "interpreter failed: {:?}", result);
}

#[test]
fn test_interpreter_match_expr() {
    // match arm bodies are expressions (not statements)
    let source = "fn main():\n    let x = 1\n    let r = match x\n        1 => 10\n        _ => 0\n    print r\n";
    let result = run_interpreter(source);
    assert!(result.is_ok(), "interpreter failed: {:?}", result);
}

#[test]
fn test_interpreter_defer_cleanup_parses_and_runs() {
    let source = r#"
fn main():
    let x = 1
    defer
        print x
    print 2
"#;
    let result = run_interpreter(source);
    assert!(result.is_ok(), "interpreter failed: {:?}", result);
}
#[test]
fn test_pipeline_full_compile() {
    let source = "fn main():\n    print 42\n";
    let compiler = Compiler::new(source, Backend::Native);
    let result = compiler.compile();
    assert!(result.program.is_some(), "No program produced");
    assert!(result.resolve_result.is_some(), "Resolution failed");
    assert!(result.type_map.is_some(), "Type checking failed");
    let has_errors = result
        .diagnostics
        .iter()
        .any(|d| d.severity == omni_compiler::diagnostics::Severity::Error);
    assert!(!has_errors, "Compilation errors: {:?}", result.diagnostics);
}

#[test]
fn test_interpreter_for_loop() {
    // simple for loop with single-statement body
    let source = "fn main():\n    for i in 3\n        print i\n";
    let result = run_interpreter(source);
    assert!(result.is_ok(), "interpreter failed: {:?}", result);
}

#[test]
fn test_interpreter_unsafe_block() {
    // unsafe with no colon
    let source = "fn main():\n    unsafe\n        let x = 42\n        print x\n";
    let result = run_interpreter(source);
    assert!(result.is_ok(), "interpreter failed: {:?}", result);
}
