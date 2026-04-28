use omni_compiler::ast::{Expr, Program, Stmt};
use omni_compiler::resolver;
use omni_compiler::type_checker;

#[test]
fn inferred_return_unifies() {
    let f_fn = Stmt::Fn {
        name: "f".to_string(),
        is_public: false,
        is_async: false,
        type_params: vec!["T".to_string()],
        params: vec![],
        ret_type: Some("T".to_string()),
        effects: vec![],
        body: vec![
            Stmt::Let("x".to_string(), Expr::Number(42)),
            Stmt::ExprStmt(Expr::Var("x".to_string())),
        ],
    };

    let call = Stmt::ExprStmt(Expr::Call("f".to_string(), vec![]));
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
        is_public: false,
        is_async: false,
        type_params: vec!["T".to_string(), "U".to_string()],
        params: vec!["a".to_string(), "b".to_string()],
        ret_type: Some("T".to_string()),
        effects: vec![],
        body: vec![Stmt::ExprStmt(Expr::Var("a".to_string()))],
    };

    let call = Stmt::Let(
        "r".to_string(),
        Expr::Call("pair".to_string(), vec![Expr::Number(1), Expr::Number(2)]),
    );
    let prog = Program {
        stmts: vec![pair_fn, call],
    };
    assert!(resolver::resolve_program(&prog).is_ok());
    assert!(type_checker::type_check_program(&prog).is_ok());
}
