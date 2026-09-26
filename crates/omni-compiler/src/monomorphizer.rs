//! Concrete generic-specialization (monomorphization) pass.
//!
//! Replaces generic function call sites with specialized concrete instances
//! generated from the generic AST definitions, substituting type annotations
//! and mangling symbols (e.g. `identity__i64`).

use crate::ast::{Expr, InterpolatedFragment, Program, Stmt};
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
            if let Some(Stmt::Fn {
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
            }) = self.generic_functions.get(caller_target)
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
                    Some(a) if type_params.iter().any(|(tp, _)| tp == a) => Some("i64".to_string()),
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
            Stmt::Annotation(..)
            | Stmt::Mod(..)
            | Stmt::Struct { .. }
            | Stmt::Enum { .. }
            | Stmt::ErrorSet { .. }
            | Stmt::Break(_)
            | Stmt::Continue(_)
            | Stmt::TypeAlias { .. }
            | Stmt::Use { .. }
            | Stmt::GcMode { .. }
            | Stmt::Channel { .. }
            | Stmt::WorkStealingExecutor { .. }
            | Stmt::DeterministicRuntime { .. }
            | Stmt::Tensor { .. }
            | Stmt::Simd { .. }
            | Stmt::DocComment { .. }
            | Stmt::DebugSession { .. }
            | Stmt::Capability { .. }
            | Stmt::FfiSandbox { .. }
            | Stmt::ComptimeLimit { .. } => {}
            Stmt::Print(expr, _)
            | Stmt::ExprStmt(expr, _)
            | Stmt::Return(expr, _)
            | Stmt::Assign(_, expr, _) => collect_expr(expr, generics, out),
            Stmt::Let(_, _, expr, _)
            | Stmt::LetMut(_, _, expr, _)
            | Stmt::LetLinear(_, _, expr, _) => collect_expr(expr, generics, out),
            Stmt::Block(body, _) | Stmt::Loop { body, .. } | Stmt::Unsafe { body, .. } => {
                collect_generic_call_sites(body, generics, out)
            }
            Stmt::ModBlock(_, body, _) => collect_generic_call_sites(body, generics, out),
            Stmt::Fn { contracts, body, .. } => {
                collect_generic_call_sites(contracts, generics, out);
                collect_generic_call_sites(body, generics, out);
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
                collect_generic_call_sites(then_body, generics, out);
                collect_generic_call_sites(else_body, generics, out);
            }
            Stmt::For { iterable, body, .. } | Stmt::WhileIn { iterable, body, .. } => {
                collect_expr(iterable, generics, out);
                collect_generic_call_sites(body, generics, out);
            }
            Stmt::While { cond, body, .. } => {
                collect_expr(cond, generics, out);
                collect_generic_call_sites(body, generics, out);
            }
            Stmt::Defer { cleanup, .. } | Stmt::AsyncDefer { cleanup, .. } => {
                collect_generic_call_sites(std::slice::from_ref(cleanup.as_ref()), generics, out)
            }
            Stmt::ExprFieldAssign(base, _, expr, _)
            | Stmt::DerefAssign(base, expr, _) => {
                collect_expr(base, generics, out);
                collect_expr(expr, generics, out);
            }
            Stmt::CancelToken { inner, .. } => {
                if let Some(inner) = inner {
                    collect_generic_call_sites(std::slice::from_ref(inner.as_ref()), generics, out);
                }
            }
            Stmt::EffectHandler { handler, .. } | Stmt::Spawn { task: handler, .. } => {
                collect_expr(handler, generics, out)
            }
            Stmt::Impl { methods, .. } | Stmt::Trait { methods, .. } => {
                collect_generic_call_sites(methods, generics, out)
            }
            Stmt::UseScoped { body, .. } => collect_generic_call_sites(body, generics, out),
            Stmt::Actor { handlers, .. } => collect_generic_call_sites(handlers, generics, out),
            Stmt::ContractRequires { condition, .. }
            | Stmt::ContractEnsures { condition, .. }
            | Stmt::ContractInvariant { condition, .. } => collect_expr(condition, generics, out),
        }
    }
}

fn collect_expr(expr: &Expr, generics: &HashMap<String, Stmt>, out: &mut Vec<(String, usize)>) {
    match expr {
        Expr::StringLit(..)
        | Expr::ByteString(..)
        | Expr::Byte(..)
        | Expr::Number(..)
        | Expr::Float(..)
        | Expr::Char(..)
        | Expr::Var(..)
        | Expr::Bool(..) => {}
        Expr::Interpolated(fragments, _) => {
            for fragment in fragments {
                if let InterpolatedFragment::Expr(inner) = fragment {
                    collect_expr(inner, generics, out);
                }
            }
        }
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
        | Expr::Deref { inner, .. }
        | Expr::Await(inner, _)
        | Expr::Try(inner, _) => collect_expr(inner, generics, out),
        Expr::FieldAccess { base, .. } => collect_expr(base, generics, out),
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
            for (_, field_expr) in fields {
                collect_expr(field_expr, generics, out);
            }
        }
    }
}

fn rewrite_program_calls(stmts: &mut [Stmt], rewrites: &HashMap<String, String>) {
    for stmt in stmts {
        match stmt {
            Stmt::Annotation(..)
            | Stmt::Mod(..)
            | Stmt::Struct { .. }
            | Stmt::Enum { .. }
            | Stmt::ErrorSet { .. }
            | Stmt::Break(_)
            | Stmt::Continue(_)
            | Stmt::TypeAlias { .. }
            | Stmt::Use { .. }
            | Stmt::GcMode { .. }
            | Stmt::Channel { .. }
            | Stmt::WorkStealingExecutor { .. }
            | Stmt::DeterministicRuntime { .. }
            | Stmt::Tensor { .. }
            | Stmt::Simd { .. }
            | Stmt::DocComment { .. }
            | Stmt::DebugSession { .. }
            | Stmt::Capability { .. }
            | Stmt::FfiSandbox { .. }
            | Stmt::ComptimeLimit { .. } => {}
            Stmt::Print(expr, _)
            | Stmt::ExprStmt(expr, _)
            | Stmt::Return(expr, _)
            | Stmt::Assign(_, expr, _)
            | Stmt::Let(_, _, expr, _)
            | Stmt::LetMut(_, _, expr, _)
            | Stmt::LetLinear(_, _, expr, _) => rewrite_expr_calls(expr, rewrites),
            Stmt::Block(body, _) | Stmt::Loop { body, .. } | Stmt::Unsafe { body, .. } => {
                rewrite_program_calls(body, rewrites)
            }
            Stmt::ModBlock(_, body, _) => rewrite_program_calls(body, rewrites),
            Stmt::Fn { contracts, body, .. } => {
                rewrite_program_calls(contracts, rewrites);
                rewrite_program_calls(body, rewrites);
            }
            Stmt::If {
                cond,
                bindings,
                then_body,
                else_body,
                ..
            } => {
                rewrite_expr_calls(cond, rewrites);
                for (_, expr) in bindings {
                    rewrite_expr_calls(expr, rewrites);
                }
                rewrite_program_calls(then_body, rewrites);
                rewrite_program_calls(else_body, rewrites);
            }
            Stmt::For {
                iterable, body, ..
            }
            | Stmt::WhileIn {
                iterable, body, ..
            } => {
                rewrite_expr_calls(iterable, rewrites);
                rewrite_program_calls(body, rewrites);
            }
            Stmt::While { cond, body, .. } => {
                rewrite_expr_calls(cond, rewrites);
                rewrite_program_calls(body, rewrites);
            }
            Stmt::Defer { cleanup, .. } | Stmt::AsyncDefer { cleanup, .. } => {
                rewrite_program_calls(std::slice::from_mut(cleanup.as_mut()), rewrites)
            }
            Stmt::ExprFieldAssign(base, _, expr, _)
            | Stmt::DerefAssign(base, expr, _) => {
                rewrite_expr_calls(base, rewrites);
                rewrite_expr_calls(expr, rewrites);
            }
            Stmt::CancelToken { inner, .. } => {
                if let Some(inner) = inner {
                    rewrite_program_calls(std::slice::from_mut(inner.as_mut()), rewrites);
                }
            }
            Stmt::EffectHandler { handler, .. } | Stmt::Spawn { task: handler, .. } => {
                rewrite_expr_calls(handler, rewrites)
            }
            Stmt::Impl { methods, .. } | Stmt::Trait { methods, .. } => {
                rewrite_program_calls(methods, rewrites)
            }
            Stmt::UseScoped { body, .. } => rewrite_program_calls(body, rewrites),
            Stmt::Actor { handlers, .. } => rewrite_program_calls(handlers, rewrites),
            Stmt::ContractRequires { condition, .. }
            | Stmt::ContractEnsures { condition, .. }
            | Stmt::ContractInvariant { condition, .. } => rewrite_expr_calls(condition, rewrites),
        }
    }
}

fn rewrite_expr_calls(expr: &mut Expr, rewrites: &HashMap<String, String>) {
    match expr {
        Expr::StringLit(..)
        | Expr::ByteString(..)
        | Expr::Byte(..)
        | Expr::Number(..)
        | Expr::Float(..)
        | Expr::Char(..)
        | Expr::Var(..)
        | Expr::Bool(..) => {}
        Expr::Interpolated(fragments, _) => {
            for fragment in fragments {
                if let InterpolatedFragment::Expr(inner) = fragment {
                    rewrite_expr_calls(inner, rewrites);
                }
            }
        }
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
        | Expr::Deref { inner, .. }
        | Expr::Await(inner, _)
        | Expr::Try(inner, _) => rewrite_expr_calls(inner, rewrites),
        Expr::FieldAccess { base, .. } => rewrite_expr_calls(base, rewrites),
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
        Expr::Index(base, index, _) => {
            rewrite_expr_calls(base, rewrites);
            rewrite_expr_calls(index, rewrites);
        }
        Expr::Match { expr, arms, .. } => {
            rewrite_expr_calls(expr, rewrites);
            for arm in arms {
                if let Some(guard) = &mut arm.guard {
                    rewrite_expr_calls(guard, rewrites);
                }
                rewrite_expr_calls(&mut arm.body, rewrites);
            }
        }
        Expr::Range { start, end, .. } => {
            rewrite_expr_calls(start, rewrites);
            rewrite_expr_calls(end, rewrites);
        }
        Expr::Lambda { body, .. } => rewrite_expr_calls(body, rewrites),
        Expr::StructLit { fields, .. } => {
            for (_, field_expr) in fields {
                rewrite_expr_calls(field_expr, rewrites);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expr, InterpolatedFragment, Program, Stmt, Visibility};
    use crate::diagnostics::Span;
    use std::collections::HashMap;

    fn span() -> Span {
        Span::new(1, 1, 1, 1)
    }

    fn identity_stmt() -> Stmt {
        Stmt::Fn {
            name: "identity".to_string(),
            visibility: Visibility::Private,
            is_async: false,
            type_params: vec![("T".to_string(), vec![])],
            params: vec![("value".to_string(), Some("T".to_string()))],
            ret_type: Some("T".to_string()),
            effects: vec![],
            contracts: vec![],
            body: vec![Stmt::Return(Expr::Var("value".to_string(), span()), span())],
            span: span(),
        }
    }

    fn identity_call() -> Expr {
        Expr::Call(
            "identity".to_string(),
            vec![Expr::Number(42, span())],
            span(),
        )
    }

    fn specialize_program(stmt: Stmt) -> Program {
        let mut program = Program {
            stmts: vec![identity_stmt(), stmt],
        };
        Monomorphizer::new()
            .specialize(&mut program, &HashMap::new())
            .expect("generic specialization must succeed");
        program
    }

    #[test]
    fn specializes_calls_in_extended_statement_positions() {
        let nested = Stmt::For {
            var_name: "x".to_string(),
            iterable: Box::new(identity_call()),
            body: vec![Stmt::WhileIn {
                var_name: "y".to_string(),
                iterable: Box::new(Expr::Index(
                    Box::new(Expr::StructLit {
                        name: "S".to_string(),
                        fields: vec![("value".to_string(), identity_call())],
                        span: span(),
                    }),
                    Box::new(Expr::Range {
                        start: Box::new(identity_call()),
                        end: Box::new(identity_call()),
                        inclusive: false,
                        span: span(),
                    }),
                    span(),
                )),
                body: vec![Stmt::ExprFieldAssign(
                    Box::new(Expr::FieldAccess {
                        base: Box::new(identity_call()),
                        field: "value".to_string(),
                        span: span(),
                    }),
                    "value".to_string(),
                    identity_call(),
                    span(),
                )],
                span: span(),
            }],
            span: span(),
        };
        let program = specialize_program(nested);
        let formatted = format!("{:?}", program.stmts[1]);
        assert!(
            !formatted.contains(r#"Call("identity","#),
            "unrewritten call: {formatted}"
        );
        assert!(
            formatted.matches(r#"Call("identity__i64""#).count() >= 6,
            "expected all nested generic calls to be rewritten: {formatted}"
        );
    }

    #[test]
    fn specializes_calls_in_interpolated_match_and_lambda_expressions() {
        let stmt = Stmt::ExprStmt(
            Expr::Match {
                expr: Box::new(identity_call()),
                arms: vec![crate::ast::MatchArm {
                    pattern: crate::ast::Pattern::Wildcard,
                    guard: Some(Box::new(identity_call())),
                    body: Box::new(Expr::Lambda {
                        params: vec![],
                        body: Box::new(Expr::Interpolated(
                            vec![InterpolatedFragment::Expr(Box::new(identity_call()))],
                            span(),
                        )),
                        span: span(),
                    }),
                    span: span(),
                }],
                span: span(),
            },
            span(),
        );
        let program = specialize_program(stmt);
        let formatted = format!("{:?}", program.stmts[1]);
        assert!(!formatted.contains(r#"Call("identity","#));
        assert_eq!(
            formatted.matches(r#"Call("identity__i64""#).count(),
            3,
            "expected match, guard, interpolation calls to rewrite: {formatted}"
        );
    }
}