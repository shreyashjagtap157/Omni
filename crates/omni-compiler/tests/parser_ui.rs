use omni_compiler::lexer::Lexer;
use omni_compiler::parser::Parser;

#[test]
fn lex_basic_tokens() {
    let src = "print hello\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("tokenize failed");
    let has_print = tokens.iter().any(|t| t.text == "print");
    assert!(has_print, "expected print keyword");
}

#[test]
fn parse_single_statement() {
    let src = "print 1\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().unwrap();
    assert_eq!(prog.stmts.len(), 1);
}

#[test]
fn format_roundtrip() {
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
