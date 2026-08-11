use omni_compiler::diagnostics::Span;
use omni_compiler::interpreter;
use omni_compiler::parse_file;
use omni_compiler::resolver;
use omni_compiler::type_checker;
use std::io::Write;

#[test]
fn hello_world_type_checks() {
    let src = "print \"hello\"\n";
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();
    let prog = parse_file(path).expect("parse failed");
    assert!(resolver::resolve_program(&prog).is_ok());
    assert!(type_checker::type_check_program(&prog).is_ok());
}

#[test]
fn undefined_variable_detected() {
    let src = "print x\n";
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();
    let prog = parse_file(path).expect("parse failed");
    let res = resolver::resolve_program(&prog);
    assert!(res.is_err());
}

#[test]
fn generic_identity_typechecks() {
    use omni_compiler::ast::{Expr, Program, Stmt, Visibility};

    let id_fn = Stmt::Fn {
        name: "id".to_string(),
        visibility: Visibility::Private,
        is_async: false,
        type_params: vec![("T".to_string(), Vec::new())],
        params: vec![("x".to_string(), None)],
        ret_type: Some("T".to_string()),
        effects: vec![],
        contracts: vec![],
        body: vec![Stmt::ExprStmt(
            Expr::Var("x".to_string(), Span::default()),
            Span::default(),
        )],
        span: Span::default(),
    };

    let call = Stmt::Let(
        "a".to_string(),
        None,
        Expr::Call(
            "id".to_string(),
            vec![Expr::Number(1, Span::default())],
            Span::default(),
        ),
        Span::default(),
    );

    let prog = Program {
        stmts: vec![id_fn, call],
    };
    assert!(resolver::resolve_program(&prog).is_ok());
    assert!(type_checker::type_check_program(&prog).is_ok());
}

#[test]
fn io_annotated_function_checks() {
    use omni_compiler::ast::{Expr, Program, Stmt, Visibility};

    let log_fn = Stmt::Fn {
        name: "log".to_string(),
        visibility: Visibility::Private,
        is_async: false,
        type_params: vec![],
        params: vec![("s".to_string(), None)],
        ret_type: None,
        effects: vec!["io".to_string()],
        contracts: vec![],
        body: vec![Stmt::Print(
            Expr::Var("s".to_string(), Span::default()),
            Span::default(),
        )],
        span: Span::default(),
    };

    let call = Stmt::ExprStmt(
        Expr::Call(
            "log".to_string(),
            vec![Expr::StringLit("hi".to_string(), Span::default())],
            Span::default(),
        ),
        Span::default(),
    );
    let prog = Program {
        stmts: vec![log_fn, call],
    };
    assert!(resolver::resolve_program(&prog).is_ok());
    assert!(type_checker::type_check_program(&prog).is_ok());
}

#[test]
fn string_field_access_runs() {
    let src = "let s = \"hello\"\nlet n = s.len\nprint n\n";
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();

    let prog = parse_file(path).expect("parse failed");
    assert!(resolver::resolve_program(&prog).is_ok());
    assert!(type_checker::type_check_program(&prog).is_ok());
    assert!(interpreter::run_program(&prog).is_ok());
}

#[test]
fn tuple_and_index_semantics_work() {
    use omni_compiler::ast::{Expr, Program, Stmt};

    let prog = Program {
        stmts: vec![
            Stmt::Let(
                "pair".to_string(),
                None,
                Expr::Tuple(
                    vec![
                        Expr::Number(1, Span::default()),
                        Expr::Number(2, Span::default()),
                    ],
                    Span::default(),
                ),
                Span::default(),
            ),
            Stmt::Let(
                "first".to_string(),
                None,
                Expr::Index(
                    Box::new(Expr::Var("pair".to_string(), Span::default())),
                    Box::new(Expr::Number(0, Span::default())),
                    Span::default(),
                ),
                Span::default(),
            ),
            Stmt::Print(
                Expr::Var("first".to_string(), Span::default()),
                Span::default(),
            ),
        ],
    };

    assert!(resolver::resolve_program(&prog).is_ok());
    assert!(type_checker::type_check_program(&prog).is_ok());
    assert!(interpreter::run_program(&prog).is_ok());
}
