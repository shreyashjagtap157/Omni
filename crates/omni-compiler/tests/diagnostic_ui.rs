use omni_compiler::diagnostics::{error_codes, Diagnostic, Severity};
use omni_compiler::lexer::Lexer;
use omni_compiler::parser::Parser;

#[test]
fn test_basic_parsing() {
    let src = "print hello\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("tokenize failed");
    let has_print = tokens.iter().any(|t| t.text == "print");
    assert!(has_print, "expected print keyword");
}

#[test]
fn test_parse_simple_statement() {
    let src = "print 1\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().unwrap();
    assert_eq!(prog.stmts.len(), 1);
}

#[test]
fn test_format_roundtrip() {
    let src = "print 42\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().unwrap();
    let formatted = omni_compiler::formatter::format_program(&prog);

    let mut lexer2 = Lexer::new(&formatted);
    let tokens2 = lexer2.tokenize().unwrap();
    let mut parser2 = Parser::new(tokens2);
    let prog2 = parser2.parse_program().unwrap();
    assert_eq!(prog2.stmts.len(), prog.stmts.len());
}

#[test]
fn test_diagnostic_error_codes() {
    let diag = Diagnostic::error(error_codes::PARSER_UNEXPECTED_TOKEN, "unexpected token");

    assert_eq!(diag.code.code(), "2001");
    assert_eq!(diag.severity, Severity::Error);
}

#[test]
fn test_diagnostic_with_span() {
    use omni_compiler::diagnostics::Span;

    let span = Span::new(1, 0, 1, 5);
    let diag =
        Diagnostic::error(error_codes::PARSER_UNEXPECTED_TOKEN, "unexpected token").with_span(span);

    assert_eq!(diag.spans.len(), 1);
}

#[test]
fn test_diagnostic_with_suggestion() {
    use omni_compiler::diagnostics::{Applicability, Span, Suggestion};

    let span = Span::new(1, 0, 1, 5);
    let suggestion = Suggestion {
        label: "Did you mean 'print'?".to_string(),
        span: span.clone(),
        text: "print".to_string(),
        applicability: Applicability::Always,
    };

    let diag = Diagnostic::error(error_codes::PARSER_UNEXPECTED_TOKEN, "unexpected token")
        .with_suggestion(suggestion);

    assert_eq!(diag.suggestions.len(), 1);
    assert_eq!(diag.suggestions[0].applicability, Applicability::Always);
}

#[test]
fn test_diagnostic_display_format() {
    let diag = Diagnostic::error(error_codes::PARSER_UNEXPECTED_TOKEN, "unexpected token `@`");

    let display = format!("{}", diag);
    assert!(display.contains("error [E2001]"));
    assert!(display.contains("unexpected token"));
}

#[test]
fn test_warning_diagnostic() {
    let diag = Diagnostic::warning(
        error_codes::RESOLVER_SHADOWED_NAME,
        "name 'x' shadows previous binding",
    );

    assert_eq!(diag.severity, Severity::Warning);
    assert_eq!(diag.code.code(), "3003");
}

#[test]
fn test_note_diagnostic() {
    let diag = Diagnostic::error(error_codes::TYPE_MISMATCH, "mismatched types")
        .with_note("expected type 'int', found type 'string'");

    assert_eq!(diag.notes.len(), 1);
    assert!(diag.notes[0].contains("expected"));
}

#[test]
fn test_label_creation() {
    use omni_compiler::diagnostics::{Label, LabelStyle, Span};

    let span = Span::new(1, 0, 1, 5);
    let label = Label::primary(span.clone());
    assert_eq!(label.style, LabelStyle::Primary);

    let label2 = Label::secondary(span).with_message("test");
    assert_eq!(label2.style, LabelStyle::Secondary);
    assert_eq!(label2.message, Some("test".to_string()));
}

#[test]
fn test_severity_levels() {
    let err = Diagnostic::error(error_codes::PARSER_UNEXPECTED_TOKEN, "test");
    assert_eq!(err.severity, Severity::Error);

    let warn = Diagnostic::warning(error_codes::RESOLVER_SHADOWED_NAME, "test");
    assert_eq!(warn.severity, Severity::Warning);
}

#[test]
fn test_error_codes_range() {
    assert_eq!(error_codes::LEXER_UNTERMINATED_STRING.code(), "1001");
    assert_eq!(error_codes::PARSER_UNEXPECTED_TOKEN.code(), "2001");
    assert_eq!(error_codes::RESOLVER_UNDEFINED_NAME.code(), "3001");
    assert_eq!(error_codes::TYPE_MISMATCH.code(), "4001");
    assert_eq!(error_codes::BORROW_USE_AFTER_MOVE.code(), "5001");
    assert_eq!(error_codes::RUNTIME_DIVIDE_BY_ZERO.code(), "6001");
    assert_eq!(error_codes::CODEGEN_UNSUPPORTED_FEATURE.code(), "7001");
}
