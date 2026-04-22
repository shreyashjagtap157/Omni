use omni_compiler::lexer::Lexer;
use omni_compiler::parser::Parser;
use omni_compiler::type_checker::type_check_program;

fn check(src: &str) -> Result<(), String> {
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().map_err(|e| e.to_string())?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().map_err(|e| e.to_string())?;
    type_check_program(&program)
}

#[test]
fn public_fn_needs_effect_annotation() {
    let src = "pub fn foo() { print \"hi\" }";
    let result = check(src);
    assert!(
        result.is_err(),
        "Public functions with IO should require effect annotation"
    );
}

#[test]
fn public_fn_with_explicit_pure() {
    let src = "pub pure fn bar() { 1 }";
    let result = check(src);
    assert!(result.is_ok());
}

#[test]
fn public_fn_with_explicit_io() {
    let src = "pub io fn baz() { print \"x\" }";
    let result = check(src);
    assert!(result.is_ok());
}

#[test]
fn private_fn_infers_effect() {
    let src = "fn qux() { 1 + 2 }";
    let result = check(src);
    assert!(result.is_ok());
}
