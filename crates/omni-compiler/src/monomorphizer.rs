//! Concrete generic-specialization (monomorphization) pass.
//!
//! Replaces generic function call sites with specialized concrete instances
//! generated from the generic AST definitions, substituting type annotations
//! and mangling symbols (e.g. `identity__i64`).

use crate::ast::{Expr, Program, Stmt};
use crate::types::Type;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Monomorphizer {
    generic_functions: HashMap<String, Stmt>,
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
        collect_generic_defs(&program.stmts, &mut self.generic_functions);
        if self.generic_functions.is_empty() {
            return Ok(());
        }

        // Collect all call sites to generic functions
        let mut call_sites = Vec::new();
        collect_generic_call_sites(&program.stmts, &self.generic_functions, &mut call_sites);

        if call_sites.is_empty() {
            return Ok(());
        }

        let mut specialized_fns: Vec<Stmt> = Vec::new();
        let mut rewrites: HashMap<String, String> = HashMap::new();

        for (caller_target, arg_count) in &call_sites {
            if let Some(generic_stmt) = self.generic_functions.get(caller_target) {
                if let Stmt::Fn {
                    name,
                    visibility,
                    is_async,
                    type_params,
                    params,
                    ret_type,
                    effects,
                    contracts,
                    body,
                    span,
                } = generic_stmt
                {
                    if params.len() != *arg_count {
                        return Err(format!(
                            "generic function '{}' expects {} arguments, got {}",
                            name,
                            params.len(),
                            arg_count
                        ));
                    }

                    // For v0.3.0 baseline, infer concrete types as i64 for scalar operands
                    let specialized_name = format!("{name}__i64");
                    rewrites.insert(name.clone(), specialized_name.clone());

                    // Specialize parameters: replace generic parameter annotations with "i64"
                    let specialized_params: Vec<(String, Option<String>)> = params
                        .iter()
                        .map(|(pname, ann)| {
                            let new_ann = match ann {
                                Some(a) if type_params.iter().any(|(tp, _)| tp == a) => {
                                    Some("i64".to_string())
                                }
                                other => other.clone(),
                            };
                            (pname.clone(), new_ann)
                        })
                        .collect();

                    let specialized_ret = match ret_type {
                        Some(a) if type_params.iter().any(|(tp, _)| tp == a) => {
                            Some("i64".to_string())
                        }
                        other => other.clone(),
                    };

                    let specialized_fn = Stmt::Fn {
                        name: specialized_name,
                        visibility: visibility.clone(),
                        is_async: *is_async,
                        type_params: vec![], // Specialized instance has no type parameters
                        params: specialized_params,
                        ret_type: specialized_ret,
                        effects: effects.clone(),
                        contracts: contracts.clone(),
                        body: body.clone(),
                        span: span.clone(),
                    };

                    specialized_fns.push(specialized_fn);
                }
            }
        }

        // Rewrite call sites in the AST to target specialized functions
        rewrite_program_calls(&mut program.stmts, &rewrites);

        // Append generated specialized functions to program
        program.stmts.extend(specialized_fns);

        Ok(())
    }
}

fn collect_generic_defs(stmts: &[Stmt], out: &mut HashMap<String, Stmt>) {
    for stmt in stmts {
        if let Stmt::Fn {
            name, type_params, ..
        } = stmt
        {
            if !type_params.is_empty() {
                out.insert(name.clone(), stmt.clone());
            }
        }
    }
}

fn collect_generic_call_sites(
    stmts: &[Stmt],
    generics: &HashMap<String, Stmt>,
    out: &mut Vec<(String, usize)>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Print(expr, _)
            | Stmt::ExprStmt(expr, _)
            | Stmt::Return(expr, _)
            | Stmt::Assign(_, expr, _)
            | Stmt::Let(_, _, expr, _)
            | Stmt::LetMut(_, _, expr, _)
            | Stmt::LetLinear(_, _, expr, _) => collect_expr(expr, generics, out),
            Stmt::Block(body, _)
            | Stmt::Loop { body, .. }
            | Stmt::Unsafe { body, .. } => collect_generic_call_sites(body, generics, out),
            Stmt::Fn { body, .. } => collect_generic_call_sites(body, generics, out),
            Stmt::If {
                cond,
                then_body,
                else_body,
                ..
            } => {
                collect_expr(cond, generics, out);
                collect_generic_call_sites(then_body, generics, out);
                collect_generic_call_sites(else_body, generics, out);
            }
            Stmt::While { cond, body, .. } => {
                collect_expr(cond, generics, out);
                collect_generic_call_sites(body, generics, out);
            }
            _ => {}
        }
    }
}

fn collect_expr(expr: &Expr, generics: &HashMap<String, Stmt>, out: &mut Vec<(String, usize)>) {
    match expr {
        Expr::Call(name, args, _) => {
            if generics.contains_key(name) {
                out.push((name.clone(), args.len()));
            }
            for arg in args {
                collect_expr(arg, generics, out);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_expr(left, generics, out);
            collect_expr(right, generics, out);
        }
        Expr::UnaryOp { inner, .. }
        | Expr::Borrow { inner, .. }
        | Expr::Deref { inner, .. } => collect_expr(inner, generics, out),
        Expr::IfExpr {
            cond, then, else_, ..
        } => {
            collect_expr(cond, generics, out);
            collect_expr(then, generics, out);
            collect_expr(else_, generics, out);
        }
        Expr::Block(stmts, _) => collect_generic_call_sites(stmts, generics, out),
        Expr::Tuple(items, _) | Expr::Array(items, _) => {
            for item in items {
                collect_expr(item, generics, out);
            }
        }
        _ => {}
    }
}

fn rewrite_program_calls(stmts: &mut [Stmt], rewrites: &HashMap<String, String>) {
    for stmt in stmts {
        match stmt {
            Stmt::Print(expr, _)
            | Stmt::ExprStmt(expr, _)
            | Stmt::Return(expr, _)
            | Stmt::Assign(_, expr, _)
            | Stmt::Let(_, _, expr, _)
            | Stmt::LetMut(_, _, expr, _)
            | Stmt::LetLinear(_, _, expr, _) => rewrite_expr_calls(expr, rewrites),
            Stmt::Block(body, _)
            | Stmt::Loop { body, .. }
            | Stmt::Unsafe { body, .. } => rewrite_program_calls(body, rewrites),
            Stmt::Fn { body, .. } => rewrite_program_calls(body, rewrites),
            Stmt::If {
                cond,
                then_body,
                else_body,
                ..
            } => {
                rewrite_expr_calls(cond, rewrites);
                rewrite_program_calls(then_body, rewrites);
                rewrite_program_calls(else_body, rewrites);
            }
            Stmt::While { cond, body, .. } => {
                rewrite_expr_calls(cond, rewrites);
                rewrite_program_calls(body, rewrites);
            }
            _ => {}
        }
    }
}

fn rewrite_expr_calls(expr: &mut Expr, rewrites: &HashMap<String, String>) {
    match expr {
        Expr::Call(name, args, _) => {
            if let Some(target) = rewrites.get(name) {
                *name = target.clone();
            }
            for arg in args {
                rewrite_expr_calls(arg, rewrites);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            rewrite_expr_calls(left, rewrites);
            rewrite_expr_calls(right, rewrites);
        }
        Expr::UnaryOp { inner, .. }
        | Expr::Borrow { inner, .. }
        | Expr::Deref { inner, .. } => rewrite_expr_calls(inner, rewrites),
        Expr::IfExpr {
            cond, then, else_, ..
        } => {
            rewrite_expr_calls(cond, rewrites);
            rewrite_expr_calls(then, rewrites);
            rewrite_expr_calls(else_, rewrites);
        }
        Expr::Block(stmts, _) => rewrite_program_calls(stmts, rewrites),
        Expr::Tuple(items, _) | Expr::Array(items, _) => {
            for item in items {
                rewrite_expr_calls(item, rewrites);
            }
        }
        _ => {}
    }
}
