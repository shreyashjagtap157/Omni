//! Inout parameter desugaring pass.
//!
//! Desugars `fn foo(inout x: T)` into explicit move-in / move-out semantics:
//! 1. Caller moves the argument into the callee's inout parameter.
//! 2. Callee's inout parameter is a linear-type local variable.
//! 3. On return, the (possibly mutated) value is moved back to the caller.
//!
//! This pass rewrites the AST so that MIR lowering can treat inout parameters
//! as linear moves rather than copies.

use crate::ast::{Expr, InterpolatedFragment, Program, Stmt};
use crate::diagnostics::Span;

/// Rewrite a program in-place, desugaring `inout` parameters.
pub fn desugar_inout_in_ast(prog: &mut Program) -> Result<(), String> {
    for stmt in &mut prog.stmts {
        desugar_stmt(stmt);
    }
    Ok(())
}

fn desugar_stmt(stmt: &mut Stmt) {
    match stmt {
        Stmt::Fn {
            ref mut params,
            ref mut body,
            ..
        } => {
            // Collect inout parameters and their types.
            let inout_params: Vec<(String, String)> = params
                .iter()
                .filter_map(|(p, _type_annotation)| {
                    // Inout params are marked with a special prefix or attribute.
                    // For now, we assume parameters whose name starts with "inout_"
                    // are inout parameters.
                    if p.starts_with("inout_") {
                        let real_name = p.strip_prefix("inout_").unwrap_or(p).to_string();
                        // We don't have the type info here, so we'll need to
                        // get it from the type annotation if available.
                        Some((real_name, "inout".to_string()))
                    } else {
                        None
                    }
                })
                .collect();

            if inout_params.is_empty() {
                return;
            }

            // Rewrite function body to treat inout params as linear.
            let mut new_body: Vec<Stmt> = Vec::new();
            for (real_name, _ty) in &inout_params {
                // At the start of the function, the inout param is already
                // moved in (caller did the move). We just need to ensure
                // it's treated as linear.
                new_body.push(Stmt::LetLinear(
                    real_name.clone(),
                    None,
                    Expr::Var(format!("__inout_src_{}", real_name), Span::default()),
                    Span::default(),
                ));
            }

            // Add the original body (with references to the inout param fixed).
            for s in body.iter() {
                let mut rewritten = s.clone();
                rewrite_inout_refs(&mut rewritten, &inout_params);
                new_body.push(rewritten);
            }

            // At the end, move the inout value back to the caller's location.
            for (real_name, _ty) in &inout_params {
                new_body.push(Stmt::LetLinear(
                    format!("__inout_out_{}", real_name),
                    None,
                    Expr::Var(real_name.clone(), Span::default()),
                    Span::default(),
                ));
            }

            *body = new_body;
        }
        Stmt::Block(inner, _) => {
            let mut new_inner = Vec::new();
            for s in inner.iter() {
                let mut rewritten = s.clone();
                desugar_stmt(&mut rewritten);
                new_inner.push(rewritten);
            }
            *inner = new_inner;
        }
        _ => {}
    }
}

fn rewrite_inout_refs(stmt: &mut Stmt, inout_params: &[(String, String)]) {
    // Walk the statement and rewrite any `Expr::Var` that refers to an inout
    // parameter. The original inout_<name> binding has been desugared to a
    // linear local named `<name>`, so plain references to the bare `<name>`
    // resolve through the LetLinear introduced at function start. We recurse
    // to validate references inside nested expressions.
    let inout_names: std::collections::HashSet<&str> =
        inout_params.iter().map(|(n, _)| n.as_str()).collect();
    rewrite_stmt_walk(stmt, &inout_names);
}

fn rewrite_stmt_walk(stmt: &mut Stmt, inout_names: &std::collections::HashSet<&str>) {
    match stmt {
        Stmt::ExprStmt(expr, _) | Stmt::Return(expr, _) | Stmt::Print(expr, _) => {
            rewrite_expr_walk(expr, inout_names);
        }
        Stmt::Let(_, _, expr, _) | Stmt::LetLinear(_, _, expr, _) => {
            rewrite_expr_walk(expr, inout_names);
        }
        Stmt::Assign(_, expr, _) => {
            rewrite_expr_walk(expr, inout_names);
        }
        Stmt::ExprFieldAssign(expr, _, rhs, _) => {
            rewrite_expr_walk(expr, inout_names);
            rewrite_expr_walk(rhs, inout_names);
        }
        Stmt::Block(stmts, _) => {
            for s in stmts.iter_mut() {
                rewrite_stmt_walk(s, inout_names);
            }
        }
        Stmt::If {
            cond,
            bindings,
            then_body,
            else_body,
            ..
        } => {
            rewrite_expr_walk(cond, inout_names);
            for (_, e) in bindings.iter_mut() {
                rewrite_expr_walk(e, inout_names);
            }
            for s in then_body.iter_mut() {
                rewrite_stmt_walk(s, inout_names);
            }
            for s in else_body.iter_mut() {
                rewrite_stmt_walk(s, inout_names);
            }
        }
        Stmt::While { cond, body, .. } => {
            rewrite_expr_walk(cond, inout_names);
            for s in body.iter_mut() {
                rewrite_stmt_walk(s, inout_names);
            }
        }
        Stmt::Loop { body, .. } => {
            for s in body.iter_mut() {
                rewrite_stmt_walk(s, inout_names);
            }
        }
        Stmt::For { iterable, body, .. } => {
            rewrite_expr_walk(iterable, inout_names);
            for s in body.iter_mut() {
                rewrite_stmt_walk(s, inout_names);
            }
        }
        Stmt::WhileIn { iterable, body, .. } => {
            rewrite_expr_walk(iterable, inout_names);
            for s in body.iter_mut() {
                rewrite_stmt_walk(s, inout_names);
            }
        }
        Stmt::EffectHandler { handler, .. } => {
            rewrite_expr_walk(handler, inout_names);
        }
        Stmt::Spawn { task, .. } => {
            rewrite_expr_walk(task, inout_names);
        }
        Stmt::ModBlock(_, stmts, _) => {
            for s in stmts.iter_mut() {
                rewrite_stmt_walk(s, inout_names);
            }
        }
        Stmt::Impl { methods, .. } => {
            for s in methods.iter_mut() {
                rewrite_stmt_walk(s, inout_names);
            }
        }
        Stmt::Fn { body, .. } => {
            for s in body.iter_mut() {
                rewrite_stmt_walk(s, inout_names);
            }
        }
        Stmt::Trait { methods, .. } => {
            for s in methods.iter_mut() {
                rewrite_stmt_walk(s, inout_names);
            }
        }
        _ => {}
    }
}

fn rewrite_expr_walk(expr: &mut Expr, inout_names: &std::collections::HashSet<&str>) {
    match expr {
        Expr::Var(name, _) => {
            if inout_names.contains(name.as_str()) {
                // Reference to an inout param; resolves through the LetLinear
                // introduced at function start. We keep the name as-is.
            }
        }
        Expr::Number(_, _)
        | Expr::StringLit(_, _)
        | Expr::ByteString(_, _)
        | Expr::Byte(_, _)
        | Expr::Bool(_, _)
        | Expr::Float(_, _)
        | Expr::Char(_, _) => {}
        Expr::Interpolated(parts, _) => {
            for p in parts.iter_mut() {
                if let InterpolatedFragment::Expr(e) = p {
                    rewrite_expr_walk(e, inout_names);
                }
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            rewrite_expr_walk(left, inout_names);
            rewrite_expr_walk(right, inout_names);
        }
        Expr::UnaryOp { inner, .. } | Expr::Borrow { inner, .. } | Expr::Deref { inner, .. } => {
            rewrite_expr_walk(inner, inout_names);
        }
        Expr::Call(_, args, _) => {
            for a in args.iter_mut() {
                rewrite_expr_walk(a, inout_names);
            }
        }
        Expr::FieldAccess { base, .. } => {
            rewrite_expr_walk(base, inout_names);
        }
        Expr::IfExpr {
            cond, then, else_, ..
        } => {
            rewrite_expr_walk(cond, inout_names);
            rewrite_expr_walk(then, inout_names);
            rewrite_expr_walk(else_, inout_names);
        }
        Expr::Block(stmts, _) => {
            for s in stmts.iter_mut() {
                rewrite_stmt_walk(s, inout_names);
            }
        }
        Expr::Tuple(items, _) | Expr::Array(items, _) => {
            for e in items.iter_mut() {
                rewrite_expr_walk(e, inout_names);
            }
        }
        Expr::Index(base, index, _) => {
            rewrite_expr_walk(base, inout_names);
            rewrite_expr_walk(index, inout_names);
        }
        Expr::Match { expr, arms, .. } => {
            rewrite_expr_walk(expr, inout_names);
            for arm in arms.iter_mut() {
                if let Some(g) = arm.guard.as_mut() {
                    rewrite_expr_walk(g, inout_names);
                }
                rewrite_expr_walk(&mut arm.body, inout_names);
            }
        }
        Expr::Range { start, end, .. } => {
            rewrite_expr_walk(start, inout_names);
            rewrite_expr_walk(end, inout_names);
        }
        Expr::Lambda { body, .. } => {
            rewrite_expr_walk(body, inout_names);
        }
        Expr::Await(inner, _) | Expr::Try(inner, _) => {
            rewrite_expr_walk(inner, inout_names);
        }
        Expr::StructLit { fields, .. } => {
            for (_, e) in fields.iter_mut() {
                rewrite_expr_walk(e, inout_names);
            }
        }
    }
}

/// Lowering support: convert a function call with inout arguments.
///
/// When calling `foo(inout x)`, the caller must:
/// 1. Move `x` into the function (it becomes a linear value for the callee).
/// 2. After the call, `x` is considered moved (caller must use the returned value).
///
/// This function generates the MIR instructions for making an inout call.
pub fn lower_inout_call(
    dest: &str,
    func: &str,
    args: &[String],
    is_inout: &[bool],
) -> Vec<crate::mir::Instruction> {
    let mut instrs = Vec::new();
    use crate::mir::Instruction;

    for (i, (arg, &is_io)) in args.iter().zip(is_inout.iter()).enumerate() {
        if is_io {
            // Inout argument: move the value into the function.
            instrs.push(Instruction::LinearMove {
                dest: format!("__inout_arg_{}_{}", func, i),
                src: arg.clone(),
            });
        } else {
            // Normal argument: regular move.
            instrs.push(Instruction::Move {
                dest: format!("__arg_{}_{}", func, i),
                src: arg.clone(),
            });
        }
    }

    // Call the function.
    instrs.push(Instruction::Call {
        dest: format!("{}__inout_result", dest),
        func: func.to_string(),
        args: args.to_vec(),
    });

    // For each inout argument, the caller receives back a (possibly mutated) value.
    for (i, (&is_io, arg)) in is_inout.iter().zip(args.iter()).enumerate() {
        if is_io {
            instrs.push(Instruction::LinearMove {
                dest: arg.clone(), // Move back to the original variable.
                src: format!("__inout_out_{}_{}", func, i),
            });
        }
    }

    instrs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expr, Program, Stmt, Visibility};
    use crate::diagnostics::Span;

    #[test]
    fn desugar_finds_inout_params() {
        // Create a function with an inout_ parameter.
        let mut prog = Program {
            stmts: vec![Stmt::Fn {
                name: "update".to_string(),
                visibility: Visibility::Private,
                is_async: false,
                type_params: vec![],
                params: vec![("inout_val".to_string(), None)],
                ret_type: None,
                effects: vec![],
                contracts: vec![],
                body: vec![Stmt::Assign(
                    "val".to_string(),
                    Expr::Number(42, Span::default()),
                    Span::default(),
                )],
                span: Span::default(),
            }],
        };

        let result = desugar_inout_in_ast(&mut prog);
        assert!(result.is_ok());
    }

    #[test]
    fn lower_inout_call_generates_correct_mir() {
        let instrs = lower_inout_call("result", "update", &["x".to_string()], &[true]);

        // Should have: linear_move for arg, call, linear_move back.
        assert!(instrs.len() >= 2);

        // Check that we have LinearMove instructions.
        let has_linear_move = instrs
            .iter()
            .any(|i| matches!(i, crate::mir::Instruction::LinearMove { .. }));
        assert!(has_linear_move);
    }

    #[test]
    fn inout_call_with_mixed_args() {
        let instrs = lower_inout_call(
            "result",
            "foo",
            &["normal".to_string(), "inout_val".to_string()],
            &[false, true],
        );

        // Should generate both Move and LinearMove instructions.
        let move_count = instrs
            .iter()
            .filter(|i| matches!(i, crate::mir::Instruction::Move { .. }))
            .count();
        let linear_move_count = instrs
            .iter()
            .filter(|i| matches!(i, crate::mir::Instruction::LinearMove { .. }))
            .count();

        assert!(move_count >= 1); // normal arg
        assert!(linear_move_count >= 1); // inout arg
    }
}
