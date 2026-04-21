use omni_compiler::lexer::Lexer;
use omni_compiler::mir::lower_program_to_mir;
use omni_compiler::parser::Parser;
use omni_compiler::polonius;
use omni_compiler::resolver;
use omni_compiler::type_checker;

#[test]
fn test_simple_move() {
    let src = "let x = 1\nprint x\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");

    let mir = lower_program_to_mir(&prog);
    let result = polonius::check_mir(&mir);
    assert!(result.is_ok(), "Simple move should pass");
}

#[test]
fn test_polonius_fact_generation() {
    let src = "let x = 1\nprint x\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");

    let mir = lower_program_to_mir(&prog);
    let facts = polonius::build_polonius_facts(&mir);

    assert!(!facts.is_empty(), "Should generate facts");
    assert!(
        facts.iter().any(|f| f.contains("def ")),
        "Should have def facts"
    );
}

#[test]
fn test_cfg_region_generation() {
    let src = "fn test()\n    return 0\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");

    let mir = lower_program_to_mir(&prog);
    let regions = polonius::generate_cfg_regions(&mir);

    assert!(!regions.is_empty(), "Should generate regions");
}

#[test]
fn test_loan_facts_generation() {
    let src = "fn test()\n    return 0\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");

    let mir = lower_program_to_mir(&prog);
    let loans = polonius::generate_loan_facts(&mir);

    // Should generate loan facts structure (may be empty for simple functions)
    let _ = loans;
}

#[test]
fn test_function_with_args() {
    let src = "fn add(a, b)\n    return a + b\nlet result = add(1, 2)\nprint result\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");

    let mir = lower_program_to_mir(&prog);
    let result = polonius::check_mir(&mir);
    assert!(result.is_ok(), "Function call should be valid");
}

#[test]
fn test_nested_expressions() {
    let src = "let a = 1 + 2 * 3\nprint a\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");

    let mir = lower_program_to_mir(&prog);
    let facts = polonius::build_polonius_facts(&mir);
    assert!(!facts.is_empty());
}
