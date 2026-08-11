//! Conservative generic-specialization gate.
//!
//! The parser and type checker already understand generic declarations, but
//! the machine-code pipeline is not yet representation-polymorphic. Until the
//! real monomorphization pass lands, this module prevents a generic call from
//! silently reaching code generation with an invented ABI. Uncalled generic
//! declarations remain legal so libraries can be parsed and type-checked.

use crate::ast::{Expr, Program, Stmt};
use crate::types::Type;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default)]
pub struct Monomorphizer {
    generic_functions: HashSet<String>,
}

impl Monomorphizer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn specialize(
        &mut self,
        program: &mut Program,
        _type_map: &HashMap<String, Type>,
    ) -> Result<(), String> {
        self.generic_functions.clear();
        collect_generic_functions(&program.stmts, &mut self.generic_functions);
        if self.generic_functions.is_empty() {
            return Ok(());
        }

        let mut calls = Vec::new();
        collect_generic_calls(&program.stmts, &self.generic_functions, &mut calls);
        calls.sort();
        calls.dedup();
        if calls.is_empty() {
            return Ok(());
        }

        Err(format!(
            "generic calls require the upcoming concrete-specialization pass; refusing to miscompile: {}",
            calls.join(", ")
        ))
    }
}

fn collect_generic_functions(stmts: &[Stmt], out: &mut HashSet<String>) {
    for stmt in stmts {
        match stmt {
            Stmt::Fn {
                name,
                type_params,
                body,
                ..
            } => {
                if !type_params.is_empty() {
                    out.insert(name.clone());
                }
                collect_generic_functions(body, out);
            }
            Stmt::Block(body, _)
            | Stmt::Loop { body, .. }
            | Stmt::For { body, .. }
            | Stmt::While { body, .. }
            | Stmt::WhileIn { body, .. }
            | Stmt::Unsafe { body, .. } => collect_generic_functions(body, out),
            Stmt::If {
                then_body,
                else_body,
                ..
            } => {
                collect_generic_functions(then_body, out);
                collect_generic_functions(else_body, out);
            }
            Stmt::ModBlock(_, body, _) | Stmt::UseScoped { body, .. } => {
                collect_generic_functions(body, out)
            }
            Stmt::Impl { methods, .. } | Stmt::Trait { methods, .. } => {
                collect_generic_functions(methods, out)
            }
            Stmt::Actor { handlers, .. } => collect_generic_functions(handlers, out),
            _ => {}
        }
    }
}

fn collect_generic_calls(stmts: &[Stmt], generics: &HashSet<String>, out: &mut Vec<String>) {
    for stmt in stmts {
        match stmt {
            Stmt::Print(expr, _)
            | Stmt::ExprStmt(expr, _)
            | Stmt::Return(expr, _)
            | Stmt::Assign(_, expr, _)
            | Stmt::Let(_, _, expr, _)
            | Stmt::LetLinear(_, _, expr, _) => collect_expr(expr, generics, out),
            Stmt::Spawn { task, .. } => collect_expr(task.as_ref(), generics, out),
            Stmt::ContractRequires { condition, .. }
            | Stmt::ContractEnsures { condition, .. }
            | Stmt::ContractInvariant { condition, .. } => collect_expr(condition, generics, out),
            Stmt::ExprFieldAssign(base, _, value, _) => {
                collect_expr(base, generics, out);
                collect_expr(value, generics, out);
            }
            Stmt::Block(body, _)
            | Stmt::Loop { body, .. }
            | Stmt::Unsafe { body, .. }
            | Stmt::ModBlock(_, body, _)
            | Stmt::UseScoped { body, .. } => collect_generic_calls(body, generics, out),
            Stmt::Fn {
                body, contracts, ..
            } => {
                collect_generic_calls(contracts, generics, out);
                collect_generic_calls(body, generics, out);
            }
            Stmt::If {
                cond,
                bindings,
                then_body,
                else_body,
                ..
            } => {
                collect_expr(cond, generics, out);
                for (_, expr) in bindings {
                    collect_expr(expr, generics, out);
                }
                collect_generic_calls(then_body, generics, out);
                collect_generic_calls(else_body, generics, out);
            }
            Stmt::For { iterable, body, .. } | Stmt::WhileIn { iterable, body, .. } => {
                collect_expr(iterable, generics, out);
                collect_generic_calls(body, generics, out);
            }
            Stmt::While { cond, body, .. } => {
                collect_expr(cond, generics, out);
                collect_generic_calls(body, generics, out);
            }
            Stmt::Impl { methods, .. } | Stmt::Trait { methods, .. } => {
                collect_generic_calls(methods, generics, out)
            }
            Stmt::EffectHandler { handler, .. } => collect_expr(handler, generics, out),
            Stmt::CancelToken { inner, .. } => {
                if let Some(inner) = inner.as_deref() {
                    collect_generic_calls(std::slice::from_ref(inner), generics, out);
                }
            }
            Stmt::Actor { handlers, .. } => collect_generic_calls(handlers, generics, out),
            _ => {}
        }
    }
}

fn collect_expr(expr: &Expr, generics: &HashSet<String>, out: &mut Vec<String>) {
    match expr {
        Expr::Call(name, args, _) => {
            if generics.contains(name) {
                out.push(name.clone());
            }
            for arg in args {
                collect_expr(arg, generics, out);
            }
        }
        Expr::Interpolated(parts, _) => {
            for part in parts {
                if let crate::ast::InterpolatedFragment::Expr(expr) = part {
                    collect_expr(expr, generics, out);
                }
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_expr(left, generics, out);
            collect_expr(right, generics, out);
        }
        Expr::UnaryOp { inner, .. }
        | Expr::Borrow { inner, .. }
        | Expr::Deref { inner, .. }
        | Expr::Await(inner, _)
        | Expr::Try(inner, _)
        | Expr::FieldAccess { base: inner, .. } => collect_expr(inner, generics, out),
        Expr::IfExpr {
            cond, then, else_, ..
        } => {
            collect_expr(cond, generics, out);
            collect_expr(then, generics, out);
            collect_expr(else_, generics, out);
        }
        Expr::Block(stmts, _) => collect_generic_calls(stmts, generics, out),
        Expr::Tuple(items, _) | Expr::Array(items, _) => {
            for item in items {
                collect_expr(item, generics, out);
            }
        }
        Expr::Index(base, index, _) => {
            collect_expr(base, generics, out);
            collect_expr(index, generics, out);
        }
        Expr::Match { expr, arms, .. } => {
            collect_expr(expr, generics, out);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    collect_expr(guard, generics, out);
                }
                collect_expr(&arm.body, generics, out);
            }
        }
        Expr::Range { start, end, .. } => {
            collect_expr(start, generics, out);
            collect_expr(end, generics, out);
        }
        Expr::Lambda { body, .. } => collect_expr(body, generics, out),
        Expr::StructLit { fields, .. } => {
            for (_, value) in fields {
                collect_expr(value, generics, out);
            }
        }
        Expr::StringLit(_, _)
        | Expr::ByteString(_, _)
        | Expr::Byte(_, _)
        | Expr::Number(_, _)
        | Expr::Float(_, _)
        | Expr::Char(_, _)
        | Expr::Var(_, _)
        | Expr::Bool(_, _) => {}
    }
}
