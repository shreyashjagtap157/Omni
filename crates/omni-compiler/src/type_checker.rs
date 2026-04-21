use crate::ast::{Expr, Program, Stmt, InterpolatedFragment};
use crate::lexer::TokenKind;
use crate::resolver;
use std::collections::{HashMap, HashSet};

pub const EF_IO: u8 = 0b0001;
pub const EF_PURE: u8 = 0b0010;
pub const EF_ASYNC: u8 = 0b0100;
pub const EF_PANIC: u8 = 0b1000;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Int,
    String,
    Bool,
    Var(u32),
    Generic(String),
    Fn {
        params: Vec<Type>,
        ret: Box<Type>,
        effects: u8,
    },
    Struct {
        name: String,
        fields: Vec<Type>,
        is_linear: bool,
    },
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
        is_sealed: bool,
    },
    Unit,
    Never,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trait {
    pub name: String,
    pub bounds: Vec<TraitBound>,
    pub required_methods: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitBound {
    pub trait_name: String,
    pub for_type: Type,
}

#[derive(Debug, Default)]
struct InferCtx {
    next_var: u32,
    subs: HashMap<u32, Type>,
}

impl InferCtx {
    fn new() -> Self {
        InferCtx {
            next_var: 0,
            subs: HashMap::new(),
        }
    }
    fn fresh_var(&mut self) -> Type {
        let id = self.next_var;
        self.next_var += 1;
        Type::Var(id)
    }
    fn resolve(&self, ty: &Type) -> Type {
        match ty {
            Type::Var(id) => {
                if let Some(t) = self.subs.get(id) {
                    self.resolve(t)
                } else {
                    Type::Var(*id)
                }
            }
            Type::Fn {
                params,
                ret,
                effects,
            } => {
                let p = params.iter().map(|p| self.resolve(p)).collect();
                Type::Fn {
                    params: p,
                    ret: Box::new(self.resolve(ret)),
                    effects: *effects,
                }
            }
            Type::Struct {
                name,
                fields,
                is_linear,
            } => {
                let resolved_fields = fields.iter().map(|f| self.resolve(f)).collect();
                Type::Struct {
                    name: name.clone(),
                    fields: resolved_fields,
                    is_linear: *is_linear,
                }
            }
            Type::Enum {
                name,
                variants,
                is_sealed,
            } => Type::Enum {
                name: name.clone(),
                variants: variants.clone(),
                is_sealed: *is_sealed,
            },
            other => other.clone(),
        }
    }

    fn contains_var(ty: &Type, id: u32) -> bool {
        match ty {
            Type::Var(v) => *v == id,
            Type::Fn { params, ret, .. } => {
                params.iter().any(|p| InferCtx::contains_var(p, id))
                    || InferCtx::contains_var(ret, id)
            }
            Type::Struct { fields, .. } => fields.iter().any(|f| InferCtx::contains_var(f, id)),
            _ => false,
        }
    }

    fn bind_var(&mut self, id: u32, ty: Type) -> Result<(), String> {
        if InferCtx::contains_var(&ty, id) {
            return Err(format!(
                "Occurs check failed for var {} in type {:?}",
                id, ty
            ));
        }
        self.subs.insert(id, ty);
        Ok(())
    }

    fn unify(&mut self, a: &Type, b: &Type) -> Result<(), String> {
        let ra = self.resolve(a);
        let rb = self.resolve(b);
        if ra == rb {
            return Ok(());
        }
        match (ra, rb) {
            (Type::Var(ida), tb) => self.bind_var(ida, tb),
            (ta, Type::Var(idb)) => self.bind_var(idb, ta),
            (
                Type::Fn {
                    params: pa,
                    ret: ra_ret,
                    effects: ea,
                },
                Type::Fn {
                    params: pb,
                    ret: rb_ret,
                    effects: eb,
                },
            ) => {
                if pa.len() != pb.len() {
                    return Err(format!("Function arity mismatch: {:?} vs {:?}", pa, pb));
                }
                if ea != eb {
                    return Err(format!("Function effect mismatch: {} vs {}", ea, eb));
                }
                for (x, y) in pa.iter().zip(pb.iter()) {
                    self.unify(x, y)?;
                }
                self.unify(&ra_ret, &rb_ret)
            }
            (Type::Int, Type::Int)
            | (Type::String, Type::String)
            | (Type::Bool, Type::Bool)
            | (Type::Unit, Type::Unit) => Ok(()),
            (Type::Generic(a), Type::Generic(b)) if a == b => Ok(()),
            (Type::Generic(_), _) | (_, Type::Generic(_)) => Ok(()),
            (Type::Never, _) | (_, Type::Never) => Ok(()),
            (
                Type::Struct {
                    name: _,
                    fields: f1,
                    is_linear: l1,
                },
                Type::Struct {
                    name: _,
                    fields: f2,
                    is_linear: l2,
                },
            ) => {
                if l1 != l2 {
                    return Err("Linear mismatch".to_string());
                }
                if f1.len() != f2.len() {
                    return Err("Field count mismatch".to_string());
                }
                for (a, b) in f1.iter().zip(f2.iter()) {
                    self.unify(a, b)?;
                }
                Ok(())
            }
            (Type::Struct { .. }, _) | (_, Type::Struct { .. }) => {
                Err("Cannot unify struct with non-struct".to_string())
            }
            (other_a, other_b) => Err(format!("Type mismatch: {:?} vs {:?}", other_a, other_b)),
        }
    }
}

pub fn type_check_program(prog: &Program) -> Result<(), String> {
    match resolver::resolve_program(prog) {
        Ok(_res) => {}
        Err(errs) => return Err(errs.join("; ")),
    }

    let mut symbols: HashMap<String, Type> = HashMap::new();

    // Builtin stdlib signatures (avoid requiring full implementations yet).
    symbols.insert(
        "str_len".to_string(),
        Type::Fn {
            params: vec![Type::String],
            ret: Box::new(Type::Int),
            effects: 0,
        },
    );
    symbols.insert(
        "string_concat".to_string(),
        Type::Fn {
            params: vec![Type::String, Type::String],
            ret: Box::new(Type::String),
            effects: 0,
        },
    );
    symbols.insert(
        "string_eq".to_string(),
        Type::Fn {
            params: vec![Type::String, Type::String],
            ret: Box::new(Type::Bool),
            effects: 0,
        },
    );
    symbols.insert(
        "string_push_char".to_string(),
        Type::Fn {
            params: vec![Type::String, Type::String],
            ret: Box::new(Type::String),
            effects: 0,
        },
    );
    symbols.insert(
        "string_substr".to_string(),
        Type::Fn {
            params: vec![Type::String, Type::Int, Type::Int],
            ret: Box::new(Type::String),
            effects: 0,
        },
    );
    symbols.insert(
        "string_starts_with".to_string(),
        Type::Fn {
            params: vec![Type::String, Type::String],
            ret: Box::new(Type::Bool),
            effects: 0,
        },
    );
    symbols.insert(
        "string_ends_with".to_string(),
        Type::Fn {
            params: vec![Type::String, Type::String],
            ret: Box::new(Type::Bool),
            effects: 0,
        },
    );
    symbols.insert(
        "string_find".to_string(),
        Type::Fn {
            params: vec![Type::String, Type::String],
            ret: Box::new(Type::Int),
            effects: 0,
        },
    );
    symbols.insert(
        "string_trim".to_string(),
        Type::Fn {
            params: vec![Type::String],
            ret: Box::new(Type::String),
            effects: 0,
        },
    );
    symbols.insert(
        "int_to_string".to_string(),
        Type::Fn {
            params: vec![Type::Int],
            ret: Box::new(Type::String),
            effects: 0,
        },
    );
    symbols.insert(
        "string_to_int".to_string(),
        Type::Fn {
            params: vec![Type::String],
            ret: Box::new(Type::Int),
            effects: 0,
        },
    );
    symbols.insert(
        "int_abs".to_string(),
        Type::Fn {
            params: vec![Type::Int],
            ret: Box::new(Type::Int),
            effects: 0,
        },
    );
    symbols.insert(
        "option_is_some".to_string(),
        Type::Fn {
            params: vec![Type::Generic("Option".to_string())],
            ret: Box::new(Type::Bool),
            effects: 0,
        },
    );
    symbols.insert(
        "option_unwrap_or".to_string(),
        Type::Fn {
            params: vec![
                Type::Generic("Option".to_string()),
                Type::Generic("T".to_string()),
            ],
            ret: Box::new(Type::Generic("T".to_string())),
            effects: 0,
        },
    );
    symbols.insert(
        "option_map".to_string(),
        Type::Fn {
            params: vec![
                Type::Generic("Option".to_string()),
                Type::Generic("F".to_string()),
            ],
            ret: Box::new(Type::Generic("Option".to_string())),
            effects: 0,
        },
    );
    symbols.insert(
        "option_and".to_string(),
        Type::Fn {
            params: vec![
                Type::Generic("Option".to_string()),
                Type::Generic("Option".to_string()),
            ],
            ret: Box::new(Type::Generic("Option".to_string())),
            effects: 0,
        },
    );
    symbols.insert(
        "result_is_ok".to_string(),
        Type::Fn {
            params: vec![Type::Generic("Result".to_string())],
            ret: Box::new(Type::Bool),
            effects: 0,
        },
    );
    symbols.insert(
        "result_unwrap_or".to_string(),
        Type::Fn {
            params: vec![
                Type::Generic("Result".to_string()),
                Type::Generic("T".to_string()),
            ],
            ret: Box::new(Type::Generic("T".to_string())),
            effects: 0,
        },
    );
    symbols.insert(
        "result_map".to_string(),
        Type::Fn {
            params: vec![
                Type::Generic("Result".to_string()),
                Type::Generic("F".to_string()),
            ],
            ret: Box::new(Type::Generic("Result".to_string())),
            effects: 0,
        },
    );
    symbols.insert(
        "result_map_err".to_string(),
        Type::Fn {
            params: vec![
                Type::Generic("Result".to_string()),
                Type::Generic("F".to_string()),
            ],
            ret: Box::new(Type::Generic("Result".to_string())),
            effects: 0,
        },
    );
    symbols.insert(
        "panic".to_string(),
        Type::Fn {
            params: vec![Type::String],
            ret: Box::new(Type::Never),
            effects: EF_PANIC,
        },
    );
    symbols.insert(
        "vector_new".to_string(),
        Type::Fn {
            params: vec![],
            ret: Box::new(Type::Generic("Vector".to_string())),
            effects: 0,
        },
    );
    symbols.insert(
        "vector_push".to_string(),
        Type::Fn {
            params: vec![
                Type::Generic("Vector".to_string()),
                Type::Generic("T".to_string()),
            ],
            ret: Box::new(Type::Int),
            effects: 0,
        },
    );
    symbols.insert(
        "vector_len".to_string(),
        Type::Fn {
            params: vec![Type::Generic("Vector".to_string())],
            ret: Box::new(Type::Int),
            effects: 0,
        },
    );
    symbols.insert(
        "vector_get".to_string(),
        Type::Fn {
            params: vec![Type::Generic("Vector".to_string()), Type::Int],
            ret: Box::new(Type::Generic("T".to_string())),
            effects: 0,
        },
    );
    symbols.insert(
        "vector_set".to_string(),
        Type::Fn {
            params: vec![
                Type::Generic("Vector".to_string()),
                Type::Int,
                Type::Generic("T".to_string()),
            ],
            ret: Box::new(Type::Int),
            effects: 0,
        },
    );
    symbols.insert(
        "vector_pop".to_string(),
        Type::Fn {
            params: vec![Type::Generic("Vector".to_string())],
            ret: Box::new(Type::Generic("T".to_string())),
            effects: 0,
        },
    );
    symbols.insert(
        "vector_push_front".to_string(),
        Type::Fn {
            params: vec![
                Type::Generic("Vector".to_string()),
                Type::Generic("T".to_string()),
            ],
            ret: Box::new(Type::Int),
            effects: 0,
        },
    );

    symbols.insert(
        "vector_insert".to_string(),
        Type::Fn {
            params: vec![
                Type::Generic("Vector".to_string()),
                Type::Int,
                Type::Generic("T".to_string()),
            ],
            ret: Box::new(Type::Int),
            effects: 0,
        },
    );
    symbols.insert(
        "vector_remove".to_string(),
        Type::Fn {
            params: vec![Type::Generic("Vector".to_string()), Type::Int],
            ret: Box::new(Type::Generic("T".to_string())),
            effects: 0,
        },
    );
    symbols.insert(
        "vector_clear".to_string(),
        Type::Fn {
            params: vec![Type::Generic("Vector".to_string())],
            ret: Box::new(Type::Int),
            effects: 0,
        },
    );
    symbols.insert(
        "vector_contains".to_string(),
        Type::Fn {
            params: vec![
                Type::Generic("Vector".to_string()),
                Type::Generic("T".to_string()),
            ],
            ret: Box::new(Type::Bool),
            effects: 0,
        },
    );
    symbols.insert(
        "vector_capacity".to_string(),
        Type::Fn {
            params: vec![Type::Generic("Vector".to_string())],
            ret: Box::new(Type::Int),
            effects: 0,
        },
    );
    symbols.insert(
        "vector_reserve".to_string(),
        Type::Fn {
            params: vec![Type::Generic("Vector".to_string()), Type::Int],
            ret: Box::new(Type::Int),
            effects: 0,
        },
    );

    symbols.insert(
        "hashmap_new".to_string(),
        Type::Fn {
            params: vec![],
            ret: Box::new(Type::Generic("HashMap".to_string())),
            effects: 0,
        },
    );
    symbols.insert(
        "hashmap_insert".to_string(),
        Type::Fn {
            params: vec![
                Type::Generic("HashMap".to_string()),
                Type::String,
                Type::Generic("T".to_string()),
            ],
            ret: Box::new(Type::Int),
            effects: 0,
        },
    );
    symbols.insert(
        "hashmap_get".to_string(),
        Type::Fn {
            params: vec![Type::Generic("HashMap".to_string()), Type::String],
            ret: Box::new(Type::Generic("T".to_string())),
            effects: 0,
        },
    );
    symbols.insert(
        "hashmap_contains".to_string(),
        Type::Fn {
            params: vec![Type::Generic("HashMap".to_string()), Type::String],
            ret: Box::new(Type::Bool),
            effects: 0,
        },
    );
    symbols.insert(
        "hashmap_remove".to_string(),
        Type::Fn {
            params: vec![Type::Generic("HashMap".to_string()), Type::String],
            ret: Box::new(Type::Int),
            effects: 0,
        },
    );
    symbols.insert(
        "hashmap_len".to_string(),
        Type::Fn {
            params: vec![Type::Generic("HashMap".to_string())],
            ret: Box::new(Type::Int),
            effects: 0,
        },
    );
    symbols.insert(
        "string_replace".to_string(),
        Type::Fn {
            params: vec![Type::String, Type::String, Type::String],
            ret: Box::new(Type::String),
            effects: 0,
        },
    );
    symbols.insert(
        "int_pow".to_string(),
        Type::Fn {
            params: vec![Type::Int, Type::Int],
            ret: Box::new(Type::Int),
            effects: 0,
        },
    );
    symbols.insert(
        "int_div".to_string(),
        Type::Fn {
            params: vec![Type::Int, Type::Int],
            ret: Box::new(Type::Int),
            effects: 0,
        },
    );

    // HashSet builtins (Map-backed at runtime)
    symbols.insert(
        "hashset_new".to_string(),
        Type::Fn {
            params: vec![],
            ret: Box::new(Type::Generic("HashSet".to_string())),
            effects: 0,
        },
    );
    symbols.insert(
        "hashset_insert".to_string(),
        Type::Fn {
            params: vec![
                Type::Generic("HashSet".to_string()),
                Type::Generic("T".to_string()),
            ],
            ret: Box::new(Type::Int),
            effects: 0,
        },
    );
    symbols.insert(
        "hashset_contains".to_string(),
        Type::Fn {
            params: vec![
                Type::Generic("HashSet".to_string()),
                Type::Generic("T".to_string()),
            ],
            ret: Box::new(Type::Bool),
            effects: 0,
        },
    );
    symbols.insert(
        "hashset_remove".to_string(),
        Type::Fn {
            params: vec![
                Type::Generic("HashSet".to_string()),
                Type::Generic("T".to_string()),
            ],
            ret: Box::new(Type::Int),
            effects: 0,
        },
    );
    symbols.insert(
        "hashset_union".to_string(),
        Type::Fn {
            params: vec![
                Type::Generic("HashSet".to_string()),
                Type::Generic("HashSet".to_string()),
            ],
            ret: Box::new(Type::Generic("HashSet".to_string())),
            effects: 0,
        },
    );
    symbols.insert(
        "hashset_intersect".to_string(),
        Type::Fn {
            params: vec![
                Type::Generic("HashSet".to_string()),
                Type::Generic("HashSet".to_string()),
            ],
            ret: Box::new(Type::Generic("HashSet".to_string())),
            effects: 0,
        },
    );
    symbols.insert(
        "hashset_len".to_string(),
        Type::Fn {
            params: vec![Type::Generic("HashSet".to_string())],
            ret: Box::new(Type::Int),
            effects: 0,
        },
    );
    symbols.insert(
        "hashset_clear".to_string(),
        Type::Fn {
            params: vec![Type::Generic("HashSet".to_string())],
            ret: Box::new(Type::Int),
            effects: 0,
        },
    );
    symbols.insert(
        "hashmap_clear".to_string(),
        Type::Fn {
            params: vec![Type::Generic("HashMap".to_string())],
            ret: Box::new(Type::Int),
            effects: 0,
        },
    );

    // Capture builtin function names so we can skip checking their bodies.
    let builtin_names: HashSet<String> = symbols.keys().cloned().collect();

    for s in &prog.stmts {
        if let Stmt::Fn {
            name,
            type_params,
            params,
            ret_type,
            effects,
            ..
        } = s
        {
            let mut ptypes = Vec::new();
            for _p in params {
                if !type_params.is_empty() {
                    ptypes.push(Type::Generic(type_params[0].clone()));
                } else {
                    ptypes.push(Type::Generic("_".to_string()));
                }
            }
            let rtype = if let Some(rt) = ret_type {
                match rt.as_str() {
                    "int" => Type::Int,
                    "string" => Type::String,
                    "bool" => Type::Bool,
                    other => Type::Generic(other.to_string()),
                }
            } else {
                Type::Unit
            };
            let mut efmask: u8 = 0;
            for e in effects {
                match e.as_str() {
                    "io" => efmask |= EF_IO,
                    "pure" => efmask |= EF_PURE,
                    "async" => efmask |= EF_ASYNC,
                    "panic" => efmask |= EF_PANIC,
                    _ => {}
                }
            }
            // Do not override builtin signatures inserted above.
            symbols.entry(name.clone()).or_insert(Type::Fn {
                params: ptypes,
                ret: Box::new(rtype),
                effects: efmask,
            });
        }
    }

    fn check_stmts(
        stmts: &[Stmt],
        symbols: &mut HashMap<String, Type>,
        ctx: &mut InferCtx,
        builtin_names: &HashSet<String>,
    ) -> Result<(Option<Type>, u8), String> {
        let mut last: Option<Type> = None;
        let mut effects: u8 = 0;
        for stmt in stmts {
            match stmt {
                Stmt::Let(name, expr) => {
                    let (typ, ef) = infer_expr_type(expr, symbols, ctx)?;
                    symbols.insert(name.clone(), typ);
                    effects |= ef;
                    last = None;
                }
                Stmt::Print(expr) => {
                    let (_t, ef) = infer_expr_type(expr, symbols, ctx)?;
                    effects |= ef;
                    effects |= EF_IO;
                    last = None;
                }
                Stmt::ExprStmt(expr) => {
                    let (t, ef) = infer_expr_type(expr, symbols, ctx)?;
                    effects |= ef;
                    last = Some(t);
                }
                Stmt::Block(inner) => {
                    let mut local = symbols.clone();
                    let (ret, ef) = check_stmts(inner, &mut local, ctx, builtin_names)?;
                    effects |= ef;
                    last = ret;
                }
                Stmt::Fn {
                    name,
                    type_params,
                    params,
                    ret_type,
                    effects: declared_effects,
                    body,
                } => {
                    let mut local = symbols.clone();
                    let mut fctx = InferCtx::new();

                    let mut gen_inst: HashMap<String, Type> = HashMap::new();
                    for tp in type_params.iter() {
                        gen_inst.insert(tp.clone(), fctx.fresh_var());
                    }

                    let mut ptypes: Vec<Type> = Vec::new();
                    for (i, _p) in params.iter().enumerate() {
                        if !type_params.is_empty() {
                            let gname = if i < type_params.len() {
                                type_params[i].clone()
                            } else {
                                type_params[0].clone()
                            };
                            if let Some(v) = gen_inst.get(&gname) {
                                ptypes.push(v.clone());
                            } else {
                                ptypes.push(fctx.fresh_var());
                            }
                        } else {
                            ptypes.push(fctx.fresh_var());
                        }
                    }
                    for (i, p) in params.iter().enumerate() {
                        local.insert(p.clone(), ptypes[i].clone());
                    }

                    let declared_ret_local = if let Some(rt) = ret_type {
                        match rt.as_str() {
                            "int" => Type::Int,
                            "string" => Type::String,
                            "bool" => Type::Bool,
                            other => gen_inst.get(other).cloned().unwrap_or(fctx.fresh_var()),
                        }
                    } else {
                        Type::Unit
                    };

                    // If this function name is a builtin, skip checking its body
                    // to avoid generic/unification conflicts with stdlib signatures.
                    let (ret_opt, efmask) = if builtin_names.contains(name) {
                        (None, 0)
                    } else {
                        check_stmts(body, &mut local, &mut fctx, builtin_names)?
                    };

                    let inferred_ret = match ret_opt {
                        Some(t) => t,
                        None => Type::Unit,
                    };

                    if declared_ret_local != Type::Unit {
                        if let Err(e) = fctx.unify(&inferred_ret, &declared_ret_local) {
                            return Err(format!("Function '{}' return type mismatch: {}", name, e));
                        }
                    }

                    let mut declared_mask: u8 = 0;
                    for e in declared_effects {
                        match e.as_str() {
                            "io" => declared_mask |= EF_IO,
                            "pure" => declared_mask |= EF_PURE,
                            "async" => declared_mask |= EF_ASYNC,
                            "panic" => declared_mask |= EF_PANIC,
                            _ => {}
                        }
                    }
                    if efmask != 0 {
                        if declared_mask == 0 {
                            return Err(format!("Function '{}' performs effects but has no explicit effect annotation", name));
                        }
                        if (efmask & !declared_mask) != 0 {
                            return Err(format!("Function '{}' performs effects {:?} not included in declared effects {:?}", name, efmask, declared_mask));
                        }
                    }

                    // Prefer existing builtin signature if present to avoid overriding
                    // standard library declarations with fresh type variables.
                    let mut top_ptypes: Vec<Type> = Vec::new();
                    let mut top_ret: Type = Type::Unit;
                    if let Some(existing) = symbols.get(name) {
                        if let Type::Fn {
                            params: eparams,
                            ret: eret,
                            ..
                        } = existing.clone()
                        {
                            top_ptypes = eparams;
                            top_ret = *eret;
                        }
                    }
                    if top_ptypes.is_empty() {
                        for i in 0..params.len() {
                            if !type_params.is_empty() {
                                let gname = if i < type_params.len() {
                                    type_params[i].clone()
                                } else {
                                    type_params[0].clone()
                                };
                                top_ptypes.push(Type::Generic(gname));
                            } else {
                                top_ptypes.push(ctx.fresh_var());
                            }
                        }
                        top_ret = if let Some(rt) = ret_type {
                            match rt.as_str() {
                                "int" => Type::Int,
                                "string" => Type::String,
                                "bool" => Type::Bool,
                                other => Type::Generic(other.to_string()),
                            }
                        } else {
                            Type::Unit
                        };
                    }
                    symbols.entry(name.clone()).or_insert(Type::Fn {
                        params: top_ptypes,
                        ret: Box::new(top_ret),
                        effects: declared_mask,
                    });
                    last = None;
                }
                Stmt::If {
                    cond,
                    then_body,
                    else_body,
                } => {
                    let (cond_type, cond_ef) = infer_expr_type(cond, symbols, ctx)?;
                    effects |= cond_ef;
                    if cond_type != Type::Bool {
                        return Err(format!("If condition must be bool, got {:?}", cond_type));
                    }
                    let (then_ret, then_ef) =
                        check_stmts(then_body, &mut symbols.clone(), ctx, builtin_names)?;
                    let (else_ret, else_ef) =
                        check_stmts(else_body, &mut symbols.clone(), ctx, builtin_names)?;
                    effects |= then_ef | else_ef;
                    last = then_ret.or(else_ret);
                }
                Stmt::Loop { body } | Stmt::For { body, .. } | Stmt::While { body, .. } => {
                    let (_ret, ef) = check_stmts(body, &mut symbols.clone(), ctx, builtin_names)?;
                    effects |= ef;
                    last = None;
                }
                Stmt::Return(expr) => {
                    let (t, ef) = infer_expr_type(expr, symbols, ctx)?;
                    effects |= ef;
                    last = Some(t);
                }
                Stmt::Break | Stmt::Continue => {
                    last = Some(Type::Never);
                }
                Stmt::Assign(name, expr) => {
                    let (typ, ef) = infer_expr_type(expr, symbols, ctx)?;
                    symbols.insert(name.clone(), typ);
                    effects |= ef;
                    last = None;
                }
                Stmt::ExprFieldAssign(base, _field, expr) => {
                    let (_base_type, base_ef) = infer_expr_type(base, symbols, ctx)?;
                    let (_expr_type, expr_ef) = infer_expr_type(expr, symbols, ctx)?;
                    effects |= base_ef | expr_ef;
                    last = None;
                }
                Stmt::WhileIn {
                    var_name,
                    iterable,
                    body,
                } => {
                    let (iter_type, iter_ef) = infer_expr_type(iterable, symbols, ctx)?;
                    let _ = iter_type;
                    symbols.insert(var_name.clone(), Type::Int);
                    let (_ret, body_ef) =
                        check_stmts(body, &mut symbols.clone(), ctx, builtin_names)?;
                    effects |= iter_ef | body_ef;
                    last = None;
                }
                Stmt::Unsafe { body } => {
                    let (ret, ef) = check_stmts(body, symbols, ctx, builtin_names)?;
                    effects |= ef | EF_PANIC;
                    last = ret;
                }
                Stmt::LetLinear(name, expr) => {
                    let (typ, ef) = infer_expr_type(expr, symbols, ctx)?;
                    symbols.insert(name.clone(), typ);
                    effects |= ef;
                    last = None;
                }
                Stmt::Struct {
                    name,
                    fields,
                    is_linear,
                } => {
                    let mut field_types = Vec::new();
                    for (_field_name, field_type_str) in fields {
                        let field_type = match field_type_str.as_str() {
                            "int" => Type::Int,
                            "string" => Type::String,
                            "bool" => Type::Bool,
                            other => Type::Generic(other.to_string()),
                        };
                        field_types.push(field_type);
                    }
                    symbols.insert(
                        name.clone(),
                        Type::Struct {
                            name: name.clone(),
                            fields: field_types,
                            is_linear: *is_linear,
                        },
                    );
                    last = None;
                }
                Stmt::Enum {
                    name,
                    variants,
                    is_sealed,
                } => {
                    let enum_variants: Vec<EnumVariant> = variants
                        .iter()
                        .map(|v| {
                            let field_types: Vec<Type> = v
                                .fields
                                .iter()
                                .map(|(_, t)| match t.as_str() {
                                    "int" => Type::Int,
                                    "string" => Type::String,
                                    "bool" => Type::Bool,
                                    other => Type::Generic(other.to_string()),
                                })
                                .collect();
                            EnumVariant {
                                name: v.name.clone(),
                                fields: field_types,
                            }
                        })
                        .collect();
                    symbols.insert(
                        name.clone(),
                        Type::Enum {
                            name: name.clone(),
                            variants: enum_variants,
                            is_sealed: *is_sealed,
                        },
                    );
                    last = None;
                }
            }
        }
        Ok((last, effects))
    }

    let mut global_ctx = InferCtx::new();
    check_stmts(&prog.stmts, &mut symbols, &mut global_ctx, &builtin_names).map(|_| ())
}

fn substitute_type(ty: &Type, gen_map: &HashMap<String, Type>) -> Type {
    match ty {
        Type::Var(id) => Type::Var(*id),
        Type::Generic(name) => gen_map
            .get(name)
            .cloned()
            .unwrap_or(Type::Generic(name.clone())),
        Type::Fn {
            params,
            ret,
            effects,
        } => {
            let new_params = params.iter().map(|p| substitute_type(p, gen_map)).collect();
            let new_ret = Box::new(substitute_type(ret, gen_map));
            Type::Fn {
                params: new_params,
                ret: new_ret,
                effects: *effects,
            }
        }
        Type::Int => Type::Int,
        Type::String => Type::String,
        Type::Bool => Type::Bool,
        Type::Unit => Type::Unit,
        Type::Never => Type::Never,
        Type::Struct {
            name,
            fields,
            is_linear,
        } => Type::Struct {
            name: name.clone(),
            fields: fields.clone(),
            is_linear: *is_linear,
        },
        Type::Enum {
            name,
            variants,
            is_sealed,
        } => Type::Enum {
            name: name.clone(),
            variants: variants.clone(),
            is_sealed: *is_sealed,
        },
    }
}

fn infer_expr_type(
    expr: &Expr,
    symbols: &HashMap<String, Type>,
    ctx: &mut InferCtx,
) -> Result<(Type, u8), String> {
    match expr {
        Expr::Number(_) => Ok((Type::Int, 0)),
        Expr::StringLit(_) => Ok((Type::String, 0)),
        Expr::Bool(_) => Ok((Type::Bool, 0)),
        Expr::Var(name) => match symbols.get(name).cloned() {
            Some(t) => Ok((ctx.resolve(&t), 0)),
            None => Err(format!("Undefined variable {}", name)),
        },
        Expr::Call(fname, args) => {
            let ftype = symbols
                .get(fname)
                .ok_or(format!("Undefined function {}", fname))?
                .clone();
            match ftype {
                Type::Fn {
                    params,
                    ret,
                    effects,
                } => {
                    if params.len() != args.len() {
                        return Err(format!(
                            "Function '{}' expected {} args, got {}",
                            fname,
                            params.len(),
                            args.len()
                        ));
                    }
                    let mut gen_map: HashMap<String, Type> = HashMap::new();
                    let mut acc_effects: u8 = 0;
                    for (i, a) in args.iter().enumerate() {
                        let (at, ef) = infer_expr_type(a, symbols, ctx)?;
                        acc_effects |= ef;
                        match &params[i] {
                            Type::Generic(gname) => {
                                if let Some(existing) = gen_map.get(gname) {
                                    if *existing != at {
                                        return Err(format!("Generic '{}' unified to conflicting types: {:?} vs {:?}", gname, existing, at));
                                    }
                                } else {
                                    gen_map.insert(gname.clone(), at);
                                }
                            }
                            Type::Var(_) => {}
                            pty => {
                                if *pty != at {
                                    return Err(format!(
                                        "Argument {} expected type {:?}, got {:?}",
                                        i, pty, at
                                    ));
                                }
                            }
                        }
                    }
                    let inst_ret = substitute_type(&ret, &gen_map);
                    acc_effects |= effects;
                    Ok((inst_ret, acc_effects))
                }
                other => Err(format!(
                    "Name '{}' is not callable (type {:?})",
                    fname, other
                )),
            }
        }
        Expr::BinaryOp { op, left, right } => {
            let (lt, lf) = infer_expr_type(left, symbols, ctx)?;
            let (rt, rf) = infer_expr_type(right, symbols, ctx)?;
            let effects = lf | rf;
            // Handle numeric/comparison operators by unifying type variables to `int` where possible.
            match op {
                TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Percent
                | TokenKind::EqEq
                | TokenKind::NotEq
                | TokenKind::Lt
                | TokenKind::LtEq
                | TokenKind::Gt
                | TokenKind::GtEq => {
                    // Attempt to bind any type variables to `int` so arithmetic/comparisons work.
                    let _ = match &lt {
                        Type::Var(_) => ctx.unify(&lt, &Type::Int),
                        _ => Ok(()),
                    };
                    let _ = match &rt {
                        Type::Var(_) => ctx.unify(&rt, &Type::Int),
                        _ => Ok(()),
                    };
                    let lt_res = ctx.resolve(&lt);
                    let rt_res = ctx.resolve(&rt);
                    if lt_res == Type::Int && rt_res == Type::Int {
                        let ret = match op {
                            TokenKind::EqEq
                            | TokenKind::NotEq
                            | TokenKind::Lt
                            | TokenKind::LtEq
                            | TokenKind::Gt
                            | TokenKind::GtEq => Type::Bool,
                            _ => Type::Int,
                        };
                        return Ok((ret, effects));
                    }
                }
                TokenKind::AndAnd | TokenKind::OrOr => {
                    let _ = match &lt {
                        Type::Var(_) => ctx.unify(&lt, &Type::Bool),
                        _ => Ok(()),
                    };
                    let _ = match &rt {
                        Type::Var(_) => ctx.unify(&rt, &Type::Bool),
                        _ => Ok(()),
                    };
                    let lt_res = ctx.resolve(&lt);
                    let rt_res = ctx.resolve(&rt);
                    if lt_res == Type::Bool && rt_res == Type::Bool {
                        return Ok((Type::Bool, effects));
                    }
                }
                _ => {}
            }
            Err(format!(
                "Unsupported binary operation {:?} between {:?} and {:?}",
                op, lt, rt
            ))
        }
        Expr::UnaryOp { op, inner } => {
            let (it, ef) = infer_expr_type(inner, symbols, ctx)?;
            let effects = ef;
            match op {
                TokenKind::Minus => {
                    if it == Type::Int {
                        return Ok((Type::Int, effects));
                    }
                    if let Type::Var(_) = it {
                        let _ = ctx.unify(&it, &Type::Int);
                        let resolved = ctx.resolve(&it);
                        if resolved == Type::Int {
                            return Ok((Type::Int, effects));
                        }
                    }
                }
                TokenKind::Bang => {
                    if it == Type::Bool {
                        return Ok((Type::Bool, effects));
                    }
                    if let Type::Var(_) = it {
                        let _ = ctx.unify(&it, &Type::Bool);
                        let resolved = ctx.resolve(&it);
                        if resolved == Type::Bool {
                            return Ok((Type::Bool, effects));
                        }
                    }
                }
                _ => {}
            }
            Err(format!("Unsupported unary operation {:?} on {:?}", op, it))
        }
        Expr::FieldAccess { base, field } => {
            let (bt, bf) = infer_expr_type(base, symbols, ctx)?;
            if bt == Type::String && field == "len" {
                return Ok((Type::Int, bf));
            }
            Err(format!("Unknown field access .{field} on {:?}", bt))
        }
        Expr::IfExpr { cond, then, else_ } => {
            let (cond_type, cond_ef) = infer_expr_type(cond, symbols, ctx)?;
            let mut effects = cond_ef;
            if cond_type != Type::Bool {
                return Err(format!("If condition must be bool, got {:?}", cond_type));
            }
            let (then_type, then_ef) = infer_expr_type(then, symbols, ctx)?;
            let (else_type, else_ef) = infer_expr_type(else_, symbols, ctx)?;
            effects |= then_ef | else_ef;
            ctx.unify(&then_type, &else_type)?;
            Ok((then_type, effects))
        }
        Expr::Block(stmts) => {
            let mut local = symbols.clone();
            let mut result_type = Type::Unit;
            let mut effects = 0u8;
            for stmt in stmts {
                match stmt {
                    Stmt::Let(name, expr) => {
                        let (typ, ef) = infer_expr_type(expr, &local, ctx)?;
                        local.insert(name.clone(), typ);
                        effects |= ef;
                    }
                    Stmt::Return(expr) => {
                        let (t, ef) = infer_expr_type(expr, &local, ctx)?;
                        effects |= ef;
                        result_type = t;
                        break;
                    }
                    Stmt::ExprStmt(expr) => {
                        let (t, ef) = infer_expr_type(expr, &local, ctx)?;
                        effects |= ef;
                        result_type = t;
                    }
                    _ => {}
                }
            }
            Ok((result_type, effects))
        }
        Expr::Tuple(exprs) => {
            let mut fields = Vec::new();
            let mut effects = 0u8;

            for expr in exprs {
                let (field_type, field_effects) = infer_expr_type(expr, symbols, ctx)?;
                effects |= field_effects;
                fields.push(field_type);
            }

            Ok((
                Type::Struct {
                    name: "Tuple".to_string(),
                    fields,
                    is_linear: false,
                },
                effects,
            ))
        }
        Expr::Match { expr, arms } => {
            let (scrutinee_type, scrutinee_effects) = infer_expr_type(expr, symbols, ctx)?;
            let mut effects = scrutinee_effects;
            let mut result_type: Option<Type> = None;

            for arm in arms {
                let _ = &scrutinee_type;
                if let Some(guard) = &arm.guard {
                    let (guard_type, guard_effects) = infer_expr_type(guard, symbols, ctx)?;
                    effects |= guard_effects;
                    if guard_type != Type::Bool {
                        return Err(format!("Match guard must be bool, got {:?}", guard_type));
                    }
                }

                let (arm_type, arm_effects) = infer_expr_type(&arm.body, symbols, ctx)?;
                effects |= arm_effects;

                if let Some(existing) = &result_type {
                    ctx.unify(existing, &arm_type)?;
                } else {
                    result_type = Some(arm_type);
                }
            }

            Ok((result_type.unwrap_or(Type::Unit), effects))
        }
        Expr::Interpolated(frags) => {
            let mut effects = 0u8;
            for frag in frags.iter() {
                match frag {
                    InterpolatedFragment::Literal(_) => {}
                    InterpolatedFragment::Expr(e) => {
                        let (_t, ef) = infer_expr_type(e, symbols, ctx)?;
                        effects |= ef;
                    }
                }
            }
            Ok((Type::String, effects))
        }
        Expr::Index(base, index) => {
            let (base_type, base_effects) = infer_expr_type(base, symbols, ctx)?;
            let (index_type, index_effects) = infer_expr_type(index, symbols, ctx)?;
            let effects = base_effects | index_effects;

            if index_type != Type::Int {
                return Err(format!("Index expression must be int, got {:?}", index_type));
            }

            match ctx.resolve(&base_type) {
                Type::Struct { name, fields, .. } if name == "Tuple" => {
                    if let Expr::Number(n) = index.as_ref() {
                        if *n < 0 {
                            return Err("Tuple index must be non-negative".to_string());
                        }

                        let idx = *n as usize;
                        match fields.get(idx).cloned() {
                            Some(field_type) => Ok((field_type, effects)),
                            None => Err(format!("Tuple index {} out of bounds", idx)),
                        }
                    } else {
                        Err("Tuple indexing requires a constant integer index".to_string())
                    }
                }
                Type::String => Ok((Type::String, effects)),
                other => Err(format!("Index expressions not yet implemented for {:?}", other)),
            }
        }
    }
}
