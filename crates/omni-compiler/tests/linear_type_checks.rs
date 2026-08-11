use omni_compiler::ast::{Expr, Program, Stmt};
use omni_compiler::diagnostics::Span;
use omni_compiler::parse_file;
use omni_compiler::type_checker::type_check_program;
use std::io::Write;

#[test]
fn linear_type_basic_parse() {
    // Test that linear let parses correctly
    let src = "linear a = 1\nprint a\n";
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();
    let prog = parse_file(path).expect("parse failed");
    let result = type_check_program(&prog);
    assert!(result.is_ok(), "linear type should work");
}

#[test]
fn linear_type_moved_error() {
    // Linear type moved to another variable should error if used again
    let src = "linear a = 1\nlet b = a\nprint a\n";
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();
    let prog = parse_file(path).expect("parse failed");
    let result = type_check_program(&prog);
    assert!(
        result.is_err(),
        "expected error for using moved linear value"
    );
}

#[test]
fn linear_type_proper_use() {
    // Linear type moved once should be fine
    let src = "linear a = 1\nlet b = a\nprint b\n";
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();
    let prog = parse_file(path).expect("parse failed");
    let result = type_check_program(&prog);
    assert!(result.is_ok(), "linear type moved once should be valid");
}

#[test]
fn linear_type_unused_error() {
    // Linear type defined but not used should error
    let src = "fn main() { linear a = 1 }";
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();
    let prog = parse_file(path).expect("parse failed");
    let result = type_check_program(&prog);
    assert!(result.is_err(), "expected error for unused linear value");
}

#[test]
fn linear_type_double_use_error() {
    // Linear type used twice should error
    let src = "linear a = 1\nprint a\nprint a\n";
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();
    let prog = parse_file(path).expect("parse failed");
    let result = type_check_program(&prog);
    assert!(
        result.is_err(),
        "expected error for double use of linear value"
    );
}

#[test]
fn linear_type_consumed_on_both_if_branches_is_valid() {
    let src = r#"
fn main() {
    linear a = 1
    if true {
        print a
    } else {
        let b = a
        print b
    }
}
"#;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();
    let prog = parse_file(path).expect("parse failed");
    let result = type_check_program(&prog);
    assert!(
        result.is_ok(),
        "linear value consumed on both branches should be valid: {result:?}"
    );
}

#[test]
fn linear_type_consumed_on_only_one_if_branch_is_rejected_at_exit() {
    let src = r#"
fn main() {
    linear a = 1
    if true {
        print a
    } else {
    }
}
"#;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();
    let prog = parse_file(path).expect("parse failed");
    let result = type_check_program(&prog);
    assert!(
        result.is_err(),
        "conditional linear consumption must be rejected"
    );
    let message = format!("{:?}", result.err().unwrap());
    assert!(
        message.contains("not consumed on every path"),
        "unexpected diagnostic: {message}"
    );
}

#[test]
fn linear_type_moved_on_only_one_if_branch_cannot_be_used_after_join() {
    let src = r#"
fn main() {
    linear a = 1
    if true {
        print a
    } else {
    }
    print a
}
"#;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();
    let prog = parse_file(path).expect("parse failed");
    let result = type_check_program(&prog);
    assert!(result.is_err(), "conditional move must poison later use");
    let message = format!("{:?}", result.err().unwrap());
    assert!(
        message.contains("conditionally available"),
        "unexpected diagnostic: {message}"
    );
}

#[test]
fn linear_type_loop_conditional_consumption_is_rejected() {
    let src = r#"
fn main() {
    linear a = 1
    while true {
        print a
    }
}
"#;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();
    let prog = parse_file(path).expect("parse failed");
    let result = type_check_program(&prog);
    assert!(
        result.is_err(),
        "loop consumption must be rejected as conditional/repeated"
    );
    let message = format!("{:?}", result.err().unwrap());
    assert!(
        message.contains("not consumed on every path")
            || message.contains("conditionally available"),
        "unexpected diagnostic: {message}"
    );
}

#[test]
fn linear_type_loop_body_local_value_must_be_consumed_each_iteration() {
    let src = r#"
fn main() {
    while true {
        linear a = 1
    }
}
"#;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();
    let prog = parse_file(path).expect("parse failed");
    let result = type_check_program(&prog);
    assert!(
        result.is_err(),
        "loop-local linear value must be consumed before iteration boundary"
    );
    let message = format!("{:?}", result.err().unwrap());
    assert!(
        message.contains("iteration boundary") || message.contains("loop path"),
        "unexpected diagnostic: {message}"
    );
}

#[test]
fn linear_type_loop_body_local_consumed_value_is_valid() {
    let src = r#"
fn main() {
    while true {
        linear a = 1
        print a
    }
}
"#;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();
    let prog = parse_file(path).expect("parse failed");
    let result = type_check_program(&prog);
    assert!(
        result.is_ok(),
        "loop-local linear value consumed inside the body should be accepted: {result:?}"
    );
}

fn test_span() -> Span {
    Span::new(1, 1, 1, 2)
}

#[test]
fn linear_type_if_expression_consumed_on_both_branches_is_valid() {
    let span = test_span();
    let prog = Program {
        stmts: vec![Stmt::Fn {
            name: "main".to_string(),
            visibility: Default::default(),
            is_async: false,
            type_params: vec![],
            params: vec![],
            ret_type: None,
            effects: vec![],
            contracts: vec![],
            body: vec![
                Stmt::LetLinear(
                    "a".to_string(),
                    None,
                    Expr::Number(1, span.clone()),
                    span.clone(),
                ),
                Stmt::Let(
                    "x".to_string(),
                    None,
                    Expr::IfExpr {
                        cond: Box::new(Expr::Bool(true, span.clone())),
                        then: Box::new(Expr::Var("a".to_string(), span.clone())),
                        else_: Box::new(Expr::Var("a".to_string(), span.clone())),
                        span: span.clone(),
                    },
                    span.clone(),
                ),
                Stmt::Print(Expr::Var("x".to_string(), span.clone()), span.clone()),
            ],
            span,
        }],
    };
    let result = type_check_program(&prog);
    assert!(
        result.is_ok(),
        "if expression with linear consumed on both branches should be accepted: {result:?}"
    );
}

#[test]
fn linear_type_if_expression_consumed_on_one_branch_poisoned_after_join() {
    let span = test_span();
    let prog = Program {
        stmts: vec![Stmt::Fn {
            name: "main".to_string(),
            visibility: Default::default(),
            is_async: false,
            type_params: vec![],
            params: vec![],
            ret_type: None,
            effects: vec![],
            contracts: vec![],
            body: vec![
                Stmt::LetLinear(
                    "a".to_string(),
                    None,
                    Expr::Number(1, span.clone()),
                    span.clone(),
                ),
                Stmt::Let(
                    "x".to_string(),
                    None,
                    Expr::IfExpr {
                        cond: Box::new(Expr::Bool(true, span.clone())),
                        then: Box::new(Expr::Var("a".to_string(), span.clone())),
                        else_: Box::new(Expr::Number(2, span.clone())),
                        span: span.clone(),
                    },
                    span.clone(),
                ),
                Stmt::Print(Expr::Var("a".to_string(), span.clone()), span.clone()),
                Stmt::Print(Expr::Var("x".to_string(), span.clone()), span.clone()),
            ],
            span,
        }],
    };
    let result = type_check_program(&prog);
    assert!(
        result.is_err(),
        "conditional expression move must poison later use"
    );
    let message = format!("{:?}", result.err().unwrap());
    assert!(
        message.contains("conditionally available")
            || message.contains("not consumed on every path"),
        "unexpected diagnostic: {message}"
    );
}

#[test]
fn linear_struct_field_move_preserves_sibling() {
    let src = r#"
struct Point { x: i64; y: i64; }
fn main() {
    linear p = Point { x: 7, y: 42 }
    print p.x
    print p.y
}
"#;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let prog = parse_file(tmp.path()).expect("parse failed");
    let result = type_check_program(&prog);
    assert!(
        result.is_ok(),
        "moving both fields should fully consume p: {result:?}"
    );
}

#[test]
fn linear_struct_whole_use_after_field_move_is_rejected() {
    let src = r#"
struct Point { x: i64; y: i64; }
fn main() {
    linear p = Point { x: 7, y: 42 }
    print p.x
    print p
}
"#;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let prog = parse_file(tmp.path()).expect("parse failed");
    let err = type_check_program(&prog).expect_err("whole use after partial move must fail");
    let message = format!("{err:?}");
    assert!(
        message.contains("partially moved"),
        "unexpected diagnostic: {message}"
    );
}

#[test]
fn linear_struct_field_double_move_is_rejected() {
    let src = r#"
struct Point { x: i64; y: i64; }
fn main() {
    linear p = Point { x: 7, y: 42 }
    print p.x
    print p.x
    print p.y
}
"#;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let prog = parse_file(tmp.path()).expect("parse failed");
    let err = type_check_program(&prog).expect_err("field double move must fail");
    let message = format!("{err:?}");
    assert!(
        message.contains("moved field") || message.contains("moved value"),
        "unexpected diagnostic: {message}"
    );
}

#[test]
fn linear_struct_partial_move_at_exit_is_rejected() {
    let src = r#"
struct Point { x: i64; y: i64; }
fn main() {
    linear p = Point { x: 7, y: 42 }
    print p.x
}
"#;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let prog = parse_file(tmp.path()).expect("parse failed");
    let err = type_check_program(&prog).expect_err("partial consumption at exit must fail");
    let message = format!("{err:?}");
    assert!(
        message.contains("partially consumed"),
        "unexpected diagnostic: {message}"
    );
}

#[test]
fn linear_struct_same_field_moved_on_both_branches_preserves_sibling() {
    let src = r#"
struct Point { x: i64; y: i64; }
fn main() {
    linear p = Point { x: 7, y: 42 }
    if true { print p.x } else { print p.x }
    print p.y
}
"#;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let prog = parse_file(tmp.path()).expect("parse failed");
    let result = type_check_program(&prog);
    assert!(
        result.is_ok(),
        "identical partial moves should merge exactly: {result:?}"
    );
}

#[test]
fn linear_struct_different_fields_moved_across_branches_are_conditional() {
    let src = r#"
struct Point { x: i64; y: i64; }
fn main() {
    linear p = Point { x: 7, y: 42 }
    if true { print p.x } else { print p.y }
    print p.x
}
"#;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let prog = parse_file(tmp.path()).expect("parse failed");
    let err =
        type_check_program(&prog).expect_err("mismatched partial moves must not merge as definite");
    let message = format!("{err:?}");
    assert!(
        message.contains("conditionally available"),
        "unexpected diagnostic: {message}"
    );
}

#[test]
fn branch_local_linear_value_must_be_consumed_before_scope_exit() {
    let src = r#"
fn main() {
    if true { linear temp = 7 } else { }
}
"#;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let prog = parse_file(tmp.path()).expect("parse failed");
    let err = type_check_program(&prog).expect_err("branch-local leak must fail");
    assert!(format!("{err:?}").contains("lexical scope"));
}

#[test]
fn branch_local_linear_value_consumed_inside_scope_does_not_leak_to_join() {
    let src = r#"
fn main() {
    linear outer = 1
    if true { linear temp = 7; print temp; print outer } else { print outer }
}
"#;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let prog = parse_file(tmp.path()).expect("parse failed");
    let result = type_check_program(&prog);
    assert!(
        result.is_ok(),
        "consumed branch-local must not contaminate join: {result:?}"
    );
}

#[test]
fn linear_partial_move_field_reinitialization_restores_whole_value() {
    let src = r#"
struct Pair { x: Int, y: Int }
fn main() {
    linear p = Pair { x: 7, y: 42 }
    print p.x
    p.x = 9
    print p
}
"#;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let prog = parse_file(tmp.path()).expect("parse failed");
    let result = type_check_program(&prog);
    assert!(
        result.is_ok(),
        "reinitializing moved field should restore aggregate: {result:?}"
    );
}

#[test]
fn linear_live_field_assignment_is_rejected_until_mutation_is_qualified() {
    let src = r#"
struct Pair { x: Int, y: Int }
fn main() {
    linear p = Pair { x: 7, y: 42 }
    p.x = 9
    print p
}
"#;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let prog = parse_file(tmp.path()).expect("parse failed");
    let result = type_check_program(&prog);
    assert!(
        result.is_err(),
        "live-field mutation must remain fail-closed"
    );
    let message = format!("{:?}", result.err().unwrap());
    assert!(
        message.contains("still initialized") || message.contains("mutation"),
        "unexpected diagnostic: {message}"
    );
}

#[test]
fn linear_reinitializing_one_of_two_moved_fields_keeps_root_partial() {
    let src = r#"
struct Pair { x: Int, y: Int }
fn main() {
    linear p = Pair { x: 7, y: 42 }
    print p.x
    print p.y
    p.x = 9
    print p
}
"#;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let prog = parse_file(tmp.path()).expect("parse failed");
    let result = type_check_program(&prog);
    assert!(
        result.is_err(),
        "root must remain partial while p.y is still moved"
    );
    let message = format!("{:?}", result.err().unwrap());
    assert!(
        message.contains("partially moved") || message.contains("moved"),
        "unexpected diagnostic: {message}"
    );
}

#[test]
fn linear_field_reinitialization_rejects_wrong_field_type() {
    let src = r#"
struct Pair { x: Int, y: Int }
fn main() {
    linear p = Pair { x: 7, y: 42 }
    print p.x
    p.x = "wrong"
    print p
}
"#;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let prog = parse_file(tmp.path()).expect("parse failed");
    let result = type_check_program(&prog);
    assert!(
        result.is_err(),
        "field reinitialization must preserve declared field type"
    );
    let message = format!("{:?}", result.err().unwrap());
    assert!(
        message.contains("expects") || message.contains("mismatch"),
        "unexpected diagnostic: {message}"
    );
}

#[test]
fn field_assignment_rejects_unknown_field() {
    let src = r#"
struct Pair { x: Int, y: Int }
fn main() {
    linear p = Pair { x: 7, y: 42 }
    p.z = 9
}
"#;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let prog = parse_file(tmp.path()).expect("parse failed");
    let result = type_check_program(&prog);
    assert!(result.is_err(), "unknown field assignment must be rejected");
    let message = format!("{:?}", result.err().unwrap());
    assert!(
        message.contains("no field") || message.contains("Unknown field"),
        "unexpected diagnostic: {message}"
    );
}
