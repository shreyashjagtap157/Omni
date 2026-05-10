use omni_compiler::ast::{Expr, Program, Stmt};
use omni_compiler::comptime::ComptimeContext;

#[test]
fn comptime_evaluates_basic_expression() {
    let program = Program {
        stmts: vec![Stmt::ExprStmt(Expr::BinaryOp {
            op: omni_compiler::complete_lexer::TokenKind::Plus,
            left: Box::new(Expr::Number(2)),
            right: Box::new(Expr::Number(3)),
        })],
    };

    let mut context = ComptimeContext::new();
    let value = context.eval_program(&program).expect("comptime failed");
    assert_eq!(value, omni_compiler::comptime::ComptimeValue::Int(5));
}

#[test]
fn comptime_match_expression_evaluates() {
    use omni_compiler::ast::MatchArm;
    use omni_compiler::ast::Pattern;

    let program = Program {
        stmts: vec![Stmt::ExprStmt(Expr::Match {
            expr: Box::new(Expr::Number(1)),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Literal(0),
                    guard: None,
                    body: Box::new(Expr::Number(0)),
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: Box::new(Expr::Number(7)),
                },
            ],
        })],
    };

    let mut context = ComptimeContext::new();
    let value = context.eval_program(&program).expect("comptime failed");
    assert_eq!(value, omni_compiler::comptime::ComptimeValue::Int(7));
}

#[test]
fn range_parses_inclusive() {
    use omni_compiler::complete_lexer;

    let src = "1...5";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(src).expect("tokenize failed");

    assert!(tokens
        .iter()
        .any(|t| t.kind == omni_compiler::complete_lexer::TokenKind::DotDotDot));
}
