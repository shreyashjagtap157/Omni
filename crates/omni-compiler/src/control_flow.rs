//! Source-level control-flow legality checks that are independent of typing.
//!
//! Keeping this pass explicit prevents `break`/`continue` outside a loop from
//! surviving until MIR/codegen, where there is no correct target to invent.

use crate::ast::{Program, Stmt};
use crate::diagnostics::{error_codes, Diagnostic};

pub fn validate(program: &Program) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Hosted/native functions own their initialization explicitly. The
    // Stage-0 compatibility mode still permits a source file with no explicit
    // main to act like a script, but once a user declares main we must never
    // silently emit and then ignore executable top-level statements.
    let has_explicit_main = program
        .stmts
        .iter()
        .any(|stmt| matches!(stmt, Stmt::Fn { name, .. } if name == "main"));
    if has_explicit_main {
        for stmt in &program.stmts {
            if is_executable_top_level(stmt) {
                diagnostics.push(Diagnostic::error(
                    error_codes::TYPE_TOP_LEVEL_EXECUTABLE_WITH_MAIN,
                    "executable top-level statements are not permitted when an explicit main function is present",
                ));
            }
        }
    }

    validate_stmts(&program.stmts, 0, &mut diagnostics);
    diagnostics
}

fn is_executable_top_level(stmt: &Stmt) -> bool {
    matches!(
        stmt,
        Stmt::Print(_, _)
            | Stmt::Let(_, _, _, _)
            | Stmt::LetLinear(_, _, _, _)
            | Stmt::ExprStmt(_, _)
            | Stmt::Block(_, _)
            | Stmt::If { .. }
            | Stmt::Loop { .. }
            | Stmt::For { .. }
            | Stmt::While { .. }
            | Stmt::Return(_, _)
            | Stmt::Break(_)
            | Stmt::Continue(_)
            | Stmt::Assign(_, _, _)
            | Stmt::ExprFieldAssign(_, _, _, _)
            | Stmt::WhileIn { .. }
            | Stmt::Unsafe { .. }
            | Stmt::Spawn { .. }
            | Stmt::Channel { .. }
    )
}

fn returns_a_value(ret_type: &Option<String>) -> bool {
    ret_type
        .as_deref()
        .is_some_and(|name| !matches!(name.trim(), "unit" | "void" | "()"))
}

fn stmt_definitely_returns(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return(_, _) => true,
        Stmt::Block(body, _) | Stmt::Unsafe { body, .. } | Stmt::UseScoped { body, .. } => {
            block_definitely_returns(body, false)
        }
        Stmt::If {
            then_body,
            else_body,
            ..
        } => {
            !else_body.is_empty()
                && block_definitely_returns(then_body, false)
                && block_definitely_returns(else_body, false)
        }
        // A `loop` executes its body at least once. If every path through the
        // first iteration returns, the function cannot fall through it.
        Stmt::Loop { body, .. } => block_definitely_returns(body, false),
        _ => false,
    }
}

fn block_definitely_returns(stmts: &[Stmt], _allow_trailing_expr_return: bool) -> bool {
    // Edition 1 eventually has value-yielding block tail expressions, but the
    // v0.1.2 parser does not preserve whether an expression had a semicolon.
    // Therefore this milestone requires an explicit `return` for value-returning
    // functions instead of inventing tail-expression semantics.
    stmts.iter().any(stmt_definitely_returns)
}

fn validate_stmts(stmts: &[Stmt], loop_depth: usize, diagnostics: &mut Vec<Diagnostic>) {
    for stmt in stmts {
        match stmt {
            Stmt::Break(_) if loop_depth == 0 => diagnostics.push(Diagnostic::error(
                error_codes::TYPE_BREAK_OUTSIDE_LOOP,
                "'break' is only valid inside loop, while, for, or while-in control flow",
            )),
            Stmt::Continue(_) if loop_depth == 0 => diagnostics.push(Diagnostic::error(
                error_codes::TYPE_CONTINUE_OUTSIDE_LOOP,
                "'continue' is only valid inside loop, while, for, or while-in control flow",
            )),
            Stmt::Loop { body, .. }
            | Stmt::For { body, .. }
            | Stmt::While { body, .. }
            | Stmt::WhileIn { body, .. } => validate_stmts(body, loop_depth + 1, diagnostics),
            Stmt::Block(body, _)
            | Stmt::ModBlock(_, body, _)
            | Stmt::Unsafe { body, .. }
            | Stmt::UseScoped { body, .. } => validate_stmts(body, loop_depth, diagnostics),
            Stmt::If {
                then_body,
                else_body,
                ..
            } => {
                validate_stmts(then_body, loop_depth, diagnostics);
                validate_stmts(else_body, loop_depth, diagnostics);
            }
            // A function is a control-flow boundary even if declared lexically
            // inside a loop. Its break/continue statements cannot target the
            // enclosing function's loop.
            Stmt::Fn {
                name,
                ret_type,
                contracts,
                body,
                ..
            } => {
                if returns_a_value(ret_type) && !block_definitely_returns(body, false) {
                    diagnostics.push(Diagnostic::error(
                        error_codes::TYPE_MISSING_RETURN,
                        format!(
                            "value-returning function '{}' can reach the end without returning a value",
                            name
                        ),
                    ));
                }
                validate_stmts(contracts, 0, diagnostics);
                validate_stmts(body, 0, diagnostics);
            }
            Stmt::Impl { methods, .. } | Stmt::Trait { methods, .. } => {
                validate_stmts(methods, 0, diagnostics)
            }
            Stmt::Actor { handlers, .. } => validate_stmts(handlers, 0, diagnostics),
            Stmt::CancelToken { inner, .. } => {
                if let Some(inner) = inner.as_deref() {
                    validate_stmts(std::slice::from_ref(inner), loop_depth, diagnostics);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::complete_lexer::CompleteLexer;
    use crate::parser::Parser;

    fn parse(source: &str) -> Program {
        let mut lexer = CompleteLexer::new(source);
        let tokens = lexer.tokenize().expect("lex");
        Parser::new(tokens).parse_program().expect("parse")
    }

    #[test]
    fn rejects_reachable_value_fallthrough() {
        let program = parse("fn main() -> i64 { let x: i64 = 1; }");
        let diagnostics = validate(&program);
        assert!(diagnostics
            .iter()
            .any(|d| d.code == error_codes::TYPE_MISSING_RETURN));
    }

    #[test]
    fn accepts_explicit_return() {
        let program = parse("fn main() -> i64 { return 42; }");
        let diagnostics = validate(&program);
        assert!(!diagnostics
            .iter()
            .any(|d| d.code == error_codes::TYPE_MISSING_RETURN));
    }

    #[test]
    fn requires_both_if_branches_to_return() {
        let program = parse("fn main() -> i64 { if true { return 1; } else { return 2; } }");
        let diagnostics = validate(&program);
        assert!(!diagnostics
            .iter()
            .any(|d| d.code == error_codes::TYPE_MISSING_RETURN));
    }

    #[test]
    fn rejects_ignored_top_level_execution_next_to_main() {
        let program = parse("print \"ignored\"; fn main() -> i64 { return 0; }");
        let diagnostics = validate(&program);
        assert!(diagnostics
            .iter()
            .any(|d| { d.code == error_codes::TYPE_TOP_LEVEL_EXECUTABLE_WITH_MAIN }));
    }
}
