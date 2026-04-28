use omni_compiler::ast::{Expr, Program, Stmt};
use omni_compiler::resolver;
use omni_compiler::type_checker;

#[test]
fn missing_effect_annotation_on_function_body() {
    let g_fn = Stmt::Fn {
        name: "g".to_string(),
        is_public: false,
        is_async: false,
        type_params: vec![],
        params: vec!["s".to_string()],
        ret_type: None,
        effects: vec![],
        body: vec![Stmt::Print(Expr::StringLit("hi".to_string()))],
    };

    let f_fn = Stmt::Fn {
        name: "f".to_string(),
        is_public: false,
        is_async: false,
        type_params: vec![],
        params: vec![],
        ret_type: None,
        effects: vec![],
        body: vec![Stmt::ExprStmt(Expr::Call(
            "g".to_string(),
            vec![Expr::StringLit("hey".to_string())],
        ))],
    };

    let prog = Program {
        stmts: vec![g_fn, f_fn],
    };
    assert!(resolver::resolve_program(&prog).is_ok());
    // g performs IO but has no effect annotation -> type checking should succeed (inferred)
    assert!(type_checker::type_check_program(&prog).is_ok());
}

#[test]
fn callee_annotated_but_caller_missing() {
    let g_fn = Stmt::Fn {
        name: "g".to_string(),
        is_public: false,
        is_async: false,
        type_params: vec![],
        params: vec!["s".to_string()],
        ret_type: None,
        effects: vec!["io".to_string()],
        body: vec![Stmt::Print(Expr::Var("s".to_string()))],
    };

    let f_fn = Stmt::Fn {
        name: "f".to_string(),
        is_public: false,
        is_async: false,
        type_params: vec![],
        params: vec![],
        ret_type: None,
        effects: vec![],
        body: vec![Stmt::ExprStmt(Expr::Call(
            "g".to_string(),
            vec![Expr::StringLit("test".to_string())],
        ))],
    };

    let prog = Program {
        stmts: vec![g_fn, f_fn],
    };
    assert!(resolver::resolve_program(&prog).is_ok());
    // f calls an io function but has no declared effects -> should be inferred and succeed
    assert!(type_checker::type_check_program(&prog).is_ok());
}

#[test]
fn both_annotated_passes() {
    let g_fn = Stmt::Fn {
        name: "g".to_string(),
        is_public: false,
        is_async: false,
        type_params: vec![],
        params: vec!["s".to_string()],
        ret_type: None,
        effects: vec!["io".to_string()],
        body: vec![Stmt::Print(Expr::Var("s".to_string()))],
    };

    let f_fn = Stmt::Fn {
        name: "f".to_string(),
        is_public: false,
        is_async: false,
        type_params: vec![],
        params: vec![],
        ret_type: None,
        effects: vec!["io".to_string()],
        body: vec![Stmt::ExprStmt(Expr::Call(
            "g".to_string(),
            vec![Expr::StringLit("test".to_string())],
        ))],
    };

    let prog = Program {
        stmts: vec![g_fn, f_fn],
    };
    assert!(resolver::resolve_program(&prog).is_ok());
    assert!(type_checker::type_check_program(&prog).is_ok());
}

#[test]
fn declared_missing_effects_fails() {
    let g_fn = Stmt::Fn {
        name: "g".to_string(),
        is_public: false,
        is_async: false,
        type_params: vec![],
        params: vec!["s".to_string()],
        ret_type: None,
        // declares only `pure` but body performs IO
        effects: vec!["pure".to_string()],
        body: vec![Stmt::Print(Expr::Var("s".to_string()))],
    };

    let prog = Program { stmts: vec![g_fn] };
    assert!(resolver::resolve_program(&prog).is_ok());
    // declaration does not include the observed IO effect -> should fail
    assert!(type_checker::type_check_program(&prog).is_err());
}
