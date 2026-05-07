//! Name resolver with explicit scope tree and parent links.
//!
//! Replaces the old flat Vec<HashMap> stack.
//! Uses ScopeId with AtomicUsize for O(1) unique ID generation.

use crate::ast::{Expr, Program, Stmt};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

// ---------------------------------------------------------------------------
// Scope types
// ---------------------------------------------------------------------------

/// Unique identifier for a scope within a compilation unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(usize);

impl ScopeId {
    fn next() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(1);
        ScopeId(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

/// A single lexical scope with an optional parent link.
#[derive(Debug, Clone)]
pub struct Scope {
    pub id: ScopeId,
    /// The ID of the parent scope. None only for the root.
    pub parent_id: Option<ScopeId>,
    /// Names bound at this level.
    pub bindings: HashMap<String, DefId>,
}

impl Scope {
    /// Creates the root scope (id=0, no parent).
    pub fn root() -> Scope {
        Scope {
            id: ScopeId(0),
            parent_id: None,
            bindings: HashMap::new(),
        }
    }

    /// Creates a child scope with the given parent.
    pub fn child(parent_id: ScopeId) -> Scope {
        Scope {
            id: ScopeId::next(),
            parent_id: Some(parent_id),
            bindings: HashMap::new(),
        }
    }

    /// Looks up `name` in this scope only (no parent walk).
    pub fn get(&self, name: &str) -> Option<DefId> {
        self.bindings.get(name).copied()
    }

    /// Binds a name in this scope.
    pub fn put(&mut self, name: String, id: DefId) {
        self.bindings.insert(name, id);
    }
}

// ---------------------------------------------------------------------------
// Definition IDs
// ---------------------------------------------------------------------------

/// Definition identifier (crate-local index).
pub type DefId = usize;

fn intern_def() -> DefId {
    static NEXT: AtomicUsize = AtomicUsize::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed) as DefId
}

// ---------------------------------------------------------------------------
// Scope tree manager
// ---------------------------------------------------------------------------

/// Manages the scope tree keyed by ScopeId for O(depth) parent traversal.
#[derive(Debug)]
pub struct ScopeTree {
    /// All scopes keyed by ID.
    pub scopes: HashMap<ScopeId, Scope>,
}

impl ScopeTree {
    pub fn new() -> Self {
        let mut tree = HashMap::new();
        tree.insert(ScopeId(0), Scope::root());
        ScopeTree { scopes: tree }
    }

    /// Returns the root scope's ID.
    pub fn root_id(&self) -> ScopeId {
        ScopeId(0)
    }

    /// Looks up a name by walking parent IDs from `scope_id`.
    /// Returns the DefId of the first (innermost) match, or None.
    pub fn lookup(&self, name: &str, mut scope_id: ScopeId) -> Option<DefId> {
        loop {
            if let Some(scope) = self.scopes.get(&scope_id) {
                if let Some(def_id) = scope.get(name) {
                    return Some(def_id);
                }
                scope_id = if let Some(pid) = scope.parent_id {
                    pid
                } else {
                    return None;
                };
            } else {
                return None;
            }
        }
    }

    /// Visits all scopes in post-order (children first) so inner bindings shadow outer.
    pub fn for_each_binding<F: FnMut(ScopeId, &str, DefId)>(&self, mut f: F) {
        let mut stack = vec![ScopeId(0)];
        let mut visited = HashMap::new();
        while let Some(id) = stack.pop() {
            if visited.get(&id) == Some(&true) {
                // Process this scope's bindings
                if let Some(scope) = self.scopes.get(&id) {
                    for (name, &def_id) in &scope.bindings {
                        f(id, name, def_id);
                    }
                }
            } else {
                visited.insert(id, true);
                stack.push(id);
                // Push children
                for (_, scope) in &self.scopes {
                    if scope.parent_id == Some(id) {
                        stack.push(scope.id);
                    }
                }
            }
        }
    }
}

impl Default for ScopeTree {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// ResolveResult
// ---------------------------------------------------------------------------

/// Result of name resolution, carrying the full scope tree.
#[derive(Debug)]
pub struct ResolveResult {
    pub root_scope: ScopeId,
    /// Flat map: symbol name → DefId for the first (innermost) binding site.
    pub symbols: HashMap<String, DefId>,
}

// ---------------------------------------------------------------------------
// Core API
// ---------------------------------------------------------------------------

/// Main entry point: resolve all names in a program.
pub fn resolve_program(prog: &Program) -> Result<ResolveResult, Vec<String>> {
    let mut tree = ScopeTree::new();
    let mut symbols = HashMap::new();
    let mut errors = Vec::new();

    // Register builtins
    if let Some(scope) = tree.scopes.get_mut(&ScopeId(0)) {
        scope.put("print".to_string(), intern_def());
    }

    resolve_stmts(
        &prog.stmts,
        ScopeId(0),
        &mut tree,
        &mut symbols,
        &mut errors,
    );

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(ResolveResult {
        root_scope: tree.root_id(),
        symbols,
    })
}

// ---------------------------------------------------------------------------
// Statement visitor
// ---------------------------------------------------------------------------

fn resolve_stmts(
    stmts: &[Stmt],
    scope_id: ScopeId,
    tree: &mut ScopeTree,
    symbols: &mut HashMap<String, DefId>,
    errors: &mut Vec<String>,
) {
    for s in stmts {
        resolve_stmt(s, scope_id, tree, symbols, errors);
    }
}

fn resolve_stmt(
    stmt: &Stmt,
    scope_id: ScopeId,
    tree: &mut ScopeTree,
    symbols: &mut HashMap<String, DefId>,
    errors: &mut Vec<String>,
) {
    match stmt {
        Stmt::Fn {
            name, params, body, ..
        } => {
            let def_id = intern_def();
            // Bind in current scope
            if let Some(scope) = tree.scopes.get_mut(&scope_id) {
                scope.put(name.clone(), def_id);
            }
            // Register in flat map (first binding wins for shadowing)
            symbols.entry(name.clone()).or_insert(def_id);
            // Create child scope for function body
            let child_id = if let Some(scope) = tree.scopes.get_mut(&scope_id) {
                let child = Scope::child(scope_id);
                let cid = child.id;
                for p in params {
                    child.put(p.clone(), intern_def());
                }
                tree.scopes.insert(cid, child);
                cid
            } else {
                scope_id
            };
            resolve_stmts(body, child_id, tree, symbols, errors);
        }
        Stmt::Block(inner) => {
            let child_id = if let Some(scope) = tree.scopes.get_mut(&scope_id) {
                let child = Scope::child(scope_id);
                let cid = child.id;
                tree.scopes.insert(cid, child);
                cid
            } else {
                scope_id
            };
            resolve_stmts(inner, child_id, tree, symbols, errors);
        }
        Stmt::If {
            then_body,
            else_body,
            ..
        } => {
            let (c1, c2) = if let Some(scope) = tree.scopes.get_mut(&scope_id) {
                let c1_id = ScopeId::next();
                tree.scopes.insert(c1_id, Scope::child(scope_id));
                let c2_id = ScopeId::next();
                tree.scopes.insert(c2_id, Scope::child(scope_id));
                (c1_id, c2_id)
            } else {
                (scope_id, scope_id)
            };
            resolve_stmts(then_body, c1, tree, symbols, errors);
            resolve_stmts(else_body, c2, tree, symbols, errors);
        }
        Stmt::Loop { body }
        | Stmt::For { body, .. }
        | Stmt::While { body, .. }
        | Stmt::WhileIn { body, .. } => {
            let child_id = if let Some(scope) = tree.scopes.get_mut(&scope_id) {
                let child = Scope::child(scope_id);
                let cid = child.id;
                tree.scopes.insert(cid, child);
                cid
            } else {
                scope_id
            };
            resolve_stmts(body, child_id, tree, symbols, errors);
        }
        Stmt::Unsafe { body } => {
            let child_id = if let Some(scope) = tree.scopes.get_mut(&scope_id) {
                let child = Scope::child(scope_id);
                let cid = child.id;
                tree.scopes.insert(cid, child);
                cid
            } else {
                scope_id
            };
            resolve_stmts(body, child_id, tree, symbols, errors);
        }
        Stmt::Let(name, Expr::Var(v)) => {
            // Check if variable exists
            if tree.lookup(v, scope_id).is_none() {
                errors.push(format!("Undefined name '{}'", v));
            }
            let def_id = intern_def();
            if let Some(scope) = tree.scopes.get_mut(&scope_id) {
                scope.put(name.clone(), def_id);
            }
            symbols.entry(name.clone()).or_insert(def_id);
        }
        Stmt::Let(name, _) => {
            let def_id = intern_def();
            if let Some(scope) = tree.scopes.get_mut(&scope_id) {
                scope.put(name.clone(), def_id);
            }
            symbols.entry(name.clone()).or_insert(def_id);
        }
        Stmt::LetLinear(name, expr) => {
            if let Expr::Var(v) = expr {
                if tree.lookup(&v, scope_id).is_none() {
                    errors.push(format!("Undefined name '{}'", v));
                }
            }
            let def_id = intern_def();
            if let Some(scope) = tree.scopes.get_mut(&scope_id) {
                scope.put(name.clone(), def_id);
            }
            symbols.entry(name.clone()).or_insert(def_id);
        }
        Stmt::Struct { name, .. } => {
            let def_id = intern_def();
            if let Some(scope) = tree.scopes.get_mut(&scope_id) {
                scope.put(name.clone(), def_id);
            }
            symbols.entry(name.clone()).or_insert(def_id);
        }
        Stmt::Enum { name, .. } => {
            let def_id = intern_def();
            if let Some(scope) = tree.scopes.get_mut(&scope_id) {
                scope.put(name.clone(), def_id);
            }
            symbols.entry(name.clone()).or_insert(def_id);
        }
        Stmt::ErrorSet { name, .. } => {
            let def_id = intern_def();
            if let Some(scope) = tree.scopes.get_mut(&scope_id) {
                scope.put(name.clone(), def_id);
            }
            symbols.entry(name.clone()).or_insert(def_id);
        }
        Stmt::Impl { target, .. } => {
            let def_id = intern_def();
            if let Some(scope) = tree.scopes.get_mut(&scope_id) {
                scope.put(target.clone(), def_id);
            }
            symbols.entry(target.clone()).or_insert(def_id);
        }
        Stmt::Trait { name, .. } => {
            let def_id = intern_def();
            if let Some(scope) = tree.scopes.get_mut(&scope_id) {
                scope.put(name.clone(), def_id);
            }
            symbols.entry(name.clone()).or_insert(def_id);
        }
        Stmt::TypeAlias { name, .. } => {
            let def_id = intern_def();
            if let Some(scope) = tree.scopes.get_mut(&scope_id) {
                scope.put(name.clone(), def_id);
            }
            symbols.entry(name.clone()).or_insert(def_id);
        }
        Stmt::Actor { name, .. } => {
            let def_id = intern_def();
            if let Some(scope) = tree.scopes.get_mut(&scope_id) {
                scope.put(name.clone(), def_id);
            }
            symbols.entry(name.clone()).or_insert(def_id);
        }
        Stmt::DocComment { target, .. } => {
            let def_id = intern_def();
            if let Some(scope) = tree.scopes.get_mut(&scope_id) {
                scope.put(target.clone(), def_id);
            }
            symbols.entry(target.clone()).or_insert(def_id);
        }
        Stmt::Capability { name, .. } => {
            let def_id = intern_def();
            if let Some(scope) = tree.scopes.get_mut(&scope_id) {
                scope.put(name.clone(), def_id);
            }
            symbols.entry(name.clone()).or_insert(def_id);
        }
        // Non-binding statements: just recurse into expressions
        Stmt::Print(expr)
        | Stmt::ExprStmt(expr)
        | Stmt::Return(expr) => {
            check_expr(expr, scope_id, tree, errors);
        }
        Stmt::ExprFieldAssign(base_expr, _field, val_expr) => {
            check_expr(base_expr, scope_id, tree, errors);
            check_expr(val_expr, scope_id, tree, errors);
        }
        Stmt::Assign(name, expr) => {
            if let Expr::Var(v) = expr {
                if tree.lookup(&v, scope_id).is_none() {
                    errors.push(format!("Undefined name '{}'", v));
                }
            }
            let def_id = intern_def();
            if let Some(scope) = tree.scopes.get_mut(&scope_id) {
                scope.put(name.clone(), def_id);
            }
            symbols.entry(name.clone()).or_insert(def_id);
        }
        Stmt::Break | Stmt::Continue => {}
        Stmt::Spawn { .. }
        | Stmt::Channel { .. }
        | Stmt::WorkStealingExecutor { .. }
        | Stmt::DeterministicRuntime { .. }
        | Stmt::Tensor { .. }
        | Stmt::Simd { .. }
        | Stmt::DebugSession { .. }
        | Stmt::FfiSandbox { .. }
        | Stmt::GcMode { .. }
        | Stmt::CancelToken { .. }
        | Stmt::EffectHandler { .. }
        | Stmt::Use { .. } => {}
    }
}

fn check_expr(expr: &Expr, scope_id: ScopeId, tree: &mut ScopeTree, errors: &mut Vec<String>) {
    match expr {
        Expr::Var(v) => {
            if tree.lookup(v, scope_id).is_none() {
                errors.push(format!("Undefined name '{}'", v));
            }
        }
        Expr::Call(fname, args) => {
            if tree.lookup(fname, scope_id).is_none() {
                errors.push(format!("Undefined function '{}'", fname));
            }
            for a in args {
                check_expr(a, scope_id, tree, errors);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            check_expr(left, scope_id, tree, errors);
            check_expr(right, scope_id, tree, errors);
        }
        Expr::IfExpr { cond, then, else_, .. } => {
            check_expr(cond, scope_id, tree, errors);
            check_expr(then, scope_id, tree, errors);
            check_expr(else_, scope_id, tree, errors);
        }
        Expr::UnaryOp { inner, .. } => {
            check_expr(inner, scope_id, tree, errors);
        }
        Expr::FieldAccess { base, .. } => {
            check_expr(base, scope_id, tree, errors);
        }
        Expr::Block(stmts) => {
            // Block creates a new scope
            let child_id = if tree.scopes.contains_key(&scope_id) {
                let child = Scope::child(scope_id);
                let cid = child.id;
                tree.scopes.insert(cid, child);
                cid
            } else {
                scope_id
            };
            resolve_stmts(stmts, child_id, tree, &mut HashMap::new(), errors);
        }
        Expr::Tuple(exprs) => {
            for e in exprs {
                check_expr(e, scope_id, tree, errors);
            }
        }
        Expr::Interpolated(frags) => {
            use crate::ast::InterpolatedFragment;
            for frag in frags {
                if let InterpolatedFragment::Expr(e) = frag {
                    check_expr(e, scope_id, tree, errors);
                }
            }
        }
        Expr::Index(base, index) => {
            check_expr(base, scope_id, tree, errors);
            check_expr(index, scope_id, tree, errors);
        }
        Expr::Match { expr, arms } => {
            check_expr(expr, scope_id, tree, errors);
            for arm in arms {
                check_expr(&arm.body, scope_id, tree, errors);
            }
        }
        Expr::Range { start, end, .. } => {
            check_expr(start, scope_id, tree, errors);
            check_expr(end, scope_id, tree, errors);
        }
        Expr::StringLit(_) | Expr::Number(_) | Expr::Bool(_) => {}
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_id_zero() {
        let t = ScopeTree::new();
        assert_eq!(t.root_id(), ScopeId(0));
    }

    #[test]
    fn test_child_has_parent() {
        let t = ScopeTree::new();
        let c = ScopeId::next();
        t.scopes.insert(c, Scope::child(ScopeId(0)));
        assert_eq!(t.scopes.get(&c).unwrap().parent_id, Some(ScopeId(0)));
    }

    #[test]
    fn test_lookup_self() {
        let mut t = ScopeTree::new();
        if let Some(scope) = t.scopes.get_mut(&ScopeId(0)) {
            scope.put("x".to_string(), 42);
        }
        assert_eq!(t.lookup("x", ScopeId(0)), Some(42));
        assert_eq!(t.lookup("y", ScopeId(0)), None);
    }

    #[test]
    fn test_lookup_parent_chain() {
        let mut t = ScopeTree::new();
        if let Some(scope) = t.scopes.get_mut(&ScopeId(0)) {
            scope.put("x".to_string(), 1);
        }
        let c1 = ScopeId::next();
        let mut s = Scope::child(ScopeId(0));
        s.put("y".to_string(), 2);
        t.scopes.insert(c1, s);

        let c2 = ScopeId::next();
        let mut s2 = Scope::child(c1);
        s2.put("z".to_string(), 3);
        t.scopes.insert(c2, s2);

        assert_eq!(t.lookup("z", c2), Some(3));
        assert_eq!(t.lookup("y", c2), Some(2));
        assert_eq!(t.lookup("x", c2), Some(1));
        assert_eq!(t.lookup("missing", c2), None);
    }

    #[test]
    fn test_resolve_program() {
        let prog = Program {
            stmts: vec![
                Stmt::Let("a".to_string(), Expr::Number(1)),
                Stmt::Print(Expr::Var("a".to_string())),
            ],
        };
        let result = resolve_program(&prog);
        assert!(result.is_ok());
        let rr = result.unwrap();
        assert!(rr.symbols.contains_key("a"));
    }
}
