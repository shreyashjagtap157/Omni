use crate::ast::{Expr, Program, Stmt};
use std::collections::HashMap;

pub type DefId = usize;

#[derive(Debug)]
pub struct ResolveResult {
    pub symbols: HashMap<String, DefId>,
}

pub fn resolve_program(prog: &Program) -> Result<ResolveResult, Vec<String>> {
    let mut scopes: Vec<HashMap<String, DefId>> = vec![HashMap::new()];
    let mut errors: Vec<String> = Vec::new();

    for s in &prog.stmts {
        if let Stmt::Fn { name, .. } = s {
            scopes.last_mut().unwrap().insert(name.clone(), 0);
        }
    }

    fn resolve_stmt_recursive(
        stmts: &[Stmt],
        scopes: &mut Vec<HashMap<String, DefId>>,
        errors: &mut Vec<String>,
    ) {
        for stmt in stmts {
            match stmt {
                Stmt::Fn {
                    name, params, body, ..
                } => {
                    scopes.last_mut().unwrap().insert(name.clone(), 0);
                    scopes.push(HashMap::new());
                    for p in params {
                        scopes.last_mut().unwrap().insert(p.clone(), 0);
                    }
                    resolve_stmt_recursive(body, scopes, errors);
                    scopes.pop();
                }
                Stmt::Let(name, expr) => {
                    match expr {
                        Expr::Var(v) => {
                            let mut found = false;
                            for s in scopes.iter().rev() {
                                if s.contains_key(v) {
                                    found = true;
                                    break;
                                }
                            }
                            if !found {
                                errors.push(format!("Undefined name '{}'", v));
                            }
                        }
                        Expr::Call(fname, args) => {
                            let mut found = false;
                            for s in scopes.iter().rev() {
                                if s.contains_key(fname) {
                                    found = true;
                                    break;
                                }
                            }
                            if !found {
                                errors.push(format!("Undefined function '{}'", fname));
                            }
                            for a in args {
                                if let Expr::Var(v) = a {
                                    let mut found = false;
                                    for s in scopes.iter().rev() {
                                        if s.contains_key(v) {
                                            found = true;
                                            break;
                                        }
                                    }
                                    if !found {
                                        errors.push(format!("Undefined name '{}'", v));
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                    scopes.last_mut().unwrap().insert(name.clone(), 0);
                }
                Stmt::Print(expr) | Stmt::ExprStmt(expr) => {
                    if let Expr::Var(v) = expr {
                        let mut found = false;
                        for s in scopes.iter().rev() {
                            if s.contains_key(v) {
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            errors.push(format!("Undefined name '{}'", v));
                        }
                    }
                }
                Stmt::Block(inner) => {
                    scopes.push(HashMap::new());
                    resolve_stmt_recursive(inner, scopes, errors);
                    scopes.pop();
                }
                Stmt::If {
                    cond,
                    then_body,
                    else_body,
                } => {
                    if let Expr::Var(v) = cond.as_ref() {
                        let mut found = false;
                        for s in scopes.iter().rev() {
                            if s.contains_key(v) {
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            errors.push(format!("Undefined name '{}'", v));
                        }
                    }
                    scopes.push(HashMap::new());
                    resolve_stmt_recursive(then_body, scopes, errors);
                    scopes.pop();
                    scopes.push(HashMap::new());
                    resolve_stmt_recursive(else_body, scopes, errors);
                    scopes.pop();
                }
                Stmt::Loop { body } | Stmt::For { body, .. } | Stmt::While { body, .. } => {
                    scopes.push(HashMap::new());
                    resolve_stmt_recursive(body, scopes, errors);
                    scopes.pop();
                }
                Stmt::Return(expr) => {
                    if let Expr::Var(v) = expr {
                        let mut found = false;
                        for s in scopes.iter().rev() {
                            if s.contains_key(v) {
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            errors.push(format!("Undefined name '{}'", v));
                        }
                    }
                }
                Stmt::Break | Stmt::Continue => {}
                Stmt::Assign(name, expr) => {
                    if let Expr::Var(v) = expr {
                        let mut found = false;
                        for s in scopes.iter().rev() {
                            if s.contains_key(v) {
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            errors.push(format!("Undefined name '{}'", v));
                        }
                    }
                    scopes.last_mut().unwrap().insert(name.clone(), 0);
                }
                Stmt::ExprFieldAssign(_, _, _) => {}
                Stmt::WhileIn {
                    var_name,
                    iterable,
                    body,
                } => {
                    if let Expr::Var(v) = iterable.as_ref() {
                        let mut found = false;
                        for s in scopes.iter().rev() {
                            if s.contains_key(v) {
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            errors.push(format!("Undefined name '{}'", v));
                        }
                    }
                    scopes.last_mut().unwrap().insert(var_name.clone(), 0);
                    scopes.push(HashMap::new());
                    resolve_stmt_recursive(body, scopes, errors);
                    scopes.pop();
                }
                Stmt::Unsafe { body } => {
                    scopes.push(HashMap::new());
                    resolve_stmt_recursive(body, scopes, errors);
                    scopes.pop();
                }
                Stmt::LetLinear(name, expr) => {
                    if let Expr::Var(v) = expr {
                        let mut found = false;
                        for s in scopes.iter().rev() {
                            if s.contains_key(v) {
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            errors.push(format!("Undefined name '{}'", v));
                        }
                    }
                    scopes.last_mut().unwrap().insert(name.clone(), 0);
                }
                Stmt::Struct { name, .. } => {
                    scopes.last_mut().unwrap().insert(name.clone(), 0);
                }
                Stmt::Enum { name, .. } => {
                    scopes.last_mut().unwrap().insert(name.clone(), 0);
                }
            }
        }
    }

    resolve_stmt_recursive(&prog.stmts, &mut scopes, &mut errors);

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(ResolveResult {
        symbols: scopes.into_iter().flatten().collect(),
    })
}
