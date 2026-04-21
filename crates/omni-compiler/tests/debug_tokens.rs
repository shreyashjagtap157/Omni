use omni_compiler::lexer;
use omni_compiler::parser;

#[test]
fn debug_parsing() {
    let src = "let a = 1\nlet b = a\nprint a";
    let mut lexer = lexer::Lexer::new(src);
    let tokens = lexer.tokenize().unwrap();
    println!("Tokens: {:?}", tokens.len());
    let mut parser = parser::Parser::new(tokens);
    match parser.parse_program() {
        Ok(prog) => {
            println!("AST: {:?}", prog.stmts);
            assert_eq!(prog.stmts.len(), 3, "expected 3 top-level statements");
        }
        Err(e) => {
            panic!("Parse error: {}", e);
        }
    }
    // test passes if parsing succeeded
}
