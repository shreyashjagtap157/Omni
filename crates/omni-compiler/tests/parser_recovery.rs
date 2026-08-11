use omni_compiler::parser::Parser;

#[test]
fn recovery_reports_errors() {
    // missing identifier after `let` should be reported, but parser should continue
    let src = "let = 1\nlet x = 2\nprint x\n";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).unwrap();
    let mut parser = Parser::new(tokens);
    let res = parser.parse_program();
    assert!(
        res.is_err(),
        "expected parse_program to return Err for invalid input"
    );
    let err = res.err().unwrap();
    assert!(
        err.message.contains("Expected identifier after 'let'"),
        "error message should mention missing identifier, got: {}",
        err.message
    );
}

#[test]
fn recovery_collects_multiple_errors() {
    let src = "let = 1\nlet = 2\n";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).unwrap();
    let mut parser = Parser::new(tokens);
    let res = parser.parse_program();
    assert!(
        res.is_err(),
        "expected parse_program to return Err for invalid input"
    );
    let err = res.err().unwrap();
    let count = err
        .message
        .matches("Expected identifier after 'let'")
        .count();
    assert!(
        count >= 2,
        "expected at least two recoverable errors, got {}: {}",
        count,
        err.message
    );
}
