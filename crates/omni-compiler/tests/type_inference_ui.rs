use omni_compiler::interpreter;
use omni_compiler::lexer::Lexer;
use omni_compiler::parser::Parser;
use omni_compiler::resolver;
use omni_compiler::type_checker;

#[test]
fn test_type_inference_int() {
    let src = "let x = 42\nprint x\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
}

#[test]
fn test_type_inference_string() {
    let src = "let s = \"hello\"\nprint s\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
}

#[test]
fn test_type_inference_bool() {
    let src = "let b = true\nprint b\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
}

#[test]
fn test_binary_op_type_inference() {
    let src = "let a = 10 + 5\nlet b = a * 2\nprint b\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
}

#[test]
fn test_comparison_type_inference() {
    let src = "let a = 10\nlet b = 20\nlet c = a < b\nprint c\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
}

#[test]
fn test_function_return_type_inference() {
    let src = "fn add<T>(a, b)\n    return a + b\nlet result = add(1, 2)\nprint result\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
}

#[test]
fn test_generic_type_inference() {
    let src = "fn id<T>(x)\n    return x\nlet a = id(42)\nprint a\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
}

#[test]
fn test_match_expression_type_inference() {
    let src = "let x = 1\nlet y = match x\n    | 0 => 0\n    | _ => 1\nprint y\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
    interpreter::run_program(&prog).expect("interpret failed");
}

#[test]
fn test_undefined_variable_error() {
    let src = "print undefined_var\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    let result = resolver::resolve_program(&prog);
    assert!(result.is_err(), "Expected undefined variable error");
}

#[test]
fn test_effect_tracking_io() {
    let src = "print \"hello\"\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
}

#[test]
fn test_function_call_argument_count() {
    let src = "fn two_args(a, b)\n    return a + b\nlet x = two_args(1)\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    let result = type_checker::type_check_program(&prog);
    assert!(result.is_err(), "Expected argument count error");
}

#[test]
fn test_struct_field_access() {
    let src = "struct Point [x: int, y: int]\nprint 1\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
}
