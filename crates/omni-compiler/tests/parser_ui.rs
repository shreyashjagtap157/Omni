use omni_compiler::complete_lexer;
use omni_compiler::parser::Parser;

#[test]
fn lex_basic_tokens() {
    let src = "print hello\n";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let has_print = tokens.iter().any(|t| t.text == "print");
    assert!(has_print, "expected print keyword");
}

#[test]
fn parse_single_statement() {
    let src = "print 1\n";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).unwrap();
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().unwrap();
    assert_eq!(prog.stmts.len(), 1);
}

#[test]
fn format_roundtrip() {
    let src = "print 42\n";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).unwrap();
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().unwrap();
    let formatted = omni_compiler::formatter::format_program(&prog);

    let tokens2 = complete_lexer::tokenize_complete(&formatted).unwrap();
    let mut parser2 = Parser::new(tokens2);
    let prog2 = parser2.parse_program().unwrap();
    assert_eq!(prog2.stmts.len(), prog.stmts.len());
}

#[test]
fn parse_mod_statement() {
    let src = "mod my_module\nmod nested_module\n    print 1\n";
    let tokens = complete_lexer::tokenize_complete(src).unwrap();
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().unwrap();
    if prog.stmts.len() != 2 {
        for (i, stmt) in prog.stmts.iter().enumerate() {
            println!("Stmt {}: {:?}", i, stmt);
        }
    }
    assert_eq!(prog.stmts.len(), 2);
    if let omni_compiler::ast::Stmt::Mod(name, _) = &prog.stmts[0] {
        assert_eq!(name, "my_module");
    } else {
        panic!("expected Mod");
    }

    if let omni_compiler::ast::Stmt::ModBlock(name, body, _) = &prog.stmts[1] {
        assert_eq!(name, "nested_module");
        assert_eq!(body.len(), 1);
    } else {
        panic!("expected ModBlock");
    }
}
