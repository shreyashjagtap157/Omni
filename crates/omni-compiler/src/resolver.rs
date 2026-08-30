use crate::ast::{Expr, Program, Stmt, Visibility};
use crate::diagnostics::{error_codes, Diagnostic};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};

pub type DefId = usize;

static NEXT_DEF_ID: AtomicUsize = AtomicUsize::new(1);

pub fn generate_def_id() -> DefId {
    NEXT_DEF_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone)]
pub struct StructBoundInfo {
    pub type_params: Vec<(String, Vec<String>)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Variable,
    Struct,
    Enum,
    Trait,
    TypeAlias,
    ErrorSet,
    Capability,
    Actor,
    Module,
    Use,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(pub usize);

#[derive(Debug)]
pub struct Scope {
    pub id: ScopeId,
    pub parent: Option<ScopeId>,
    pub symbols: HashMap<String, DefId>,
}

#[derive(Debug)]
pub struct ScopeTree {
    pub scopes: Vec<Scope>,
    pub current: ScopeId,
}

impl ScopeTree {
    pub fn new() -> Self {
        ScopeTree {
            scopes: vec![Scope {
                id: ScopeId(0),
                parent: None,
                symbols: HashMap::new(),
            }],
            current: ScopeId(0),
        }
    }

    pub fn enter_scope(&mut self) -> ScopeId {
        let id = ScopeId(self.scopes.len());
        self.scopes.push(Scope {
            id,
            parent: Some(self.current),
            symbols: HashMap::new(),
        });
        let prev = self.current;
        self.current = id;
        prev
    }

    pub fn exit_to(&mut self, prev: ScopeId) {
        self.current = prev;
    }

    pub fn insert(&mut self, name: String, def: DefId) -> Option<DefId> {
        self.scopes[self.current.0].symbols.insert(name, def)
    }

    pub fn lookup(&self, name: &str) -> Option<&DefId> {
        let mut scope_id = Some(self.current);
        while let Some(sid) = scope_id {
            if let Some(def) = self.scopes[sid.0].symbols.get(name) {
                return Some(def);
            }
            scope_id = self.scopes[sid.0].parent;
        }
        None
    }

    pub fn lookup_current_scope(&self, name: &str) -> Option<&DefId> {
        self.scopes[self.current.0].symbols.get(name)
    }

    pub fn lookup_all_scopes(&self, name: &str) -> Vec<(ScopeId, DefId)> {
        let mut results = Vec::new();
        let mut scope_id = Some(self.current);
        while let Some(sid) = scope_id {
            if let Some(def) = self.scopes[sid.0].symbols.get(name) {
                results.push((sid, *def));
            }
            scope_id = self.scopes[sid.0].parent;
        }
        results
    }
}

#[derive(Debug)]
pub struct ResolveResult {
    pub symbols: HashMap<String, DefId>,
    pub def_names: HashMap<DefId, String>,
    pub def_visibility: HashMap<DefId, Visibility>,
    pub def_kinds: HashMap<DefId, SymbolKind>,
}

/// Built-in function names that are provided by the compiler (type checker hardcoded
/// signatures and interpreter runtime implementations). These don't need to be defined
/// in user source code but must be resolvable by the name resolver.
const BUILTIN_FUNCTIONS: &[&str] = &[
    // String operations
    "str_len",
    "string_concat",
    "string_eq",
    "string_push_char",
    "string_substr",
    "string_starts_with",
    "string_ends_with",
    "string_find",
    "string_trim",
    "string_replace",
    "string_to_int",
    "int_to_string",
    // Integer operations
    "int_abs",
    "int_pow",
    "int_div",
    // Option combinators
    "option_is_some",
    "option_unwrap_or",
    "option_map",
    "option_and",
    "option_flat_map",
    "option_or_else",
    "option_filter",
    "option_zip",
    "option_transpose",
    // Result combinators
    "result_is_ok",
    "result_unwrap_or",
    "result_map",
    "result_map_err",
    "result_flat_map",
    "result_or_else",
    "result_transpose",
    // Vector operations
    "vector_new",
    "vector_push",
    "vector_len",
    "vector_get",
    "vector_set",
    "vector_pop",
    "vector_push_front",
    "vector_insert",
    "vector_remove",
    "vector_clear",
    "vector_contains",
    "vector_capacity",
    "vector_reserve",
    // HashMap operations
    "hashmap_new",
    "hashmap_insert",
    "hashmap_get",
    "hashmap_contains",
    "hashmap_remove",
    "hashmap_len",
    "hashmap_clear",
    // HashSet operations
    "hashset_new",
    "hashset_insert",
    "hashset_contains",
    "hashset_remove",
    "hashset_union",
    "hashset_intersect",
    "hashset_len",
    "hashset_clear",
    // Runtime support
    "panic",
    "print",
    "print_str",
    "__register_effect_handler",
    // Comptime
    "comptime_eval",
    "comptime",
];

fn define_current(
    tree: &mut ScopeTree,
    name: &str,
    errors: &mut Vec<Diagnostic>,
    def_names: &mut HashMap<DefId, String>,
) -> Option<DefId> {
    if tree.lookup_current_scope(name).is_some() {
        errors.push(Diagnostic::error(
            error_codes::RESOLVER_DUPLICATE_DEFINITION,
            format!("Duplicate definition '{}' in the same scope", name),
        ));
        return None;
    }
    let def_id = generate_def_id();
    def_names.insert(def_id, name.to_string());
    tree.insert(name.to_string(), def_id);
    Some(def_id)
}

pub fn resolve_program(prog: &Program) -> Result<ResolveResult, Vec<Diagnostic>> {
    let mut tree = ScopeTree::new();
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut def_names: HashMap<DefId, String> = HashMap::new();
    let mut struct_bounds: HashMap<String, StructBoundInfo> = HashMap::new();

    // Pre-register built-in function names so they pass resolution. User items
    // may not silently replace a builtin or another top-level item.
    for func_name in BUILTIN_FUNCTIONS {
        let _ = define_current(&mut tree, func_name, &mut errors, &mut def_names);
    }

    // Predeclare top-level items to permit forward references while still
    // diagnosing duplicate definitions deterministically.
    for s in &prog.stmts {
        let name_opt = match s {
            Stmt::Fn { name, .. }
            | Stmt::Struct { name, .. }
            | Stmt::Enum { name, .. }
            | Stmt::Trait { name, .. }
            | Stmt::TypeAlias { name, .. }
            | Stmt::ErrorSet { name, .. }
            | Stmt::Capability { name, .. }
            | Stmt::Actor { name, .. } => Some(name.as_str()),
            _ => None,
        };
        if let Some(name) = name_opt {
            let _ = define_current(&mut tree, name, &mut errors, &mut def_names);
        }
        if let Stmt::Enum { name, variants, .. } = s {
            for variant in variants {
                let constructor = format!("{name}::{}", variant.name);
                let _ = define_current(&mut tree, &constructor, &mut errors, &mut def_names);
            }
        }
    }

    collect_struct_bounds(&prog.stmts, &mut struct_bounds);

    resolve_stmt_recursive(
        &prog.stmts,
        &mut tree,
        &mut errors,
        &mut def_names,
        &struct_bounds,
    );

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut symbols = HashMap::new();
    for scope in &tree.scopes {
        for (name, def) in &scope.symbols {
            symbols.insert(name.clone(), *def);
        }
    }

    let (def_visibility, def_kinds) = collect_symbol_metadata(&prog.stmts, &symbols);

    Ok(ResolveResult {
        symbols,
        def_names,
        def_visibility,
        def_kinds,
    })
}

fn collect_symbol_metadata(
    stmts: &[Stmt],
    symbols: &HashMap<String, DefId>,
) -> (HashMap<DefId, Visibility>, HashMap<DefId, SymbolKind>) {
    let mut vis_map = HashMap::new();
    let mut kind_map = HashMap::new();
    for stmt in stmts {
        match stmt {
            Stmt::Fn {
                name, visibility, ..
            } => {
                if let Some(&def_id) = symbols.get(name) {
                    vis_map.insert(def_id, visibility.clone());
                    kind_map.insert(def_id, SymbolKind::Function);
                }
            }
            Stmt::Struct {
                name, visibility, ..
            } => {
                if let Some(&def_id) = symbols.get(name) {
                    vis_map.insert(def_id, visibility.clone());
                    kind_map.insert(def_id, SymbolKind::Struct);
                }
            }
            Stmt::Enum {
                name, visibility, ..
            } => {
                if let Some(&def_id) = symbols.get(name) {
                    vis_map.insert(def_id, visibility.clone());
                    kind_map.insert(def_id, SymbolKind::Enum);
                }
            }
            Stmt::Trait {
                name, visibility, ..
            } => {
                if let Some(&def_id) = symbols.get(name) {
                    vis_map.insert(def_id, visibility.clone());
                    kind_map.insert(def_id, SymbolKind::Trait);
                }
            }
            Stmt::TypeAlias {
                name, visibility, ..
            } => {
                if let Some(&def_id) = symbols.get(name) {
                    vis_map.insert(def_id, visibility.clone());
                    kind_map.insert(def_id, SymbolKind::TypeAlias);
                }
            }
            Stmt::ErrorSet { name, .. } => {
                if let Some(&def_id) = symbols.get(name) {
                    vis_map.insert(def_id, Visibility::Private);
                    kind_map.insert(def_id, SymbolKind::ErrorSet);
                }
            }
            Stmt::Capability { name, .. } => {
                if let Some(&def_id) = symbols.get(name) {
                    vis_map.insert(def_id, Visibility::Pub);
                    kind_map.insert(def_id, SymbolKind::Capability);
                }
            }
            Stmt::Actor { name, .. } => {
                if let Some(&def_id) = symbols.get(name) {
                    vis_map.insert(def_id, Visibility::Private);
                    kind_map.insert(def_id, SymbolKind::Actor);
                }
            }
            _ => {}
        }
    }
    (vis_map, kind_map)
}

fn collect_struct_bounds(stmts: &[Stmt], struct_bounds: &mut HashMap<String, StructBoundInfo>) {
    for stmt in stmts {
        match stmt {
            Stmt::Fn {
                type_params, body, ..
            } => {
                if !type_params.is_empty() {
                    struct_bounds.insert(
                        format!("fn_{}", type_params[0].0),
                        StructBoundInfo {
                            type_params: type_params.clone(),
                        },
                    );
                }
                collect_struct_bounds(body, struct_bounds);
            }
            Stmt::Impl {
                target,
                type_params,
                methods,
                ..
            } => {
                if !type_params.is_empty() {
                    struct_bounds.insert(
                        target.clone(),
                        StructBoundInfo {
                            type_params: type_params.clone(),
                        },
                    );
                }
                collect_struct_bounds(methods, struct_bounds);
            }
            Stmt::Block(inner, _) => {
                collect_struct_bounds(inner, struct_bounds);
            }
            Stmt::If {
                then_body,
                else_body,
                ..
            } => {
                collect_struct_bounds(then_body, struct_bounds);
                collect_struct_bounds(else_body, struct_bounds);
            }
            Stmt::Loop { body, .. }
            | Stmt::For { body, .. }
            | Stmt::While { body, .. }
            | Stmt::WhileIn { body, .. }
            | Stmt::Unsafe { body, .. } => {
                collect_struct_bounds(body, struct_bounds);
            }
            Stmt::UseScoped { body, .. } => {
                collect_struct_bounds(body, struct_bounds);
            }
            Stmt::Actor { handlers, .. } => {
                collect_struct_bounds(handlers, struct_bounds);
            }
            _ => {}
        }
    }
}

fn resolve_var(tree: &mut ScopeTree, v: &str, errors: &mut Vec<Diagnostic>) {
    if tree.lookup(v).is_none() {
        errors.push(Diagnostic::error(
            error_codes::RESOLVER_UNDEFINED_NAME,
            format!("Undefined name '{}'", v),
        ));
    }
}

fn collect_pattern_binding_names(pattern: &crate::ast::Pattern, names: &mut HashSet<String>) {
    match pattern {
        crate::ast::Pattern::Var(name) => {
            names.insert(name.clone());
        }
        crate::ast::Pattern::Struct(_, fields) => {
            for (_, nested) in fields {
                collect_pattern_binding_names(nested, names);
            }
        }
        crate::ast::Pattern::Or(patterns) => {
            for nested in patterns {
                collect_pattern_binding_names(nested, names);
            }
        }
        crate::ast::Pattern::Wildcard | crate::ast::Pattern::Literal(_) => {}
    }
}

fn define_pattern_bindings(
    tree: &mut ScopeTree,
    pattern: &crate::ast::Pattern,
    errors: &mut Vec<Diagnostic>,
    def_names: &mut HashMap<DefId, String>,
) {
    let mut names = HashSet::new();
    collect_pattern_binding_names(pattern, &mut names);
    let mut names: Vec<_> = names.into_iter().collect();
    names.sort();
    for name in names {
        let _ = define_current(tree, &name, errors, def_names);
    }
}

fn resolve_expr(
    tree: &mut ScopeTree,
    expr: &Expr,
    errors: &mut Vec<Diagnostic>,
    def_names: &mut HashMap<DefId, String>,
    struct_bounds: &HashMap<String, StructBoundInfo>,
) {
    match expr {
        Expr::Var(v, _) => resolve_var(tree, v, errors),
        Expr::Call(fname, args, _) => {
            if tree.lookup(fname).is_none() {
                errors.push(Diagnostic::error(
                    error_codes::RESOLVER_UNDEFINED_FUNCTION,
                    format!("Undefined function '{}'", fname),
                ));
            }
            for a in args {
                resolve_expr(tree, a, errors, def_names, struct_bounds);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            resolve_expr(tree, left, errors, def_names, struct_bounds);
            resolve_expr(tree, right, errors, def_names, struct_bounds);
        }
        Expr::UnaryOp { inner, .. } | Expr::Borrow { inner, .. } | Expr::Deref { inner, .. } => {
            resolve_expr(tree, inner, errors, def_names, struct_bounds)
        }
        Expr::FieldAccess { base, .. } => {
            resolve_expr(tree, base, errors, def_names, struct_bounds)
        }
        Expr::IfExpr {
            cond, then, else_, ..
        } => {
            resolve_expr(tree, cond, errors, def_names, struct_bounds);
            resolve_expr(tree, then, errors, def_names, struct_bounds);
            resolve_expr(tree, else_, errors, def_names, struct_bounds);
        }
        Expr::Block(stmts, _) => {
            let prev = tree.enter_scope();
            resolve_stmt_recursive(stmts, tree, errors, def_names, struct_bounds);
            tree.exit_to(prev);
        }
        Expr::Tuple(items, _) | Expr::Array(items, _) => {
            for item in items {
                resolve_expr(tree, item, errors, def_names, struct_bounds);
            }
        }
        Expr::Index(base, index, _) => {
            resolve_expr(tree, base, errors, def_names, struct_bounds);
            resolve_expr(tree, index, errors, def_names, struct_bounds);
        }
        Expr::Match { expr, arms, .. } => {
            resolve_expr(tree, expr, errors, def_names, struct_bounds);
            for arm in arms {
                let previous = tree.enter_scope();
                define_pattern_bindings(tree, &arm.pattern, errors, def_names);
                if let Some(g) = &arm.guard {
                    resolve_expr(tree, g, errors, def_names, struct_bounds);
                }
                resolve_expr(tree, &arm.body, errors, def_names, struct_bounds);
                tree.exit_to(previous);
            }
        }
        Expr::Range { start, end, .. } => {
            resolve_expr(tree, start, errors, def_names, struct_bounds);
            resolve_expr(tree, end, errors, def_names, struct_bounds);
        }
        Expr::Lambda { params, body, .. } => {
            let prev = tree.enter_scope();
            for p in params {
                let def_id = generate_def_id();
                def_names.insert(def_id, p.0.clone());
                tree.insert(p.0.clone(), def_id);
            }
            resolve_expr(tree, body, errors, def_names, struct_bounds);
            tree.exit_to(prev);
        }
        Expr::Await(inner, _) => resolve_expr(tree, inner, errors, def_names, struct_bounds),
        Expr::Try(inner, _) => resolve_expr(tree, inner, errors, def_names, struct_bounds),
        Expr::StructLit { fields, .. } => {
            for (_, expr) in fields {
                resolve_expr(tree, expr, errors, def_names, struct_bounds);
            }
        }
        Expr::Number(_, _)
        | Expr::Float(_, _)
        | Expr::Char(_, _)
        | Expr::StringLit(_, _)
        | Expr::ByteString(_, _)
        | Expr::Byte(_, _)
        | Expr::Interpolated(_, _)
        | Expr::Bool(_, _) => {}
    }
}

fn resolve_stmt_recursive(
    stmts: &[Stmt],
    tree: &mut ScopeTree,
    errors: &mut Vec<Diagnostic>,
    def_names: &mut HashMap<DefId, String>,
    struct_bounds: &HashMap<String, StructBoundInfo>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Fn {
                name, params, body, ..
            } => {
                // Root functions were predeclared above for forward references.
                // Nested functions are declared at their lexical position.
                if tree.current != ScopeId(0) {
                    let _ = define_current(tree, name, errors, def_names);
                }
                let prev = tree.enter_scope();
                for p in params {
                    let _ = define_current(tree, &p.0, errors, def_names);
                }
                resolve_stmt_recursive(body, tree, errors, def_names, struct_bounds);
                tree.exit_to(prev);
            }
            Stmt::Let(name, _type_ann, expr, _) | Stmt::LetMut(name, _type_ann, expr, _) => {
                resolve_expr(tree, expr, errors, def_names, struct_bounds);
                let _ = define_current(tree, name, errors, def_names);
            }
            Stmt::LetLinear(name, _type_ann, expr, _) => {
                resolve_expr(tree, expr, errors, def_names, struct_bounds);
                let _ = define_current(tree, name, errors, def_names);
            }
            Stmt::Print(expr, _) | Stmt::ExprStmt(expr, _) => {
                resolve_expr(tree, expr, errors, def_names, struct_bounds);
            }
            Stmt::Return(expr, _) => {
                resolve_expr(tree, expr, errors, def_names, struct_bounds);
            }
            Stmt::Block(inner, _) => {
                let prev = tree.enter_scope();
                resolve_stmt_recursive(inner, tree, errors, def_names, struct_bounds);
                tree.exit_to(prev);
            }
            Stmt::If {
                cond,
                bindings,
                then_body,
                else_body,
                ..
            } => {
                resolve_expr(tree, cond, errors, def_names, struct_bounds);
                let prev = tree.enter_scope();
                for (name, expr) in bindings {
                    resolve_expr(tree, expr, errors, def_names, struct_bounds);
                    let _ = define_current(tree, name, errors, def_names);
                }
                resolve_stmt_recursive(then_body, tree, errors, def_names, struct_bounds);
                tree.exit_to(prev);
                let prev = tree.enter_scope();
                resolve_stmt_recursive(else_body, tree, errors, def_names, struct_bounds);
                tree.exit_to(prev);
            }
            Stmt::Loop { body, .. } => {
                let prev = tree.enter_scope();
                resolve_stmt_recursive(body, tree, errors, def_names, struct_bounds);
                tree.exit_to(prev);
            }
            Stmt::For {
                var_name,
                iterable,
                body,
                ..
            } => {
                resolve_expr(tree, iterable, errors, def_names, struct_bounds);
                let prev = tree.enter_scope();
                let _ = define_current(tree, var_name, errors, def_names);
                resolve_stmt_recursive(body, tree, errors, def_names, struct_bounds);
                tree.exit_to(prev);
            }
            Stmt::While { cond, body, .. } => {
                resolve_expr(tree, cond, errors, def_names, struct_bounds);
                let prev = tree.enter_scope();
                resolve_stmt_recursive(body, tree, errors, def_names, struct_bounds);
                tree.exit_to(prev);
            }
            Stmt::Break(_) | Stmt::Continue(_) => {}
            Stmt::Assign(name, expr, _) => {
                resolve_var(tree, name, errors);
                resolve_expr(tree, expr, errors, def_names, struct_bounds);
            }
            Stmt::ExprFieldAssign(base, _, expr, _) => {
                resolve_expr(tree, base, errors, def_names, struct_bounds);
                resolve_expr(tree, expr, errors, def_names, struct_bounds);
            }
            Stmt::DerefAssign(reference, expr, _) => {
                resolve_expr(tree, reference, errors, def_names, struct_bounds);
                resolve_expr(tree, expr, errors, def_names, struct_bounds);
            }
            Stmt::WhileIn {
                var_name,
                iterable,
                body,
                ..
            } => {
                resolve_expr(tree, iterable, errors, def_names, struct_bounds);
                let prev = tree.enter_scope();
                let _ = define_current(tree, var_name, errors, def_names);
                resolve_stmt_recursive(body, tree, errors, def_names, struct_bounds);
                tree.exit_to(prev);
            }
            Stmt::Unsafe { body, .. } => {
                let prev = tree.enter_scope();
                resolve_stmt_recursive(body, tree, errors, def_names, struct_bounds);
                tree.exit_to(prev);
            }
            Stmt::Struct { name, .. } => {
                if tree.current != ScopeId(0) {
                    let _ = define_current(tree, name, errors, def_names);
                }
            }
            Stmt::Enum { name, variants, .. } => {
                if tree.current != ScopeId(0) {
                    let _ = define_current(tree, name, errors, def_names);
                    for variant in variants {
                        let constructor = format!("{name}::{}", variant.name);
                        let _ = define_current(tree, &constructor, errors, def_names);
                    }
                }
            }
            Stmt::ErrorSet { name, .. } => {
                if tree.current != ScopeId(0) {
                    let _ = define_current(tree, name, errors, def_names);
                }
            }
            Stmt::Trait { name, .. } => {
                if tree.current != ScopeId(0) {
                    let _ = define_current(tree, name, errors, def_names);
                }
            }
            Stmt::TypeAlias { name, .. } => {
                if tree.current != ScopeId(0) {
                    let _ = define_current(tree, name, errors, def_names);
                }
            }
            Stmt::Use { path, alias, .. } => {
                let name = alias.clone().unwrap_or_else(|| {
                    let parts: Vec<&str> = if path.contains("::") {
                        path.split("::").collect()
                    } else {
                        path.split('.').collect()
                    };
                    parts.last().copied().unwrap_or(path.as_str()).to_string()
                });
                let _ = define_current(tree, &name, errors, def_names);
            }
            Stmt::EffectHandler { handler, .. } => {
                resolve_expr(tree, handler, errors, def_names, struct_bounds);
            }
            Stmt::Spawn { task, .. } => {
                resolve_expr(tree, task, errors, def_names, struct_bounds);
            }
            Stmt::GcMode { .. }
            | Stmt::CancelToken { .. }
            | Stmt::Channel { .. }
            | Stmt::WorkStealingExecutor { .. }
            | Stmt::DeterministicRuntime { .. }
            | Stmt::Tensor { .. }
            | Stmt::Simd { .. }
            | Stmt::DebugSession { .. }
            | Stmt::FfiSandbox { .. } => {}
            Stmt::DocComment { .. } => {
                // Documentation attaches metadata to a declaration; it never
                // creates a value/type definition of its own.
            }
            Stmt::Capability { name, .. } | Stmt::Actor { name, .. } => {
                if tree.current != ScopeId(0) {
                    let _ = define_current(tree, name, errors, def_names);
                }
            }
            Stmt::UseScoped {
                path: _,
                aliases,
                body,
                ..
            } => {
                let prev = tree.enter_scope();
                for (source_name, local_alias) in aliases {
                    // Resolve the imported/source name before defining its
                    // local alias; otherwise an alias can accidentally make an
                    // unresolved import appear valid.
                    if tree.lookup(source_name).is_none() {
                        errors.push(Diagnostic::error(
                            error_codes::RESOLVER_UNDEFINED_NAME,
                            format!("Undefined name '{}' in scoped import", source_name),
                        ));
                    }
                    let local_name = local_alias.as_deref().unwrap_or(source_name);
                    let _ = define_current(tree, local_name, errors, def_names);
                }
                resolve_stmt_recursive(body, tree, errors, def_names, struct_bounds);
                tree.exit_to(prev);
            }
            Stmt::Impl {
                target,
                type_params,
                methods,
                ..
            } => {
                // An impl extends an existing nominal type/trait; it must not
                // create or overwrite that definition.
                resolve_var(tree, target, errors);
                let prev = tree.enter_scope();

                for (tp_name, tp_bounds) in type_params {
                    let _ = define_current(tree, tp_name, errors, def_names);
                    for bound in tp_bounds {
                        if tree.lookup(bound).is_none() {
                            errors.push(Diagnostic::error(
                                error_codes::RESOLVER_UNDEFINED_NAME,
                                format!("Undefined trait bound '{}'", bound),
                            ));
                        }
                    }
                }

                if let Some(bounds_info) = struct_bounds.get(target) {
                    for (tp_name, tp_bounds) in &bounds_info.type_params {
                        if tree.lookup_current_scope(tp_name).is_none() {
                            let _ = define_current(tree, tp_name, errors, def_names);
                        }
                        for bound in tp_bounds {
                            if tree.lookup(bound).is_none() {
                                errors.push(Diagnostic::error(
                                    error_codes::RESOLVER_UNDEFINED_NAME,
                                    format!("Undefined trait bound '{}'", bound),
                                ));
                            }
                        }
                    }
                }

                resolve_stmt_recursive(methods, tree, errors, def_names, struct_bounds);
                tree.exit_to(prev);
            }
            Stmt::ContractRequires { condition, .. } => {
                resolve_expr(tree, condition, errors, def_names, struct_bounds);
            }
            Stmt::ContractEnsures { condition, .. } => {
                resolve_expr(tree, condition, errors, def_names, struct_bounds);
            }
            Stmt::ContractInvariant { condition, .. } => {
                resolve_expr(tree, condition, errors, def_names, struct_bounds);
            }
            Stmt::ComptimeLimit { .. } => {}
            Stmt::ModBlock(name, body, _) => {
                let _ = define_current(tree, name, errors, def_names);
                let prev = tree.enter_scope();
                resolve_stmt_recursive(body, tree, errors, def_names, struct_bounds);
                tree.exit_to(prev);
            }
            Stmt::Mod(name, _) => {
                let _ = define_current(tree, name, errors, def_names);
            }
            Stmt::Annotation(_, _) => {}
            &Stmt::Defer { .. } | &Stmt::AsyncDefer { .. } => {}
        }
    }
}

impl Default for ScopeTree {
    fn default() -> Self {
        ScopeTree::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Span;

    #[test]
    fn test_scope_tree_basic() {
        let mut tree = ScopeTree::new();
        tree.insert("x".to_string(), generate_def_id());
        assert!(tree.lookup("x").is_some());
        assert!(tree.lookup("y").is_none());
    }

    #[test]
    fn test_scope_tree_nesting() {
        let mut tree = ScopeTree::new();
        let global_id = generate_def_id();
        tree.insert("global".to_string(), global_id);
        let prev = tree.enter_scope();
        let local_id = generate_def_id();
        tree.insert("local".to_string(), local_id);
        assert!(tree.lookup("global").is_some());
        assert!(tree.lookup("local").is_some());
        assert_eq!(*tree.lookup("local").unwrap(), local_id);
        tree.exit_to(prev);
        assert!(tree.lookup("local").is_none());
        assert!(tree.lookup("global").is_some());
    }

    #[test]
    fn test_scope_tree_shadowing() {
        let mut tree = ScopeTree::new();
        tree.insert("x".to_string(), generate_def_id());
        let _prev = tree.enter_scope();
        let shadow_id = generate_def_id();
        tree.insert("x".to_string(), shadow_id);
        assert_eq!(*tree.lookup("x").unwrap(), shadow_id);
    }

    #[test]
    fn test_def_ids_are_unique() {
        let id1 = generate_def_id();
        let id2 = generate_def_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn duplicate_local_is_rejected() {
        let span = Span::default();
        let prog = Program {
            stmts: vec![Stmt::Fn {
                name: "main".to_string(),
                visibility: Visibility::Private,
                is_async: false,
                type_params: vec![],
                params: vec![],
                ret_type: Some("i64".to_string()),
                effects: vec![],
                contracts: vec![],
                body: vec![
                    Stmt::Let(
                        "x".to_string(),
                        Some("i64".to_string()),
                        Expr::Number(1, span.clone()),
                        span.clone(),
                    ),
                    Stmt::Let(
                        "x".to_string(),
                        Some("i64".to_string()),
                        Expr::Number(2, span.clone()),
                        span.clone(),
                    ),
                    Stmt::Return(Expr::Number(0, span.clone()), span.clone()),
                ],
                span: span.clone(),
            }],
        };
        let errors = resolve_program(&prog).expect_err("duplicate local must fail");
        assert!(errors
            .iter()
            .any(|e| e.code == error_codes::RESOLVER_DUPLICATE_DEFINITION));
    }

    #[test]
    fn assignment_requires_existing_binding() {
        let span = Span::default();
        let prog = Program {
            stmts: vec![Stmt::Fn {
                name: "main".to_string(),
                visibility: Visibility::Private,
                is_async: false,
                type_params: vec![],
                params: vec![],
                ret_type: Some("i64".to_string()),
                effects: vec![],
                contracts: vec![],
                body: vec![
                    Stmt::Assign("x".to_string(), Expr::Number(1, span.clone()), span.clone()),
                    Stmt::Return(Expr::Number(0, span.clone()), span.clone()),
                ],
                span: span.clone(),
            }],
        };
        let errors = resolve_program(&prog).expect_err("undefined assignment target must fail");
        assert!(errors
            .iter()
            .any(|e| e.code == error_codes::RESOLVER_UNDEFINED_NAME));
    }

    use crate::ast::{Expr, Program, Stmt, Visibility};

    #[test]
    fn test_resolve_program_assigns_unique_def_ids() {
        let prog = Program {
            stmts: vec![
                Stmt::Fn {
                    name: "foo".to_string(),
                    visibility: Visibility::Pub,
                    is_async: false,
                    type_params: vec![],
                    params: vec![],
                    ret_type: None,
                    effects: vec![],
                    contracts: vec![],
                    body: vec![Stmt::Let(
                        "x".to_string(),
                        None,
                        Expr::Number(1, Span::default()),
                        Span::default(),
                    )],
                    span: Span::default(),
                },
                Stmt::Fn {
                    name: "bar".to_string(),
                    visibility: Visibility::Pub,
                    is_async: false,
                    type_params: vec![],
                    params: vec![],
                    ret_type: None,
                    effects: vec![],
                    contracts: vec![],
                    body: vec![Stmt::Let(
                        "y".to_string(),
                        None,
                        Expr::Number(2, Span::default()),
                        Span::default(),
                    )],
                    span: Span::default(),
                },
            ],
        };
        let result = resolve_program(&prog).unwrap();
        let foo_id = result.symbols.get("foo").unwrap();
        let bar_id = result.symbols.get("bar").unwrap();
        assert_ne!(foo_id, bar_id, "foo and bar must have distinct DefIds");
        let x_id = result.symbols.get("x").unwrap();
        let y_id = result.symbols.get("y").unwrap();
        assert_ne!(x_id, y_id, "x and y must have distinct DefIds");
    }
}
