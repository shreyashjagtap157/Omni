use omni_compiler::complete_lexer::CompleteLexer;
use omni_compiler::parser::Parser;

#[test]
fn brace_and_semicolon_surface_is_accepted() {
    let src = r#"
fn add(a: i64, b: i64) -> i64 {
    let c: i64 = a + b;
    return c;
}

fn main() -> i64 {
    return add(40, 2);
}
"#;
    let mut lexer = CompleteLexer::new(src);
    let tokens = lexer.tokenize().expect("lex");
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().expect("parse");
    assert_eq!(program.stmts.len(), 2);
}
