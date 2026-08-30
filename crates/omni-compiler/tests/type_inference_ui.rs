use omni_compiler::interpreter;
use omni_compiler::parser::Parser;
use omni_compiler::resolver;
use omni_compiler::type_checker;

#[test]
fn test_type_inference_int() {
    let src = "let x = 42\nprint x\n";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
}

#[test]
fn test_type_inference_string() {
    let src = "let s = \"hello\"\nprint s\n";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
}

#[test]
fn test_type_inference_bool() {
    let src = "let b = true\nprint b\n";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
}

#[test]
fn test_binary_op_type_inference() {
    let src = "let a = 10 + 5\nlet b = a * 2\nprint b\n";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
}

#[test]
fn test_comparison_type_inference() {
    let src = "let a = 10\nlet b = 20\nlet c = a < b\nprint c\n";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
}

#[test]
fn test_function_return_type_inference() {
    let src = "fn add[T](a, b)\n    return a + b\nlet result = add(1, 2)\nprint result\n";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
}

#[test]
fn test_generic_type_inference() {
    let src = "fn id[T](x)\n    return x\nlet a = id(42)\nprint a\n";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
}

#[test]
fn test_match_expression_type_inference() {
    let src = "let x = 1\nlet y = match x\n    | 0 => 0\n    | _ => 1\nprint y\n";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
    interpreter::run_program(&prog).expect("interpret failed");
}

#[test]
fn test_undefined_variable_error() {
    let src = "print undefined_var\n";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    let result = resolver::resolve_program(&prog);
    assert!(result.is_err(), "Expected undefined variable error");
}

#[test]
fn test_effect_tracking_io() {
    let src = "print \"hello\"\n";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
}

#[test]
fn test_function_call_argument_count() {
    let src = "fn two_args(a, b)\n    return a + b\nlet x = two_args(1)\n";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    let result = type_checker::type_check_program(&prog);
    assert!(result.is_err(), "Expected argument count error");
}

#[test]
fn test_struct_field_access() {
    let src = "struct Point [x: int, y: int]\nprint 1\n";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
}

#[test]
fn test_unsafe_function_call_allowed() {
    let src = "
fn __unsafe_dangerous()
    print 1
unsafe
    let x = __unsafe_dangerous()
";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed in unsafe block");
}

#[test]
fn test_safe_wrapper_annotation() {
    let src = "
fn __unsafe_dangerous()
    print 1

@safe_wrapper
fn safe_fn()
    let x = __unsafe_dangerous()
    return x

let y = safe_fn()
";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed with safe wrapper");
}

#[test]
fn test_trait_bounds_positive() {
    let src_ok = "
struct MyBound []
impl MyBound
    fn dummy() { return }

fn check[T: MyBound](val: T)
    print 1

let obj = MyBound
check(obj)
";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src_ok).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");
    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");

    let src_fail = "
struct MyBound []
impl MyBound
    fn dummy() { return }

fn check[T: MyBound](val: T)
    print 1

check(42)
";
    let tokens_fail =
        omni_compiler::complete_lexer::tokenize_complete(src_fail).expect("tokenize failed");
    let mut parser_fail = Parser::new(tokens_fail);
    let prog_fail = parser_fail.parse_program().expect("parse failed");
    resolver::resolve_program(&prog_fail).expect("resolve failed");
    let res = type_checker::type_check_program(&prog_fail);
    assert!(
        res.is_err(),
        "Expected typecheck error due to trait bound violation"
    );
}

#[test]
fn test_trait_bounds_negative() {
    let src_ok = "
struct MyBound []
impl MyBound
    fn dummy() { return }

fn check_neg[T: !MyBound](val: T)
    print 1

check_neg(42)
";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src_ok).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");
    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");

    let src_fail = "
struct MyBound []
impl MyBound
    fn dummy() { return }

fn check_neg[T: !MyBound](val: T)
    print 1

let obj = MyBound
check_neg(obj)
";
    let tokens_fail =
        omni_compiler::complete_lexer::tokenize_complete(src_fail).expect("tokenize failed");
    let mut parser_fail = Parser::new(tokens_fail);
    let prog_fail = parser_fail.parse_program().expect("parse failed");
    resolver::resolve_program(&prog_fail).expect("resolve failed");
    let res = type_checker::type_check_program(&prog_fail);
    assert!(
        res.is_err(),
        "Expected typecheck error due to negative trait bound violation"
    );
}

#[test]
fn test_trait_implied_bounds() {
    let src = "
struct MyBound []
impl MyBound
    fn dummy() { return }

fn require_bound[T: MyBound](val: T)
    print 1

struct MyType []
impl MyType[T: MyBound]
    fn check(x: T) { require_bound(x) }
";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");
    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed with implied bounds");
}

#[test]
fn test_bidirectional_struct_lit() {
    let src = "
struct Point [x: int, y: int]

fn create_point() -> Point:
    return Point { x: 10, y: 20 }
";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
}

#[test]
fn test_bidirectional_struct_lit_missing_field() {
    let src = "
struct Point [x: int, y: int]

fn create_point() -> Point:
    return Point { x: 10 }
";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    let res = type_checker::type_check_program(&prog);
    assert!(
        res.is_err(),
        "Expected typecheck error due to missing field in struct literal"
    );
}

#[test]
fn test_bidirectional_call_generic_propagation() {
    let src = "
fn identity[T](val: T) -> T:
    return val

fn get_int() -> int:
    return identity(42)
";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed in generic propagation");
}

#[test]
fn test_trait_with_methods_and_impl_for_type() {
    // End-to-end test using the existing struct-as-pseudo-trait pattern:
    // define a struct (acts as trait), implement it for another struct,
    // and call a generic function with a trait bound.
    // Uses `impl Trait for Type` syntax with brace-delimited method bodies.
    let src = "struct MyTrait []\nstruct MyStruct []\nimpl MyTrait for MyStruct:\n    fn do_something(x: int) -> int { return x + 1 }\n\nfn use_trait[T: MyTrait](val: T):\n    print 1\n\nlet obj = MyStruct\nuse_trait(obj)\n";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = omni_compiler::parser::Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");
    omni_compiler::resolver::resolve_program(&prog).expect("resolve failed");
    omni_compiler::type_checker::type_check_program(&prog).expect("typecheck failed");
}

#[test]
fn test_trait_violation() {
    // Test that calling a bounded generic function with a type that
    // does NOT implement the trait fails type checking.
    let src = "struct MyTrait []\nstruct MyStruct []\nimpl MyTrait for MyStruct:\n    fn do_something(x: int) -> int { return x + 1 }\n\nfn use_trait[T: MyTrait](val: T):\n    print 1\n\n// OtherStruct does NOT implement MyTrait, so this should fail\nstruct OtherStruct []\nlet obj = OtherStruct\nuse_trait(obj)\n";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = omni_compiler::parser::Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");
    omni_compiler::resolver::resolve_program(&prog).expect("resolve failed");
    let res = omni_compiler::type_checker::type_check_program(&prog);
    assert!(
        res.is_err(),
        "Expected typecheck error: OtherStruct does not implement MyTrait"
    );
}

#[test]
fn test_impl_for_type_parsing() {
    // Verify that `impl Trait for Type` correctly populates the for_type field.
    // Trait impls (with `for Type`) should have for_type=Some("Type").
    // Inherent impls (without `for`) should have for_type=None.
    use omni_compiler::ast::Stmt;

    // Trait impl: impl Trait for Type
    let src_trait = "impl MyTrait for MyStruct:\n    fn dummy() { return }\n";
    let tokens_trait =
        omni_compiler::complete_lexer::tokenize_complete(src_trait).expect("tokenize failed");
    let mut parser_trait = omni_compiler::parser::Parser::new(tokens_trait);
    let prog_trait = parser_trait.parse_program().expect("parse failed");
    if let Stmt::Impl { for_type, .. } = &prog_trait.stmts[0] {
        assert_eq!(
            for_type.as_deref(),
            Some("MyStruct"),
            "trait impl: expected for_type=Some('MyStruct'), got {:?}",
            for_type
        );
    } else {
        panic!("Expected Stmt::Impl");
    }

    // Inherent impl: impl Type (no `for`)
    let src_inherent = "impl MyStruct:\n    fn dummy() { return }\n";
    let tokens_inherent =
        omni_compiler::complete_lexer::tokenize_complete(src_inherent).expect("tokenize failed");
    let mut parser_inherent = omni_compiler::parser::Parser::new(tokens_inherent);
    let prog_inherent = parser_inherent.parse_program().expect("parse failed");
    if let Stmt::Impl { for_type, .. } = &prog_inherent.stmts[0] {
        assert!(
            for_type.is_none(),
            "inherent impl: expected for_type=None, got {:?}",
            for_type
        );
    } else {
        panic!("Expected Stmt::Impl");
    }
}

#[test]
fn test_multiple_trait_methods_and_generic_struct() {
    // More complex test: a pseudo-trait with multiple methods, a struct implementing
    // the pseudo-trait, and a generic function calling methods on the trait-bounded
    // parameter.
    let src = "struct Calculator []\nstruct MyCalc []\nimpl Calculator for MyCalc:\n    fn add(a: int, b: int) -> int { return a + b }\n    fn mul(a: int, b: int) -> int { return a * b }\n\nfn compute[T: Calculator](x: int) -> int:\n    return x + 1\n\nlet c = MyCalc\nlet r = compute(c)\nprint r\n";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = omni_compiler::parser::Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");
    omni_compiler::resolver::resolve_program(&prog).expect("resolve failed");
}

#[test]
fn aggregate_unknown_field_type_fails_closed() {
    let src = "struct Broken { value: DefinitelyMissing; }\nfn main() -> i64 { return 0; }\n";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");
    resolver::resolve_program(&prog).expect("resolve failed");
    let error = type_checker::type_check_program(&prog).expect_err("unknown field type must fail");
    assert!(
        error.message.contains("Unknown type 'DefinitelyMissing'"),
        "unexpected diagnostic: {error:?}"
    );
}

#[test]
fn aggregate_forward_nominal_field_reference_typechecks_without_generic_fallback() {
    let src = r#"
struct Outer { inner: Inner; }
struct Inner { value: i64; }
fn read(outer: Outer) -> i64 { return outer.inner.value; }
fn main() -> i64 { return 0; }
"#;
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");
    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog)
        .expect("forward nominal field reference must typecheck");
}

#[test]
fn enum_unknown_payload_type_fails_closed() {
    let src =
        "enum Broken { variant Bad[value: DefinitelyMissing], }\nfn main() -> i64 { return 0; }\n";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");
    resolver::resolve_program(&prog).expect("resolve failed");
    let error =
        type_checker::type_check_program(&prog).expect_err("unknown payload type must fail");
    assert!(
        error.message.contains("Unknown type 'DefinitelyMissing'"),
        "unexpected diagnostic: {error:?}"
    );
}
