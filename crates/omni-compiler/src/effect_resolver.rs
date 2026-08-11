use crate::ast::{Expr, Program, Stmt};
use crate::diagnostics::{error_codes, Diagnostic};
use crate::types::{Effect, EffectSet};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct EffectResolver {
    pub function_effects: HashMap<String, EffectSet>,
    pub call_graph: HashMap<String, Vec<String>>,
}

impl Default for EffectResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectResolver {
    pub fn new() -> Self {
        Self {
            function_effects: HashMap::new(),
            call_graph: HashMap::new(),
        }
    }

    pub fn resolve(&mut self, prog: &Program) -> Result<(), Diagnostic> {
        self.collect_initial_effects(prog);
        self.build_call_graph(prog);
        self.propagate_effects();
        self.verify_entry_point(prog)
    }

    fn collect_initial_effects(&mut self, prog: &Program) {
        for stmt in &prog.stmts {
            if let Stmt::Fn { name, effects, .. } = stmt {
                let mut ef_set = EffectSet::new();
                for e in effects {
                    match e.as_str() {
                        "io" => ef_set.add(Effect::Io),
                        "async" => ef_set.add(Effect::Async),
                        "panic" => ef_set.add(Effect::Panic),
                        "pure" => ef_set.add(Effect::Pure),
                        custom => ef_set.add(Effect::Custom(custom.to_string())),
                    }
                }
                self.function_effects.insert(name.clone(), ef_set);
            }
        }
    }

    fn build_call_graph(&mut self, prog: &Program) {
        for stmt in &prog.stmts {
            if let Stmt::Fn { name, body, .. } = stmt {
                let mut called = Vec::new();
                self.find_calls(body, &mut called);
                self.call_graph.insert(name.clone(), called);
            }
        }
    }

    fn find_calls(&self, stmts: &[Stmt], called: &mut Vec<String>) {
        for stmt in stmts {
            match stmt {
                Stmt::ExprStmt(expr, _) => self.find_expr_calls(expr, called),
                Stmt::Let(_, _, expr, _) | Stmt::LetLinear(_, _, expr, _) => {
                    self.find_expr_calls(expr, called)
                }
                Stmt::Print(expr, _) => self.find_expr_calls(expr, called),
                Stmt::Return(expr, _) => self.find_expr_calls(expr, called),
                Stmt::Assign(_, expr, _) => self.find_expr_calls(expr, called),
                Stmt::ExprFieldAssign(base, _, expr, _) => {
                    self.find_expr_calls(base, called);
                    self.find_expr_calls(expr, called);
                }
                Stmt::Block(inner, _) => self.find_calls(inner, called),
                Stmt::Loop { body, .. }
                | Stmt::While { body, .. }
                | Stmt::For { body, .. }
                | Stmt::WhileIn { body, .. }
                | Stmt::Unsafe { body, .. } => self.find_calls(body, called),
                Stmt::If {
                    then_body,
                    else_body,
                    ..
                } => {
                    self.find_calls(then_body, called);
                    self.find_calls(else_body, called);
                }
                Stmt::Fn { body, .. } => self.find_calls(body, called),
                _ => {}
            }
        }
    }

    fn find_expr_calls(&self, expr: &Expr, called: &mut Vec<String>) {
        match expr {
            Expr::Call(name, args, _) => {
                called.push(name.clone());
                for arg in args {
                    self.find_expr_calls(arg, called);
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                self.find_expr_calls(left, called);
                self.find_expr_calls(right, called);
            }
            Expr::UnaryOp { inner, .. } => self.find_expr_calls(inner, called),
            Expr::IfExpr {
                cond, then, else_, ..
            } => {
                self.find_expr_calls(cond, called);
                self.find_expr_calls(then, called);
                self.find_expr_calls(else_, called);
            }
            Expr::Block(stmts, _) => {
                let mut inner_called = Vec::new();
                self.find_calls(stmts, &mut inner_called);
                called.extend(inner_called);
            }
            Expr::Await(inner, _) => self.find_expr_calls(inner, called),
            Expr::Try(inner, _) => self.find_expr_calls(inner, called),
            Expr::FieldAccess { base, .. } => self.find_expr_calls(base, called),
            Expr::Index(base, index, _) => {
                self.find_expr_calls(base, called);
                self.find_expr_calls(index, called);
            }
            Expr::Tuple(items, _) => {
                for item in items {
                    self.find_expr_calls(item, called);
                }
            }
            Expr::Match { expr, arms, .. } => {
                self.find_expr_calls(expr, called);
                for arm in arms {
                    self.find_expr_calls(&arm.body, called);
                }
            }
            Expr::Range { start, end, .. } => {
                self.find_expr_calls(start, called);
                self.find_expr_calls(end, called);
            }
            Expr::Lambda { body, .. } => self.find_expr_calls(body, called),
            Expr::StructLit { fields, .. } => {
                for (_, expr) in fields {
                    self.find_expr_calls(expr, called);
                }
            }
            Expr::Interpolated(frags, _) => {
                for frag in frags {
                    if let crate::ast::InterpolatedFragment::Expr(inner) = frag {
                        self.find_expr_calls(inner, called);
                    }
                }
            }
            _ => {}
        }
    }

    fn propagate_effects(&mut self) {
        let mut changed = true;
        while changed {
            changed = false;
            let current_effects = self.function_effects.clone();
            for (func, calls) in &self.call_graph {
                if let Some(ef_set) = self.function_effects.get_mut(func) {
                    for callee in calls {
                        if let Some(callee_ef) = current_effects.get(callee) {
                            let pre_len = ef_set.to_string_list().len();
                            ef_set.union_with(callee_ef);
                            if ef_set.to_string_list().len() != pre_len {
                                changed = true;
                            }
                        }
                    }
                }
            }
        }
    }

    fn verify_entry_point(&self, _prog: &Program) -> Result<(), Diagnostic> {
        if let Some(main_ef) = self.function_effects.get("main") {
            let unhandled = main_ef.non_pure_effect_strings();
            if !unhandled.is_empty() {
                Err(Diagnostic::error(
                    error_codes::TYPE_EFFECT_MISMATCH,
                    format!(
                        "entry point 'main' has unhandled effects: {}",
                        unhandled.join(", ")
                    ),
                ))
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }
}
