use omni_compiler::ast::{Expr, Program, Stmt, Visibility};
use omni_compiler::diagnostics::Span;
use omni_compiler::resolver;
use omni_compiler::type_checker;

#[test]
fn inferred_return_unifies() {
    let f_fn = Stmt::Fn {
        name: "f".to_string(),
        visibility: Visibility::Private,
        is_async: false,
        type_params: vec![("T".to_string(), Vec::new())],
        params: vec![],
        ret_type: Some("T".to_string()),
        effects: vec![],
        contracts: vec![],
        body: vec![
            Stmt::Let(
                "x".to_string(),
                None,
                Expr::Number(42, Span::default()),
                Span::default(),
            ),
            Stmt::ExprStmt(Expr::Var("x".to_string(), Span::default()), Span::default()),
        ],
        span: Span::default(),
    };

    let call = Stmt::ExprStmt(
        Expr::Call("f".to_string(), vec![], Span::default()),
        Span::default(),
    );
    let prog = Program {
        stmts: vec![f_fn, call],
    };
    assert!(resolver::resolve_program(&prog).is_ok());
    assert!(type_checker::type_check_program(&prog).is_ok());
}

#[test]
fn two_param_generic_unify() {
    let pair_fn = Stmt::Fn {
        name: "pair".to_string(),
        visibility: Visibility::Private,
        is_async: false,
        type_params: vec![("T".to_string(), Vec::new()), ("U".to_string(), Vec::new())],
        params: vec![("a".to_string(), None), ("b".to_string(), None)],
        ret_type: Some("T".to_string()),
        effects: vec![],
        contracts: vec![],
        body: vec![Stmt::ExprStmt(
            Expr::Var("a".to_string(), Span::default()),
            Span::default(),
        )],
        span: Span::default(),
    };

    let call = Stmt::Let(
        "r".to_string(),
        None,
        Expr::Call(
            "pair".to_string(),
            vec![
                Expr::Number(1, Span::default()),
                Expr::Number(2, Span::default()),
            ],
            Span::default(),
        ),
        Span::default(),
    );
    let prog = Program {
        stmts: vec![pair_fn, call],
    };
    assert!(resolver::resolve_program(&prog).is_ok());
    assert!(type_checker::type_check_program(&prog).is_ok());
}
