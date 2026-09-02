use crate::ast::{Expr, InterpolatedFragment, MatchArm, Pattern, Program, Stmt, Visibility};
use crate::complete_lexer::TokenKind;
use crate::diagnostics::{error_codes, Diagnostic};
use crate::resolver;
use crate::traits::TraitSystem;
pub use crate::types::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
struct VisibilityInfo {
    visibility: Visibility,
    defining_scope: String,
}

#[derive(Debug, Default)]
struct VisibilityCtx {
    items: HashMap<String, VisibilityInfo>,
    current_scope: String,
}

impl VisibilityCtx {
    fn new() -> Self {
        VisibilityCtx {
            items: HashMap::new(),
            current_scope: "global".to_string(),
        }
    }

    fn register(&mut self, name: &str, visibility: Visibility) {
        self.items.insert(
            name.to_string(),
            VisibilityInfo {
                visibility,
                defining_scope: self.current_scope.clone(),
            },
        );
    }

    fn check_access(&self, name: &str) -> Result<(), Diagnostic> {
        if let Some(info) = self.items.get(name) {
            match &info.visibility {
                Visibility::Private => {
                    if self.current_scope != info.defining_scope {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_VISIBILITY_PRIVATE,
                            format!(
                                "Cannot access private item '{}' from outside its defining scope",
                                name
                            ),
                        ));
                    }
                }
                Visibility::PubMod | Visibility::PubPkg | Visibility::Pub => {
                    // These are visible within their respective scopes
                }
                Visibility::PubCap(_) | Visibility::PubFriend(_) => {
                    // Capability and friend visibility - defer to runtime for now
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
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
            Type::Ref { mutable, inner } => Type::Ref {
                mutable: *mutable,
                inner: Box::new(self.resolve(inner)),
            },
            Type::Fn {
                params,
                ret,
                effects,
            } => {
                let p = params.iter().map(|p| self.resolve(p)).collect();
                Type::Fn {
                    params: p,
                    ret: Box::new(self.resolve(ret)),
                    effects: effects.clone(),
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
            Type::Option(inner) => Type::Option(Box::new(self.resolve(inner))),
            Type::Result(ok, err) => {
                Type::Result(Box::new(self.resolve(ok)), Box::new(self.resolve(err)))
            }
            Type::ErrorSet(name) => Type::ErrorSet(name.clone()),
            other => other.clone(),
        }
    }

    fn contains_var(ty: &Type, id: u32) -> bool {
        match ty {
            Type::Var(v) => *v == id,
            Type::Ref { inner, .. } => InferCtx::contains_var(inner, id),
            Type::Fn { params, ret, .. } => {
                params.iter().any(|p| InferCtx::contains_var(p, id))
                    || InferCtx::contains_var(ret, id)
            }
            Type::Struct { fields, .. } => fields.iter().any(|f| InferCtx::contains_var(f, id)),
            Type::Option(inner) => InferCtx::contains_var(inner, id),
            Type::Result(ok, err) => {
                InferCtx::contains_var(ok, id) || InferCtx::contains_var(err, id)
            }
            Type::ErrorSet(_) => false,
            _ => false,
        }
    }

    fn bind_var(&mut self, id: u32, ty: Type) -> Result<(), Diagnostic> {
        if InferCtx::contains_var(&ty, id) {
            return Err(Diagnostic::error(
                error_codes::TYPE_INFERENCE_FAILED,
                format!("Occurs check failed for var {} in type {:?}", id, ty),
            ));
        }
        self.subs.insert(id, ty);
        Ok(())
    }

    fn coerce_value_to_expected(
        &mut self,
        actual: &Type,
        expected: &Type,
    ) -> Result<(), Diagnostic> {
        let actual_resolved = self.resolve(actual);
        let expected_resolved = self.resolve(expected);
        match (&actual_resolved, &expected_resolved) {
            (
                Type::Ref {
                    mutable: true,
                    inner: actual_inner,
                },
                Type::Ref {
                    mutable: false,
                    inner: expected_inner,
                },
            ) => self.unify(actual_inner, expected_inner),
            _ => self.unify(&actual_resolved, &expected_resolved),
        }
    }

    fn unify(&mut self, a: &Type, b: &Type) -> Result<(), Diagnostic> {
        let ra = self.resolve(a);
        let rb = self.resolve(b);
        if ra == rb {
            return Ok(());
        }
        match (ra, rb) {
            (Type::Var(ida), tb) => self.bind_var(ida, tb),
            (ta, Type::Var(idb)) => self.bind_var(idb, ta),
            (
                Type::Ref {
                    mutable: ma,
                    inner: ia,
                },
                Type::Ref {
                    mutable: mb,
                    inner: ib,
                },
            ) => {
                if ma != mb {
                    return Err(Diagnostic::error(
                        error_codes::TYPE_MISMATCH,
                        "reference mutability mismatch",
                    ));
                }
                self.unify(&ia, &ib)
            }
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
                    return Err(Diagnostic::error(
                        error_codes::TYPE_MISMATCH,
                        format!("Function arity mismatch: {} vs {}", pa.len(), pb.len()),
                    ));
                }
                if ea != eb {
                    return Err(Diagnostic::error(
                        error_codes::TYPE_EFFECT_MISMATCH,
                        format!(
                            "Function effect mismatch: {} vs {}",
                            ea.to_string_list(),
                            eb.to_string_list()
                        ),
                    ));
                }
                for (x, y) in pa.iter().zip(pb.iter()) {
                    self.unify(x, y)?;
                }
                self.unify(&ra_ret, &rb_ret)
            }
            (Type::Int, Type::Int)
            | (Type::Float, Type::Float)
            | (Type::Char, Type::Char)
            | (Type::Byte, Type::Byte)
            | (Type::String, Type::String)
            | (Type::Bytes, Type::Bytes)
            | (Type::Bool, Type::Bool)
            | (Type::Unit, Type::Unit) => Ok(()),
            (Type::Generic(a), Type::Generic(b)) if a == b => Ok(()),
            (Type::Generic(_), _) | (_, Type::Generic(_)) => Ok(()),
            (Type::Never, _) | (_, Type::Never) => Ok(()),
            (
                Type::Struct {
                    name: n1,
                    fields: f1,
                    is_linear: l1,
                },
                Type::Struct {
                    name: n2,
                    fields: f2,
                    is_linear: l2,
                },
            ) => {
                if n1 != n2 {
                    return Err(Diagnostic::error(
                        error_codes::TYPE_MISMATCH,
                        format!("Nominal/aggregate type mismatch: {n1} vs {n2}"),
                    ));
                }
                if l1 != l2 {
                    return Err(Diagnostic::error(
                        error_codes::TYPE_MISMATCH,
                        "Linear mismatch",
                    ));
                }
                if matches!(n1.as_str(), "Tuple" | "Array" | "Slice") {
                    if f1.len() != f2.len() {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_MISMATCH,
                            "Structural aggregate field count mismatch",
                        ));
                    }
                    for (a, b) in f1.iter().zip(f2.iter()) {
                        self.unify(a, b)?;
                    }
                }
                Ok(())
            }
            (Type::Enum { name: n1, .. }, Type::Enum { name: n2, .. }) => {
                if n1 == n2 {
                    Ok(())
                } else {
                    Err(Diagnostic::error(
                        error_codes::TYPE_MISMATCH,
                        format!("Nominal enum type mismatch: {n1} vs {n2}"),
                    ))
                }
            }
            (Type::Struct { .. }, _) | (_, Type::Struct { .. }) => Err(Diagnostic::error(
                error_codes::TYPE_NOT_STRUCT,
                "Cannot unify struct with non-struct",
            )),
            (Type::Option(a), Type::Option(b)) => self.unify(&a, &b),
            (Type::Result(a_ok, a_err), Type::Result(b_ok, b_err)) => {
                self.unify(&a_ok, &b_ok)?;
                self.unify(&a_err, &b_err)
            }
            (Type::ErrorSet(a), Type::ErrorSet(b)) if a == b => Ok(()),
            (Type::ErrorSet(_), _) | (_, Type::ErrorSet(_)) => Err(Diagnostic::error(
                error_codes::TYPE_MISMATCH,
                "Cannot unify error set with non-error-set",
            )),
            (other_a, other_b) => Err(Diagnostic::error(
                error_codes::TYPE_MISMATCH,
                format!("Type mismatch: expected {:?}, found {:?}", other_b, other_a),
            )),
        }
    }
}

fn mutable_binding_symbol(name: &str) -> String {
    format!("__omni_mut_binding::{name}")
}

fn struct_field_symbol(name: &str, field: &str) -> String {
    format!("__omni_struct_field::{name}::{field}")
}

fn parse_type_annotation_with_symbols(
    type_str: &str,
    symbols: &HashMap<String, Type>,
) -> Result<Type, Diagnostic> {
    if let Some(ty) = symbols.get(type_str) {
        if !matches!(ty, Type::Fn { .. }) {
            return Ok(ty.clone());
        }
    }
    parse_type_annotation(type_str)
}

fn validate_struct_literal(
    name: &str,
    fields: &[(String, Expr)],
    symbols: &HashMap<String, Type>,
    ctx: &mut InferCtx,
    vis_ctx: &VisibilityCtx,
) -> Result<(Type, EffectSet), Diagnostic> {
    let declared = symbols.get(name).cloned().ok_or_else(|| {
        Diagnostic::error(
            error_codes::TYPE_UNDEFINED_TYPE,
            format!("Unknown struct type '{name}'"),
        )
    })?;
    let Type::Struct {
        name: declared_name,
        fields: declared_types,
        is_linear,
    } = declared
    else {
        return Err(Diagnostic::error(
            error_codes::TYPE_NOT_STRUCT,
            format!("'{name}' is not a struct type"),
        ));
    };

    let mut seen = HashSet::new();
    let mut effects = EffectSet::new();
    for (field_name, value) in fields {
        if !seen.insert(field_name.clone()) {
            return Err(Diagnostic::error(
                error_codes::TYPE_MISMATCH,
                format!("Duplicate field '{field_name}' in struct literal '{name}'"),
            ));
        }
        let field_ty = symbols
            .get(&struct_field_symbol(name, field_name))
            .cloned()
            .ok_or_else(|| {
                Diagnostic::error(
                    error_codes::TYPE_INVALID_FIELD_ACCESS,
                    format!("Struct '{name}' has no field '{field_name}'"),
                )
            })?;
        let (value_ty, value_effects) = synthesize_expr(value, symbols, ctx, vis_ctx)?;
        effects.union_with(&value_effects);
        ctx.unify(&value_ty, &field_ty).map_err(|_| {
            Diagnostic::error(
                error_codes::TYPE_MISMATCH,
                format!(
                    "Field '{field_name}' of struct '{name}' expects {:?}, found {:?}",
                    field_ty, value_ty
                ),
            )
        })?;
    }

    if fields.len() != declared_types.len() {
        return Err(Diagnostic::error(
            error_codes::TYPE_MISMATCH,
            format!(
                "Struct literal '{name}' initializes {} of {} required fields",
                fields.len(),
                declared_types.len()
            ),
        ));
    }

    Ok((
        Type::Struct {
            name: declared_name,
            fields: declared_types,
            is_linear,
        },
        effects,
    ))
}

fn enum_pattern_variant_name<'a>(enum_name: &str, pattern_name: &'a str) -> Option<&'a str> {
    if let Some((prefix, variant)) = pattern_name.rsplit_once("::") {
        if prefix == enum_name {
            Some(variant)
        } else {
            None
        }
    } else {
        Some(pattern_name)
    }
}

fn merge_pattern_binding(
    bindings: &mut HashMap<String, Type>,
    name: &str,
    ty: Type,
    ctx: &mut InferCtx,
) -> Result<(), Diagnostic> {
    if bindings.contains_key(name) {
        return Err(Diagnostic::error(
            error_codes::TYPE_MISMATCH,
            format!("Pattern binding '{}' is declared more than once", name),
        ));
    }
    let _ = ctx;
    bindings.insert(name.to_string(), ty);
    Ok(())
}

fn collect_pattern_bindings(
    pattern: &Pattern,
    scrutinee_type: &Type,
    ctx: &mut InferCtx,
) -> Result<HashMap<String, Type>, Diagnostic> {
    let resolved = ctx.resolve(scrutinee_type);
    match pattern {
        Pattern::Wildcard => Ok(HashMap::new()),
        Pattern::Var(name) => {
            let mut bindings = HashMap::new();
            bindings.insert(name.clone(), resolved);
            Ok(bindings)
        }
        Pattern::Literal(_) => {
            if matches!(resolved, Type::Int | Type::Bool) {
                Ok(HashMap::new())
            } else {
                Err(Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!("Literal pattern cannot match {:?}", resolved),
                ))
            }
        }
        Pattern::Struct(pattern_name, fields) => {
            let field_types: Vec<Type> = match &resolved {
                Type::Enum { name, variants, .. } => {
                    let variant_name =
                        enum_pattern_variant_name(name, pattern_name).ok_or_else(|| {
                            Diagnostic::error(
                                error_codes::TYPE_MISMATCH,
                                format!(
                                    "Variant pattern '{}' does not belong to enum '{}'",
                                    pattern_name, name
                                ),
                            )
                        })?;
                    variants
                        .iter()
                        .find(|variant| variant.name == variant_name)
                        .map(|variant| variant.fields.clone())
                        .ok_or_else(|| {
                            Diagnostic::error(
                                error_codes::TYPE_MISMATCH,
                                format!("Enum '{}' has no variant '{}'", name, variant_name),
                            )
                        })?
                }
                Type::Option(inner) => match pattern_name.as_str() {
                    "Some" => vec![inner.as_ref().clone()],
                    "None" => Vec::new(),
                    _ => {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_MISMATCH,
                            format!("Option has no variant '{}'", pattern_name),
                        ))
                    }
                },
                Type::Result(ok, err) => match pattern_name.as_str() {
                    "Ok" => vec![ok.as_ref().clone()],
                    "Err" => vec![err.as_ref().clone()],
                    _ => {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_MISMATCH,
                            format!("Result has no variant '{}'", pattern_name),
                        ))
                    }
                },
                Type::Struct { name, fields, .. } if name == pattern_name => fields.clone(),
                other => {
                    return Err(Diagnostic::error(
                        error_codes::TYPE_MISMATCH,
                        format!(
                            "Structured pattern '{}' cannot match {:?}",
                            pattern_name, other
                        ),
                    ))
                }
            };

            if fields.len() != field_types.len() {
                return Err(Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!(
                        "Pattern '{}' binds {} fields but the variant/type has {}",
                        pattern_name,
                        fields.len(),
                        field_types.len()
                    ),
                ));
            }

            let mut bindings = HashMap::new();
            for ((_, nested), field_type) in fields.iter().zip(field_types.iter()) {
                let nested_bindings = collect_pattern_bindings(nested, field_type, ctx)?;
                for (name, ty) in nested_bindings {
                    merge_pattern_binding(&mut bindings, &name, ty, ctx)?;
                }
            }
            Ok(bindings)
        }
        Pattern::Or(patterns) => {
            let mut expected: Option<HashMap<String, Type>> = None;
            for alternative in patterns {
                let current = collect_pattern_bindings(alternative, &resolved, ctx)?;
                if let Some(reference) = &expected {
                    if reference.len() != current.len()
                        || reference.keys().any(|name| !current.contains_key(name))
                    {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_MISMATCH,
                            "All alternatives of an or-pattern must bind the same names",
                        ));
                    }
                    for (name, reference_type) in reference {
                        let current_type = current.get(name).expect("binding set checked above");
                        ctx.unify(reference_type, current_type)?;
                    }
                } else {
                    expected = Some(current);
                }
            }
            Ok(expected.unwrap_or_default())
        }
    }
}

/// Synthesize (infer) the type of an expression.
/// Returns (Type, effects) where effects is a u8 bitmask.
fn synthesize_expr(
    expr: &Expr,
    symbols: &HashMap<String, Type>,
    ctx: &mut InferCtx,
    vis_ctx: &VisibilityCtx,
) -> Result<(Type, EffectSet), Diagnostic> {
    match expr {
        Expr::Number(_, _) => Ok((Type::Int, EffectSet::new())),
        Expr::Float(_, _) => Ok((Type::Float, EffectSet::new())),
        Expr::Char(_, _) => Ok((Type::Char, EffectSet::new())),
        Expr::Byte(_, _) => Ok((Type::Byte, EffectSet::new())),
        Expr::StringLit(_, _) => Ok((Type::String, EffectSet::new())),
        Expr::ByteString(_, _) => Ok((Type::Bytes, EffectSet::new())),
        Expr::Bool(_, _) => Ok((Type::Bool, EffectSet::new())),
        Expr::Var(name, _) => {
            vis_ctx.check_access(name)?;
            match symbols.get(name).cloned() {
                Some(t) => Ok((ctx.resolve(&t), EffectSet::new())),
                None => Err(Diagnostic::error(
                    error_codes::TYPE_INFERENCE_FAILED,
                    format!("Undefined variable '{}'", name),
                )),
            }
        }
        Expr::Borrow { mutable, inner, .. } => {
            match inner.as_ref() {
                Expr::Var(name, _) => {
                    if *mutable && !symbols.contains_key(&mutable_binding_symbol(name)) {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_MISMATCH,
                            format!("cannot mutably borrow immutable binding '{}'", name),
                        ));
                    }
                }
                Expr::FieldAccess { .. } if !*mutable => {}
                Expr::Deref { inner: parent, .. } => {
                    let (parent_ty, _) = synthesize_expr(parent, symbols, ctx, vis_ctx)?;
                    match ctx.resolve(&parent_ty) {
                        Type::Ref {
                            mutable: parent_mutable,
                            ..
                        } => {
                            if *mutable && !parent_mutable {
                                return Err(Diagnostic::error(
                                    error_codes::TYPE_MISMATCH,
                                    "cannot create mutable reborrow from shared reference",
                                ));
                            }
                        }
                        _ => {
                            return Err(Diagnostic::error(
                                error_codes::TYPE_MISMATCH,
                                "reborrow requires a reference parent",
                            ));
                        }
                    }
                }
                _ => {
                    return Err(Diagnostic::error(
                        error_codes::TYPE_MISMATCH,
                        if *mutable {
                            "mutable borrows currently require a mutable named local binding or mutable reborrow"
                        } else {
                            "safe borrows currently require a named local place or reborrow"
                        },
                    ));
                }
            }
            let (inner_ty, effects) = synthesize_expr(inner, symbols, ctx, vis_ctx)?;
            Ok((
                Type::Ref {
                    mutable: *mutable,
                    inner: Box::new(inner_ty),
                },
                effects,
            ))
        }
        Expr::Deref { inner, .. } => {
            let (reference_ty, effects) = synthesize_expr(inner, symbols, ctx, vis_ctx)?;
            match ctx.resolve(&reference_ty) {
                Type::Ref { inner, .. } => Ok((*inner, effects)),
                other => Err(Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!("cannot dereference non-reference type {:?}", other),
                )),
            }
        }
        Expr::Call(fname, args, _) => {
            match fname.as_str() {
                "Some" => {
                    if args.len() != 1 {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_INVALID_CONSTRUCTOR,
                            "Some expects exactly 1 argument",
                        ));
                    }
                    let (arg_ty, arg_ef) = synthesize_expr(&args[0], symbols, ctx, vis_ctx)?;
                    return Ok((Type::Option(Box::new(arg_ty)), arg_ef));
                }
                "None" => {
                    if !args.is_empty() {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_INVALID_CONSTRUCTOR,
                            "None expects 0 arguments",
                        ));
                    }
                    return Ok((Type::Option(Box::new(ctx.fresh_var())), EffectSet::new()));
                }
                "Ok" => {
                    if args.len() != 1 {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_INVALID_CONSTRUCTOR,
                            "Ok expects exactly 1 argument",
                        ));
                    }
                    let (arg_ty, arg_ef) = synthesize_expr(&args[0], symbols, ctx, vis_ctx)?;
                    return Ok((
                        Type::Result(Box::new(arg_ty), Box::new(ctx.fresh_var())),
                        arg_ef,
                    ));
                }
                "Err" => {
                    if args.len() != 1 {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_INVALID_CONSTRUCTOR,
                            "Err expects exactly 1 argument",
                        ));
                    }
                    let (arg_ty, arg_ef) = synthesize_expr(&args[0], symbols, ctx, vis_ctx)?;
                    return Ok((
                        Type::Result(Box::new(ctx.fresh_var()), Box::new(arg_ty)),
                        arg_ef,
                    ));
                }
                _ => {}
            }
            vis_ctx.check_access(fname)?;
            let ftype = symbols
                .get(fname)
                .ok_or_else(|| {
                    Diagnostic::error(
                        error_codes::TYPE_INFERENCE_FAILED,
                        format!("Undefined function '{}'", fname),
                    )
                })?
                .clone();
            match ftype {
                Type::Fn {
                    params,
                    ret,
                    effects,
                } => {
                    if params.len() != args.len() {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_MISSING_ARGUMENT,
                            format!(
                                "Function '{}' expected {} args, got {}",
                                fname,
                                params.len(),
                                args.len()
                            ),
                        ));
                    }
                    let mut gen_map: HashMap<String, Type> = HashMap::new();
                    let mut acc_effects = EffectSet::new();
                    for (i, a) in args.iter().enumerate() {
                        let (at, ef) = synthesize_expr(a, symbols, ctx, vis_ctx)?;
                        acc_effects.union_with(&ef);
                        match &params[i] {
                            Type::Generic(gname) => {
                                if let Some(existing) = gen_map.get(gname) {
                                    ctx.unify(existing, &at).map_err(|_e| {
                                        Diagnostic::error(
                                            error_codes::TYPE_INFERENCE_FAILED,
                                            format!(
                                                "Generic '{}' unified to conflicting types: {:?} vs {:?}",
                                                gname, existing, at
                                            ),
                                        )
                                    })?;
                                } else {
                                    gen_map.insert(gname.clone(), at);
                                }
                            }
                            Type::Var(_) => {}
                            pty => {
                                ctx.coerce_value_to_expected(&at, pty).map_err(|_| {
                                    Diagnostic::error(
                                        error_codes::TYPE_MISMATCH,
                                        format!(
                                            "Argument {} expected type {:?}, got {:?}",
                                            i, pty, at
                                        ),
                                    )
                                })?;
                            }
                        }
                    }
                    let inst_ret = substitute_type(&ret, &gen_map);
                    acc_effects.union_with(&effects);
                    Ok((inst_ret, acc_effects))
                }
                other => Err(Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!("Name '{}' is not callable (type {:?})", fname, other),
                )),
            }
        }
        Expr::BinaryOp {
            op, left, right, ..
        } => {
            let (lt, lf) = synthesize_expr(left, symbols, ctx, vis_ctx)?;
            let (rt, rf) = synthesize_expr(right, symbols, ctx, vis_ctx)?;
            let effects = lf | rf;
            let lt_orig = lt.clone();
            let rt_orig = rt.clone();
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
                    let lt_res = if let Type::Var(_) = &lt {
                        ctx.unify(&lt, &Type::Int)?;
                        ctx.resolve(&lt)
                    } else {
                        lt
                    };
                    let rt_res = if let Type::Var(_) = &rt {
                        ctx.unify(&rt, &Type::Int)?;
                        ctx.resolve(&rt)
                    } else {
                        rt
                    };
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
                    let lt_res = if let Type::Var(_) = &lt {
                        ctx.unify(&lt, &Type::Bool)?;
                        ctx.resolve(&lt)
                    } else {
                        lt
                    };
                    let rt_res = if let Type::Var(_) = &rt {
                        ctx.unify(&rt, &Type::Bool)?;
                        ctx.resolve(&rt)
                    } else {
                        rt
                    };
                    if lt_res == Type::Bool && rt_res == Type::Bool {
                        return Ok((Type::Bool, effects));
                    }
                }
                _ => {}
            }
            Err(Diagnostic::error(
                error_codes::TYPE_INVALID_OP,
                format!(
                    "Unsupported binary operation {:?} between {:?} and {:?}",
                    op, lt_orig, rt_orig
                ),
            ))
        }
        Expr::UnaryOp { op, inner, .. } => {
            let (it, ef) = synthesize_expr(inner, symbols, ctx, vis_ctx)?;
            let effects = ef;
            let it_orig = it.clone();
            match op {
                TokenKind::Minus => {
                    let it_res = if let Type::Var(_) = &it {
                        ctx.unify(&it, &Type::Int)?;
                        ctx.resolve(&it)
                    } else {
                        it
                    };
                    if it_res == Type::Int {
                        return Ok((Type::Int, effects));
                    }
                }
                TokenKind::Bang => {
                    let it_res = if let Type::Var(_) = &it {
                        ctx.unify(&it, &Type::Bool)?;
                        ctx.resolve(&it)
                    } else {
                        it
                    };
                    if it_res == Type::Bool {
                        return Ok((Type::Bool, effects));
                    }
                }
                _ => {}
            }
            Err(Diagnostic::error(
                error_codes::TYPE_INVALID_OP,
                format!("Unsupported unary operation {:?} on {:?}", op, it_orig),
            ))
        }
        Expr::FieldAccess { base, field, .. } => {
            let (bt, bf) = synthesize_expr(base, symbols, ctx, vis_ctx)?;
            if matches!(bt, Type::String | Type::Bytes) && field == "len" {
                return Ok((Type::Int, bf));
            }
            if let Type::Struct { name, .. } = ctx.resolve(&bt) {
                if let Some(field_ty) = symbols.get(&struct_field_symbol(&name, field)) {
                    return Ok((ctx.resolve(field_ty), bf));
                }
            }
            Err(Diagnostic::error(
                error_codes::TYPE_INVALID_FIELD_ACCESS,
                format!("Unknown field access '.{}' on {:?}", field, bt),
            ))
        }
        Expr::IfExpr {
            cond, then, else_, ..
        } => {
            let (cond_type, cond_ef) = synthesize_expr(cond, symbols, ctx, vis_ctx)?;
            let mut effects = cond_ef;
            if cond_type != Type::Bool {
                return Err(Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!("If condition must be bool, got {:?}", cond_type),
                ));
            }
            let (then_type, then_ef) = synthesize_expr(then, symbols, ctx, vis_ctx)?;
            let (else_type, else_ef) = synthesize_expr(else_, symbols, ctx, vis_ctx)?;
            effects.union_with(&then_ef);
            effects.union_with(&else_ef);
            ctx.unify(&then_type, &else_type)?;
            Ok((ctx.resolve(&then_type), effects))
        }
        Expr::Block(stmts, _) => {
            let mut local = symbols.clone();
            let mut result_type = Type::Unit;
            let mut effects = EffectSet::new();
            for stmt in stmts {
                match stmt {
                    Stmt::Let(name, type_ann, expr, _) => {
                        let (typ, ef) = synthesize_expr(expr, &local, ctx, vis_ctx)?;
                        let final_type = if let Some(ref ann) = type_ann {
                            let ann_type = parse_type_annotation_with_symbols(ann, symbols)?;
                            ctx.unify(&typ, &ann_type).map_err(|_| {
                                Diagnostic::error(
                                    error_codes::TYPE_MISMATCH,
                                    format!(
                                        "Type annotation mismatch for '{}': expected {:?}, found {:?}",
                                        name, ann_type, typ
                                    ),
                                )
                            })?;
                            ann_type
                        } else {
                            typ
                        };
                        local.insert(name.clone(), final_type);
                        effects.union_with(&ef);
                    }
                    Stmt::Return(expr, _) => {
                        let (t, ef) = synthesize_expr(expr, &local, ctx, vis_ctx)?;
                        effects.union_with(&ef);
                        result_type = t;
                        break;
                    }
                    Stmt::ExprStmt(expr, _) => {
                        let (t, ef) = synthesize_expr(expr, &local, ctx, vis_ctx)?;
                        effects.union_with(&ef);
                        result_type = t;
                    }
                    _ => {}
                }
            }
            Ok((result_type, effects))
        }
        Expr::Tuple(exprs, _) => {
            let mut fields = Vec::new();
            let mut effects = EffectSet::new();
            for e in exprs {
                let (field_type, field_effects) = synthesize_expr(e, symbols, ctx, vis_ctx)?;
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
        Expr::Array(exprs, _) => {
            if exprs.is_empty() {
                return Err(Diagnostic::error(
                    error_codes::TYPE_INFERENCE_FAILED,
                    "Empty array literal requires an explicit element type",
                ));
            }
            let mut fields = Vec::with_capacity(exprs.len());
            let mut effects = EffectSet::new();
            let mut element_type: Option<Type> = None;
            for expr in exprs {
                let (field_type, field_effects) = synthesize_expr(expr, symbols, ctx, vis_ctx)?;
                effects.union_with(&field_effects);
                if let Some(expected) = &element_type {
                    ctx.unify(&field_type, expected).map_err(|_| {
                        Diagnostic::error(
                            error_codes::TYPE_MISMATCH,
                            format!(
                                "Array elements must have one type; expected {:?}, found {:?}",
                                expected, field_type
                            ),
                        )
                    })?;
                } else {
                    element_type = Some(field_type.clone());
                }
                fields.push(field_type);
            }
            Ok((
                Type::Struct {
                    name: "Array".to_string(),
                    fields,
                    is_linear: false,
                },
                effects,
            ))
        }
        Expr::Match { expr, arms, .. } => {
            let (scrutinee_type, scrutinee_effects) = synthesize_expr(expr, symbols, ctx, vis_ctx)?;
            let mut effects = scrutinee_effects;
            let mut result_type: Option<Type> = None;

            // Check exhaustiveness before processing arms
            check_match_exhaustiveness(&scrutinee_type, arms, ctx)?;

            for arm in arms {
                let bindings = collect_pattern_bindings(&arm.pattern, &scrutinee_type, ctx)?;
                let mut arm_symbols = symbols.clone();
                arm_symbols.extend(bindings);
                if let Some(guard) = &arm.guard {
                    let (guard_type, guard_effects) =
                        synthesize_expr(guard, &arm_symbols, ctx, vis_ctx)?;
                    effects |= guard_effects;
                    if guard_type != Type::Bool {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_MISMATCH,
                            format!("Match guard must be bool, got {:?}", guard_type),
                        ));
                    }
                }

                let (arm_type, arm_effects) =
                    synthesize_expr(&arm.body, &arm_symbols, ctx, vis_ctx)?;
                effects |= arm_effects;

                if let Some(existing) = &result_type {
                    ctx.unify(existing, &arm_type)?;
                } else {
                    result_type = Some(arm_type);
                }
            }

            Ok((result_type.unwrap_or(Type::Unit), effects))
        }
        Expr::Interpolated(frags, _) => {
            let mut effects = EffectSet::new();
            for frag in frags.iter() {
                match frag {
                    InterpolatedFragment::Literal(_, _) => {}
                    InterpolatedFragment::Expr(e) => {
                        let (_t, ef) = synthesize_expr(e, symbols, ctx, vis_ctx)?;
                        effects.union_with(&ef);
                    }
                }
            }
            Ok((Type::String, effects))
        }
        Expr::Index(base, index, _) => {
            let (base_type, base_effects) = synthesize_expr(base, symbols, ctx, vis_ctx)?;
            if let Expr::Range { start, end, .. } = index.as_ref() {
                let (start_type, start_effects) = synthesize_expr(start, symbols, ctx, vis_ctx)?;
                let (end_type, end_effects) = synthesize_expr(end, symbols, ctx, vis_ctx)?;
                if start_type != Type::Int || end_type != Type::Int {
                    return Err(Diagnostic::error(
                        error_codes::TYPE_MISMATCH,
                        "Slice range bounds must both be int",
                    ));
                }
                let effects = base_effects | start_effects | end_effects;
                let element_type = match ctx.resolve(&base_type) {
                    Type::Struct { name, fields, .. }
                        if matches!(name.as_str(), "Array" | "Slice") =>
                    {
                        fields.first().cloned().ok_or_else(|| {
                            Diagnostic::error(
                                error_codes::TYPE_INVALID_OP,
                                "Cannot slice a zero-element aggregate",
                            )
                        })?
                    }
                    other => {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_INVALID_OP,
                            format!("Slice ranges are not implemented for {:?}", other),
                        ));
                    }
                };
                return Ok((
                    Type::Struct {
                        name: "Slice".to_string(),
                        fields: vec![element_type],
                        is_linear: false,
                    },
                    effects,
                ));
            }
            let (index_type, index_effects) = synthesize_expr(index, symbols, ctx, vis_ctx)?;
            let effects = base_effects | index_effects;

            if index_type != Type::Int {
                return Err(Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!("Index expression must be int, got {:?}", index_type),
                ));
            }

            match ctx.resolve(&base_type) {
                Type::Struct { name, fields, .. } if name == "Tuple" => {
                    if let Expr::Number(n, _) = index.as_ref() {
                        if *n < 0 {
                            return Err(Diagnostic::error(
                                error_codes::TYPE_INVALID_OP,
                                "Tuple index must be non-negative",
                            ));
                        }
                        let idx = *n as usize;
                        match fields.get(idx).cloned() {
                            Some(field_type) => Ok((field_type, effects)),
                            None => Err(Diagnostic::error(
                                error_codes::TYPE_MISMATCH,
                                format!("Tuple index {} out of bounds", idx),
                            )),
                        }
                    } else {
                        Err(Diagnostic::error(
                            error_codes::TYPE_INVALID_OP,
                            "Tuple indexing requires a constant integer index",
                        ))
                    }
                }
                Type::Struct { name, fields, .. } if matches!(name.as_str(), "Array" | "Slice") => {
                    fields
                        .first()
                        .cloned()
                        .map(|field_type| (field_type, effects))
                        .ok_or_else(|| {
                            Diagnostic::error(
                                error_codes::TYPE_INVALID_OP,
                                "Cannot index an empty array",
                            )
                        })
                }
                Type::Bytes => Ok((Type::Byte, effects)),
                Type::String => Err(Diagnostic::error(
                    error_codes::TYPE_INVALID_OP,
                    "String byte indexing is not qualified: UTF-8 indexing requires an explicit byte/character view",
                )),
                other => Err(Diagnostic::error(
                    error_codes::TYPE_INVALID_OP,
                    format!("Index expressions not yet implemented for {:?}", other),
                )),
            }
        }
        Expr::Range { .. } => Ok((
            Type::Struct {
                name: "Vector".to_string(),
                fields: vec![Type::Int],
                is_linear: true,
            },
            EffectSet::new(),
        )),
        Expr::Lambda { params, body, .. } => {
            let mut local_symbols = symbols.clone();
            for (name, ty_opt) in params {
                let ptype = match ty_opt {
                    Some(t) => parse_type_annotation_with_symbols(t, symbols)?,
                    None => ctx.fresh_var(),
                };
                local_symbols.insert(name.clone(), ptype);
            }
            let (ret_type, effects) = synthesize_expr(body, &local_symbols, ctx, vis_ctx)?;
            let param_types: Vec<Type> = params
                .iter()
                .map(|(name, _)| {
                    local_symbols
                        .get(name)
                        .cloned()
                        .unwrap_or_else(|| ctx.fresh_var())
                })
                .collect();
            Ok((
                Type::Fn {
                    params: param_types,
                    ret: Box::new(ret_type),
                    effects: effects.clone(),
                },
                effects,
            ))
        }
        Expr::Await(inner, _) => {
            let (ty, effects) = synthesize_expr(inner, symbols, ctx, vis_ctx)?;
            if let Type::Struct { name, fields, .. } = &ty {
                if name == "Future" && fields.len() == 1 {
                    return Ok((fields[0].clone(), effects));
                }
            }
            Ok((ty, effects))
        }
        Expr::Try(inner, _) => {
            let (ty, effects) = synthesize_expr(inner, symbols, ctx, vis_ctx)?;
            if let Type::Enum { name, variants, .. } = &ty {
                if name == "Result" && variants.len() == 2 && !variants[0].fields.is_empty() {
                    return Ok((variants[0].fields[0].clone(), effects));
                }
            }
            if let Type::Result(ok_ty, _) = &ty {
                return Ok((ok_ty.as_ref().clone(), effects));
            }
            Ok((ty, effects))
        }
        Expr::StructLit { name, fields, .. } => {
            validate_struct_literal(name, fields, symbols, ctx, vis_ctx)
        }
    }
}

/// Check an expression against an expected type.
/// Returns the effects bitmask on success.
/// This is the "checking" direction of bidirectional type checking:
/// we know what type we expect and verify the expression conforms to it.
fn check_expr(
    expr: &Expr,
    expected: &Type,
    symbols: &HashMap<String, Type>,
    ctx: &mut InferCtx,
    vis_ctx: &VisibilityCtx,
) -> Result<EffectSet, Diagnostic> {
    match expr {
        Expr::Number(_, _) => {
            ctx.unify(&Type::Int, expected).map_err(|_| {
                Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!("Expected type {:?}, found integer literal", expected),
                )
            })?;
            Ok(EffectSet::new())
        }
        Expr::Float(_, _) => {
            ctx.unify(&Type::Float, expected).map_err(|_| {
                Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!("Expected type {:?}, found float literal", expected),
                )
            })?;
            Ok(EffectSet::new())
        }
        Expr::Char(_, _) => {
            ctx.unify(&Type::Char, expected).map_err(|_| {
                Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!("Expected type {:?}, found char literal", expected),
                )
            })?;
            Ok(EffectSet::new())
        }
        Expr::Byte(_, _) => {
            ctx.unify(&Type::Byte, expected).map_err(|_| {
                Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!("Expected type {:?}, found byte literal", expected),
                )
            })?;
            Ok(EffectSet::new())
        }
        Expr::StringLit(_, _) => {
            ctx.unify(&Type::String, expected).map_err(|_| {
                Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!("Expected type {:?}, found string literal", expected),
                )
            })?;
            Ok(EffectSet::new())
        }
        Expr::ByteString(_, _) => {
            ctx.unify(&Type::Bytes, expected).map_err(|_| {
                Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!("Expected type {:?}, found byte-string literal", expected),
                )
            })?;
            Ok(EffectSet::new())
        }
        Expr::Bool(_, _) => {
            ctx.unify(&Type::Bool, expected).map_err(|_| {
                Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!("Expected type {:?}, found bool literal", expected),
                )
            })?;
            Ok(EffectSet::new())
        }
        Expr::Var(name, _) => {
            let actual = symbols.get(name).ok_or_else(|| {
                Diagnostic::error(
                    error_codes::TYPE_INFERENCE_FAILED,
                    format!("Undefined variable '{}'", name),
                )
            })?;
            ctx.unify(&ctx.resolve(actual), expected).map_err(|_| {
                Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!(
                        "Type mismatch for '{}': expected {:?}, found {:?}",
                        name,
                        expected,
                        ctx.resolve(actual)
                    ),
                )
            })?;
            Ok(EffectSet::new())
        }
        Expr::BinaryOp {
            op, left, right, ..
        } => {
            let (lt, lf) = synthesize_expr(left, symbols, ctx, vis_ctx)?;
            let (rt, rf) = synthesize_expr(right, symbols, ctx, vis_ctx)?;
            let effects = lf.union(&rf);
            let lt_orig = lt.clone();
            let rt_orig = rt.clone();
            let result_type = match op {
                TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Percent => {
                    ctx.unify(&lt, &Type::Int).map_err(|_| {
                        Diagnostic::error(
                            error_codes::TYPE_INVALID_OP,
                            format!("Arithmetic operation requires int, got {:?}", lt_orig),
                        )
                    })?;
                    ctx.unify(&rt, &Type::Int).map_err(|_| {
                        Diagnostic::error(
                            error_codes::TYPE_INVALID_OP,
                            format!("Arithmetic operation requires int, got {:?}", rt_orig),
                        )
                    })?;
                    Type::Int
                }
                TokenKind::EqEq
                | TokenKind::NotEq
                | TokenKind::Lt
                | TokenKind::LtEq
                | TokenKind::Gt
                | TokenKind::GtEq => {
                    ctx.unify(&lt, &Type::Int).map_err(|_| {
                        Diagnostic::error(
                            error_codes::TYPE_INVALID_OP,
                            format!("Comparison operation requires int, got {:?}", lt_orig),
                        )
                    })?;
                    ctx.unify(&rt, &Type::Int).map_err(|_| {
                        Diagnostic::error(
                            error_codes::TYPE_INVALID_OP,
                            format!("Comparison operation requires int, got {:?}", rt_orig),
                        )
                    })?;
                    Type::Bool
                }
                TokenKind::AndAnd | TokenKind::OrOr => {
                    ctx.unify(&lt, &Type::Bool).map_err(|_| {
                        Diagnostic::error(
                            error_codes::TYPE_INVALID_OP,
                            format!("Logical operation requires bool, got {:?}", lt_orig),
                        )
                    })?;
                    ctx.unify(&rt, &Type::Bool).map_err(|_| {
                        Diagnostic::error(
                            error_codes::TYPE_INVALID_OP,
                            format!("Logical operation requires bool, got {:?}", rt_orig),
                        )
                    })?;
                    Type::Bool
                }
                _ => {
                    return Err(Diagnostic::error(
                        error_codes::TYPE_INVALID_OP,
                        format!("Unsupported binary operation {:?}", op),
                    ));
                }
            };
            ctx.unify(&result_type, expected).map_err(|_| {
                Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!(
                        "Binary operation result type {:?} does not match expected {:?}",
                        result_type, expected
                    ),
                )
            })?;
            Ok(effects)
        }
        Expr::UnaryOp { op, inner, .. } => {
            let (it, ef) = synthesize_expr(inner, symbols, ctx, vis_ctx)?;
            let effects = ef;
            let it_orig = it.clone();
            let result_type = match op {
                TokenKind::Minus => {
                    ctx.unify(&it, &Type::Int).map_err(|_| {
                        Diagnostic::error(
                            error_codes::TYPE_INVALID_OP,
                            format!("Unary negation requires int, got {:?}", it_orig),
                        )
                    })?;
                    Type::Int
                }
                TokenKind::Bang => {
                    ctx.unify(&it, &Type::Bool).map_err(|_| {
                        Diagnostic::error(
                            error_codes::TYPE_INVALID_OP,
                            format!("Logical not requires bool, got {:?}", it_orig),
                        )
                    })?;
                    Type::Bool
                }
                _ => {
                    return Err(Diagnostic::error(
                        error_codes::TYPE_INVALID_OP,
                        format!("Unsupported unary operation {:?}", op),
                    ));
                }
            };
            ctx.unify(&result_type, expected).map_err(|_| {
                Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!(
                        "Unary operation result type {:?} does not match expected {:?}",
                        result_type, expected
                    ),
                )
            })?;
            Ok(effects)
        }
        Expr::FieldAccess { base, field, .. } => {
            let (bt, bf) = synthesize_expr(base, symbols, ctx, vis_ctx)?;
            let result_type = if matches!(bt, Type::String | Type::Bytes) && field == "len" {
                Type::Int
            } else if let Type::Struct { name, .. } = ctx.resolve(&bt) {
                symbols
                    .get(&struct_field_symbol(&name, field))
                    .cloned()
                    .ok_or_else(|| {
                        Diagnostic::error(
                            error_codes::TYPE_INVALID_FIELD_ACCESS,
                            format!("Unknown field access '.{}' on {:?}", field, bt),
                        )
                    })?
            } else {
                return Err(Diagnostic::error(
                    error_codes::TYPE_INVALID_FIELD_ACCESS,
                    format!("Unknown field access '.{}' on {:?}", field, bt),
                ));
            };
            ctx.unify(&result_type, expected).map_err(|_| {
                Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!(
                        "Field access type {:?} does not match expected {:?}",
                        result_type, expected
                    ),
                )
            })?;
            Ok(bf)
        }
        Expr::IfExpr {
            cond, then, else_, ..
        } => {
            let (cond_type, cond_ef) = synthesize_expr(cond, symbols, ctx, vis_ctx)?;
            let mut effects = cond_ef;
            if cond_type != Type::Bool {
                return Err(Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!("If condition must be bool, got {:?}", cond_type),
                ));
            }
            let then_ef = check_expr(then, expected, symbols, ctx, vis_ctx)?;
            let else_ef = check_expr(else_, expected, symbols, ctx, vis_ctx)?;
            effects |= then_ef | else_ef;
            Ok(effects)
        }
        Expr::Match {
            expr: scrutinee,
            arms,
            ..
        } => {
            let (scrutinee_type, scrutinee_effects) =
                synthesize_expr(scrutinee, symbols, ctx, vis_ctx)?;
            let mut effects = scrutinee_effects;

            // Check exhaustiveness before processing arms
            check_match_exhaustiveness(&scrutinee_type, arms, ctx)?;

            for arm in arms {
                let bindings = collect_pattern_bindings(&arm.pattern, &scrutinee_type, ctx)?;
                let mut arm_symbols = symbols.clone();
                arm_symbols.extend(bindings);
                if let Some(guard) = &arm.guard {
                    let (guard_type, guard_effects) =
                        synthesize_expr(guard, &arm_symbols, ctx, vis_ctx)?;
                    effects |= guard_effects;
                    if guard_type != Type::Bool {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_MISMATCH,
                            format!("Match guard must be bool, got {:?}", guard_type),
                        ));
                    }
                }
                let arm_ef = check_expr(&arm.body, expected, &arm_symbols, ctx, vis_ctx)?;
                effects |= arm_ef;
            }
            Ok(effects)
        }
        Expr::Block(stmts, _span) => {
            let mut local = symbols.clone();
            let mut result_type = Type::Unit;
            let mut effects = EffectSet::new();
            for stmt in stmts {
                match stmt {
                    Stmt::Let(name, type_ann, expr, _) | Stmt::LetMut(name, type_ann, expr, _) => {
                        if let Some(ref ann) = type_ann {
                            let ann_type = parse_type_annotation_with_symbols(ann, symbols)?;
                            check_expr(expr, &ann_type, &local, ctx, vis_ctx).map_err(|e| {
                                Diagnostic::error(
                                    error_codes::TYPE_MISMATCH,
                                    format!(
                                        "Type annotation mismatch for '{}': expected {:?}, but expression has incompatible type",
                                        name, ann_type
                                    ),
                                )
                                .with_note(format!("Original error: {}", e.message))
                            })?;
                            local.insert(name.clone(), ann_type);
                        } else {
                            let (typ, ef) = synthesize_expr(expr, &local, ctx, vis_ctx)?;
                            local.insert(name.clone(), typ);
                            effects.union_with(&ef);
                        }
                    }
                    Stmt::LetLinear(name, type_ann, expr, _) => {
                        if let Some(ref ann) = type_ann {
                            let ann_type = parse_type_annotation_with_symbols(ann, symbols)?;
                            check_expr(expr, &ann_type, &local, ctx, vis_ctx).map_err(|e| {
                                Diagnostic::error(
                                    error_codes::TYPE_MISMATCH,
                                    format!(
                                        "Type annotation mismatch for '{}': expected {:?}, but expression has incompatible type",
                                        name, ann_type
                                    ),
                                )
                                .with_note(format!("Original error: {}", e.message))
                            })?;
                            local.insert(name.clone(), ann_type);
                        } else {
                            let (typ, ef) = synthesize_expr(expr, &local, ctx, vis_ctx)?;
                            local.insert(name.clone(), typ);
                            effects.union_with(&ef);
                        }
                    }
                    Stmt::Return(expr, _) => {
                        let (t, ef) = synthesize_expr(expr, &local, ctx, vis_ctx)?;
                        effects.union_with(&ef);
                        result_type = t;
                        break;
                    }
                    Stmt::ExprStmt(expr, _) => {
                        let (t, ef) = synthesize_expr(expr, &local, ctx, vis_ctx)?;
                        effects.union_with(&ef);
                        result_type = t;
                    }
                    _ => {}
                }
            }
            ctx.unify(&result_type, expected).map_err(|_| {
                Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!(
                        "Block expression expected type {:?}, found {:?}",
                        expected, result_type
                    ),
                )
            })?;
            Ok(effects)
        }
        Expr::Tuple(exprs, _) => {
            let mut fields = Vec::new();
            let mut effects = EffectSet::new();
            for e in exprs {
                let (field_type, field_effects) = synthesize_expr(e, symbols, ctx, vis_ctx)?;
                effects |= field_effects;
                fields.push(field_type);
            }
            let tuple_type = Type::Struct {
                name: "Tuple".to_string(),
                fields,
                is_linear: false,
            };
            ctx.unify(&tuple_type, expected).map_err(|_| {
                Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!(
                        "Tuple type {:?} does not match expected {:?}",
                        tuple_type, expected
                    ),
                )
            })?;
            Ok(effects)
        }
        Expr::Array(_, _) => {
            let (array_type, effects) = synthesize_expr(expr, symbols, ctx, vis_ctx)?;
            ctx.unify(&array_type, expected).map_err(|_| {
                Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!(
                        "Array type {:?} does not match expected {:?}",
                        array_type, expected
                    ),
                )
            })?;
            Ok(effects)
        }
        Expr::Index(base, index, _) => {
            let (base_type, base_effects) = synthesize_expr(base, symbols, ctx, vis_ctx)?;
            if matches!(index.as_ref(), Expr::Range { .. }) {
                let (actual, effects) = synthesize_expr(expr, symbols, ctx, vis_ctx)?;
                ctx.unify(&actual, expected).map_err(|_| {
                    Diagnostic::error(
                        error_codes::TYPE_MISMATCH,
                        format!(
                            "Slice expression type {:?} does not match expected {:?}",
                            actual, expected
                        ),
                    )
                })?;
                return Ok(effects | base_effects);
            }
            let (index_type, index_effects) = synthesize_expr(index, symbols, ctx, vis_ctx)?;
            let effects = base_effects | index_effects;

            if index_type != Type::Int {
                return Err(Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!("Index expression must be int, got {:?}", index_type),
                ));
            }

            let result_type = match ctx.resolve(&base_type) {
                Type::Struct { name, fields, .. } if name == "Tuple" => {
                    if let Expr::Number(n, _) = index.as_ref() {
                        if *n < 0 {
                            return Err(Diagnostic::error(
                                error_codes::TYPE_INVALID_OP,
                                "Tuple index must be non-negative",
                            ));
                        }
                        let idx = *n as usize;
                        match fields.get(idx).cloned() {
                            Some(field_type) => field_type,
                            None => {
                                return Err(Diagnostic::error(
                                    error_codes::TYPE_MISMATCH,
                                    format!("Tuple index {} out of bounds", idx),
                                ));
                            }
                        }
                    } else {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_INVALID_OP,
                            "Tuple indexing requires a constant integer index",
                        ));
                    }
                }
                Type::Struct { name, fields, .. } if matches!(name.as_str(), "Array" | "Slice") => {
                    fields.first().cloned().ok_or_else(|| {
                        Diagnostic::error(
                            error_codes::TYPE_INVALID_OP,
                            "Cannot index an empty array",
                        )
                    })?
                }
                Type::String => Type::String,
                other => {
                    return Err(Diagnostic::error(
                        error_codes::TYPE_INVALID_OP,
                        format!("Index expressions not yet implemented for {:?}", other),
                    ));
                }
            };
            ctx.unify(&result_type, expected).map_err(|_| {
                Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!(
                        "Index expression type {:?} does not match expected {:?}",
                        result_type, expected
                    ),
                )
            })?;
            Ok(effects)
        }
        Expr::Range { .. } => {
            let range_type = Type::Struct {
                name: "Vector".to_string(),
                fields: vec![Type::Int],
                is_linear: true,
            };
            ctx.unify(&range_type, expected).map_err(|_| {
                Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!(
                        "Range type {:?} does not match expected {:?}",
                        range_type, expected
                    ),
                )
            })?;
            Ok(EffectSet::new())
        }
        Expr::Lambda { params, body, .. } => {
            if let Type::Fn {
                params: expected_params,
                ret: expected_ret,
                effects: _expected_effects,
            } = expected
            {
                let mut local_symbols = symbols.clone();
                let mut param_types: Vec<Type> = Vec::new();
                let mut effects = EffectSet::new();

                for (i, (name, ty_opt)) in params.iter().enumerate() {
                    let ptype = if let Some(t) = ty_opt {
                        let ann_type = parse_type_annotation(t)?;
                        if let Some(exp_p) = expected_params.get(i) {
                            ctx.unify(&ann_type, exp_p).map_err(|_| {
                                Diagnostic::error(
                                    error_codes::TYPE_MISMATCH,
                                    format!(
                                        "Lambda parameter '{}' annotated as {:?} but expected {:?}",
                                        name, ann_type, exp_p
                                    ),
                                )
                            })?;
                            exp_p.clone()
                        } else {
                            ann_type
                        }
                    } else if let Some(exp_p) = expected_params.get(i) {
                        exp_p.clone()
                    } else {
                        ctx.fresh_var()
                    };
                    param_types.push(ptype.clone());
                    local_symbols.insert(name.clone(), ptype);
                }

                let body_ef = check_expr(body, expected_ret, &local_symbols, ctx, vis_ctx)?;
                effects.union_with(&body_ef);

                let inferred_ret =
                    if let Type::Fn { ret, .. } = synthesize_expr(expr, symbols, ctx, vis_ctx)?.0 {
                        ret
                    } else {
                        Box::new(Type::Unit)
                    };

                ctx.unify(&inferred_ret, expected_ret).map_err(|_| {
                    Diagnostic::error(
                        error_codes::TYPE_MISMATCH,
                        format!(
                            "Lambda return type {:?} does not match expected {:?}",
                            inferred_ret, expected_ret
                        ),
                    )
                })?;
                Ok(effects)
            } else {
                let (synthesized, ef) = synthesize_expr(expr, symbols, ctx, vis_ctx)?;
                ctx.unify(&synthesized, expected).map_err(|_| {
                    Diagnostic::error(
                        error_codes::TYPE_MISMATCH,
                        format!("Expected type {:?}, found {:?}", expected, synthesized),
                    )
                })?;
                Ok(ef)
            }
        }
        Expr::Call(_, _, _) => {
            let (call_type, call_ef) = synthesize_expr(expr, symbols, ctx, vis_ctx)?;
            ctx.unify(&call_type, expected).map_err(|_| {
                Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!(
                        "Call result type {:?} does not match expected {:?}",
                        call_type, expected
                    ),
                )
            })?;
            Ok(call_ef)
        }
        Expr::Interpolated(_, _) => {
            let (str_type, str_ef) = synthesize_expr(expr, symbols, ctx, vis_ctx)?;
            ctx.unify(&str_type, expected).map_err(|_| {
                Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!(
                        "Interpolated string type {:?} does not match expected {:?}",
                        str_type, expected
                    ),
                )
            })?;
            Ok(str_ef)
        }
        Expr::Await(inner, _) => {
            let (ty, effects) = synthesize_expr(inner, symbols, ctx, vis_ctx)?;
            let result_type = if let Type::Struct { name, fields, .. } = &ty {
                if name == "Future" && fields.len() == 1 {
                    fields[0].clone()
                } else {
                    ty
                }
            } else {
                ty
            };
            ctx.unify(&result_type, expected).map_err(|_| {
                Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!(
                        "Await result type {:?} does not match expected {:?}",
                        result_type, expected
                    ),
                )
            })?;
            Ok(effects)
        }
        Expr::Try(inner, _) => {
            let (ty, effects) = synthesize_expr(inner, symbols, ctx, vis_ctx)?;
            let result_type = if let Type::Enum { name, variants, .. } = &ty {
                if name == "Result" && variants.len() == 2 && !variants[0].fields.is_empty() {
                    variants[0].fields[0].clone()
                } else {
                    ty
                }
            } else if let Type::Result(ok_ty, _) = &ty {
                ok_ty.as_ref().clone()
            } else {
                ty
            };
            ctx.unify(&result_type, expected).map_err(|_| {
                Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!(
                        "Try result type {:?} does not match expected {:?}",
                        result_type, expected
                    ),
                )
            })?;
            Ok(effects)
        }
        Expr::Borrow { .. } | Expr::Deref { .. } => {
            let (actual, effects) = synthesize_expr(expr, symbols, ctx, vis_ctx)?;
            ctx.unify(&actual, expected).map_err(|_| {
                Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!(
                        "Reference expression type {:?} does not match expected {:?}",
                        actual, expected
                    ),
                )
            })?;
            Ok(effects)
        }
        Expr::StructLit { name, fields, .. } => {
            let (struct_type, effects) =
                validate_struct_literal(name, fields, symbols, ctx, vis_ctx)?;
            ctx.unify(&struct_type, expected).map_err(|_| {
                Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!(
                        "Struct literal type {:?} does not match expected {:?}",
                        struct_type, expected
                    ),
                )
            })?;
            Ok(effects)
        }
    }
}

pub fn type_check_program(prog: &Program) -> Result<HashMap<String, Type>, Diagnostic> {
    if let Err(errs) = resolver::resolve_program(prog) {
        let joined: Vec<String> = errs.iter().map(|d| d.message.clone()).collect();
        return Err(Diagnostic::error(
            error_codes::TYPE_INFERENCE_FAILED,
            joined.join("; "),
        ));
    }

    check_linear_types(prog)?;

    let mut symbols: HashMap<String, Type> = HashMap::new();
    let mut vis_ctx = VisibilityCtx::new();
    let mut struct_bounds: HashMap<String, Vec<(String, Vec<String>)>> = HashMap::new();

    for s in &prog.stmts {
        match s {
            Stmt::Struct {
                name, visibility, ..
            } => {
                vis_ctx.register(name, visibility.clone());
            }
            Stmt::Enum {
                name, visibility, ..
            } => {
                vis_ctx.register(name, visibility.clone());
            }
            Stmt::Fn {
                name, visibility, ..
            } => {
                vis_ctx.register(name, visibility.clone());
            }
            Stmt::Impl {
                target,
                type_params,
                ..
            } => {
                if !type_params.is_empty() {
                    struct_bounds.insert(target.clone(), type_params.clone());
                }
            }
            Stmt::TypeAlias {
                name, type_params, ..
            } if !type_params.is_empty() => {
                struct_bounds.insert(name.clone(), type_params.clone());
            }
            _ => {}
        }
    }

    symbols.insert(
        "str_len".to_string(),
        Type::Fn {
            params: vec![Type::String],
            ret: Box::new(Type::Int),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "string_concat".to_string(),
        Type::Fn {
            params: vec![Type::String, Type::String],
            ret: Box::new(Type::String),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "string_eq".to_string(),
        Type::Fn {
            params: vec![Type::String, Type::String],
            ret: Box::new(Type::Bool),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "string_push_char".to_string(),
        Type::Fn {
            params: vec![Type::String, Type::String],
            ret: Box::new(Type::String),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "string_substr".to_string(),
        Type::Fn {
            params: vec![Type::String, Type::Int, Type::Int],
            ret: Box::new(Type::String),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "string_starts_with".to_string(),
        Type::Fn {
            params: vec![Type::String, Type::String],
            ret: Box::new(Type::Bool),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "string_ends_with".to_string(),
        Type::Fn {
            params: vec![Type::String, Type::String],
            ret: Box::new(Type::Bool),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "string_find".to_string(),
        Type::Fn {
            params: vec![Type::String, Type::String],
            ret: Box::new(Type::Int),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "string_trim".to_string(),
        Type::Fn {
            params: vec![Type::String],
            ret: Box::new(Type::String),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "int_to_string".to_string(),
        Type::Fn {
            params: vec![Type::Int],
            ret: Box::new(Type::String),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "string_to_int".to_string(),
        Type::Fn {
            params: vec![Type::String],
            ret: Box::new(Type::Int),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "int_abs".to_string(),
        Type::Fn {
            params: vec![Type::Int],
            ret: Box::new(Type::Int),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "option_is_some".to_string(),
        Type::Fn {
            params: vec![Type::Generic("Option".to_string())],
            ret: Box::new(Type::Bool),
            effects: EffectSet::new(),
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
            effects: EffectSet::new(),
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
            effects: EffectSet::new(),
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
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "result_is_ok".to_string(),
        Type::Fn {
            params: vec![Type::Generic("Result".to_string())],
            ret: Box::new(Type::Bool),
            effects: EffectSet::new(),
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
            effects: EffectSet::new(),
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
            effects: EffectSet::new(),
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
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "panic".to_string(),
        Type::Fn {
            params: vec![Type::String],
            ret: Box::new(Type::Never),
            effects: EffectSet::with_panic(),
        },
    );
    symbols.insert(
        "vector_new".to_string(),
        Type::Fn {
            params: vec![],
            ret: Box::new(Type::Generic("Vector".to_string())),
            effects: EffectSet::new(),
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
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "vector_len".to_string(),
        Type::Fn {
            params: vec![Type::Generic("Vector".to_string())],
            ret: Box::new(Type::Int),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "vector_get".to_string(),
        Type::Fn {
            params: vec![Type::Generic("Vector".to_string()), Type::Int],
            ret: Box::new(Type::Generic("T".to_string())),
            effects: EffectSet::new(),
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
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "vector_pop".to_string(),
        Type::Fn {
            params: vec![Type::Generic("Vector".to_string())],
            ret: Box::new(Type::Generic("T".to_string())),
            effects: EffectSet::new(),
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
            effects: EffectSet::new(),
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
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "vector_remove".to_string(),
        Type::Fn {
            params: vec![Type::Generic("Vector".to_string()), Type::Int],
            ret: Box::new(Type::Generic("T".to_string())),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "vector_clear".to_string(),
        Type::Fn {
            params: vec![Type::Generic("Vector".to_string())],
            ret: Box::new(Type::Unit),
            effects: EffectSet::new(),
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
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "vector_capacity".to_string(),
        Type::Fn {
            params: vec![Type::Generic("Vector".to_string())],
            ret: Box::new(Type::Int),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "vector_reserve".to_string(),
        Type::Fn {
            params: vec![Type::Generic("Vector".to_string()), Type::Int],
            ret: Box::new(Type::Unit),
            effects: EffectSet::new(),
        },
    );

    symbols.insert(
        "hashmap_new".to_string(),
        Type::Fn {
            params: vec![],
            ret: Box::new(Type::Generic("HashMap".to_string())),
            effects: EffectSet::new(),
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
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "hashmap_get".to_string(),
        Type::Fn {
            params: vec![Type::Generic("HashMap".to_string()), Type::String],
            ret: Box::new(Type::Generic("T".to_string())),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "hashmap_contains".to_string(),
        Type::Fn {
            params: vec![Type::Generic("HashMap".to_string()), Type::String],
            ret: Box::new(Type::Bool),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "hashmap_remove".to_string(),
        Type::Fn {
            params: vec![Type::Generic("HashMap".to_string()), Type::String],
            ret: Box::new(Type::Int),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "hashmap_len".to_string(),
        Type::Fn {
            params: vec![Type::Generic("HashMap".to_string())],
            ret: Box::new(Type::Int),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "string_replace".to_string(),
        Type::Fn {
            params: vec![Type::String, Type::String, Type::String],
            ret: Box::new(Type::String),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "int_pow".to_string(),
        Type::Fn {
            params: vec![Type::Int, Type::Int],
            ret: Box::new(Type::Int),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "int_div".to_string(),
        Type::Fn {
            params: vec![Type::Int, Type::Int],
            ret: Box::new(Type::Int),
            effects: EffectSet::new(),
        },
    );

    symbols.insert(
        "hashset_new".to_string(),
        Type::Fn {
            params: vec![],
            ret: Box::new(Type::Generic("HashSet".to_string())),
            effects: EffectSet::new(),
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
            effects: EffectSet::new(),
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
            effects: EffectSet::new(),
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
            effects: EffectSet::new(),
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
            effects: EffectSet::new(),
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
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "hashset_len".to_string(),
        Type::Fn {
            params: vec![Type::Generic("HashSet".to_string())],
            ret: Box::new(Type::Int),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "hashset_clear".to_string(),
        Type::Fn {
            params: vec![Type::Generic("HashSet".to_string())],
            ret: Box::new(Type::Unit),
            effects: EffectSet::new(),
        },
    );
    symbols.insert(
        "hashmap_clear".to_string(),
        Type::Fn {
            params: vec![Type::Generic("HashMap".to_string())],
            ret: Box::new(Type::Unit),
            effects: EffectSet::new(),
        },
    );

    // Register aggregate nominal identities before resolving their fields so forward
    // references can be recognized without fabricating a generic type. Named structs/enums
    // unify by declaration identity; Tuple/Array/Slice remain structural.
    for s in &prog.stmts {
        match s {
            Stmt::Struct {
                name, is_linear, ..
            } => {
                symbols.insert(
                    name.clone(),
                    Type::Struct {
                        name: name.clone(),
                        fields: Vec::new(),
                        is_linear: *is_linear,
                    },
                );
            }
            Stmt::Enum {
                name, is_sealed, ..
            } => {
                symbols.insert(
                    name.clone(),
                    Type::Enum {
                        name: name.clone(),
                        variants: Vec::new(),
                        is_sealed: *is_sealed,
                    },
                );
            }
            _ => {}
        }
    }

    // Resolve aggregate declarations without a fresh-variable fallback. Unknown field or
    // payload annotations are diagnostics; known nominal aggregate names may be forward
    // references and are preserved by nominal identity.
    for s in &prog.stmts {
        match s {
            Stmt::Struct {
                name,
                fields,
                is_linear,
                ..
            } => {
                let mut field_types = Vec::with_capacity(fields.len());
                for (_, ty) in fields {
                    field_types.push(parse_type_annotation_with_symbols(ty, &symbols)?);
                }
                symbols.insert(
                    name.clone(),
                    Type::Struct {
                        name: name.clone(),
                        fields: field_types.clone(),
                        is_linear: *is_linear,
                    },
                );
                for ((field_name, _), field_ty) in fields.iter().zip(field_types) {
                    symbols.insert(struct_field_symbol(name, field_name), field_ty);
                }
            }
            Stmt::Enum {
                name,
                variants,
                is_sealed,
                ..
            } => {
                let mut enum_variants = Vec::with_capacity(variants.len());
                for variant in variants {
                    let mut variant_fields = Vec::with_capacity(variant.fields.len());
                    for (_, ty) in &variant.fields {
                        variant_fields.push(parse_type_annotation_with_symbols(ty, &symbols)?);
                    }
                    enum_variants.push(EnumVariant {
                        name: variant.name.clone(),
                        fields: variant_fields,
                    });
                }
                let enum_type = Type::Enum {
                    name: name.clone(),
                    variants: enum_variants.clone(),
                    is_sealed: *is_sealed,
                };
                symbols.insert(name.clone(), enum_type.clone());
                for variant in enum_variants {
                    symbols.insert(
                        format!("{name}::{}", variant.name),
                        Type::Fn {
                            params: variant.fields,
                            ret: Box::new(enum_type.clone()),
                            effects: EffectSet::new(),
                        },
                    );
                }
            }
            _ => {}
        }
    }

    let builtin_names: HashSet<String> = symbols.keys().cloned().collect();

    for s in &prog.stmts {
        if let Stmt::Fn {
            name,
            visibility: _,
            type_params,
            params,
            ret_type,
            effects,
            ..
        } = s
        {
            let generic_names: HashSet<&str> = type_params
                .iter()
                .map(|(generic, _)| generic.as_str())
                .collect();
            let mut ptypes = Vec::new();
            for (_param_name, annotation) in params {
                let param_type = if let Some(annotation) = annotation {
                    if generic_names.contains(annotation.as_str()) {
                        Type::Generic(annotation.clone())
                    } else {
                        parse_type_annotation_with_symbols(annotation, &symbols)?
                    }
                } else if !type_params.is_empty() {
                    Type::Generic(type_params[0].0.clone())
                } else {
                    Type::Generic("_".to_string())
                };
                ptypes.push(param_type);
            }
            let rtype = if let Some(rt) = ret_type {
                if generic_names.contains(rt.as_str()) {
                    Type::Generic(rt.clone())
                } else {
                    parse_type_annotation_with_symbols(rt, &symbols)?
                }
            } else {
                Type::Generic(format!("__inferred_ret_{name}"))
            };
            let mut efmask = EffectSet::new();
            for e in effects {
                match e.as_str() {
                    "io" => efmask.add(Effect::Io),
                    "pure" => efmask.add(Effect::Pure),
                    "async" => efmask.add(Effect::Async),
                    "panic" => efmask.add(Effect::Panic),
                    _ => {}
                }
            }
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
        vis_ctx: &VisibilityCtx,
        builtin_names: &HashSet<String>,
        struct_bounds: &HashMap<String, Vec<(String, Vec<String>)>>,
        expected_return: Option<&Type>,
    ) -> Result<(Option<Type>, EffectSet), Diagnostic> {
        let mut last: Option<Type> = None;
        let mut effects = EffectSet::new();
        for stmt in stmts {
            match stmt {
                Stmt::Let(name, type_ann, expr, _) => {
                    if let Some(ref ann) = type_ann {
                        let ann_type = parse_type_annotation_with_symbols(ann, symbols)?;
                        check_expr(expr, &ann_type, symbols, ctx, vis_ctx).map_err(|e| {
                            Diagnostic::error(
                                error_codes::TYPE_MISMATCH,
                                format!(
                                    "Type annotation mismatch for '{}': expected {:?}, but expression has incompatible type",
                                    name, ann_type
                                ),
                            )
                            .with_note(format!("Original error: {}", e.message))
                        })?;
                        symbols.insert(name.clone(), ann_type);
                    } else {
                        let (typ, ef) = synthesize_expr(expr, symbols, ctx, vis_ctx)?;
                        symbols.insert(name.clone(), typ);
                        effects.union_with(&ef);
                    }
                    last = None;
                }
                Stmt::LetMut(name, type_ann, expr, _) => {
                    if let Some(ref ann) = type_ann {
                        let ann_type = parse_type_annotation_with_symbols(ann, symbols)?;
                        check_expr(expr, &ann_type, symbols, ctx, vis_ctx).map_err(|e| {
                            Diagnostic::error(
                                error_codes::TYPE_MISMATCH,
                                format!(
                                    "Type annotation mismatch for '{}': expected {:?}, but expression has incompatible type",
                                    name, ann_type
                                ),
                            )
                            .with_note(format!("Original error: {}", e.message))
                        })?;
                        symbols.insert(name.clone(), ann_type);
                    } else {
                        let (typ, ef) = synthesize_expr(expr, symbols, ctx, vis_ctx)?;
                        symbols.insert(name.clone(), typ);
                        effects.union_with(&ef);
                    }
                    symbols.insert(mutable_binding_symbol(name), Type::Bool);
                    last = None;
                }
                Stmt::LetLinear(name, type_ann, expr, _) => {
                    if let Some(ref ann) = type_ann {
                        let ann_type = parse_type_annotation_with_symbols(ann, symbols)?;
                        check_expr(expr, &ann_type, symbols, ctx, vis_ctx).map_err(|e| {
                            Diagnostic::error(
                                error_codes::TYPE_MISMATCH,
                                format!(
                                    "Type annotation mismatch for '{}': expected {:?}, but expression has incompatible type",
                                    name, ann_type
                                ),
                            )
                            .with_note(format!("Original error: {}", e.message))
                        })?;
                        symbols.insert(name.clone(), ann_type);
                    } else {
                        let (typ, ef) = synthesize_expr(expr, symbols, ctx, vis_ctx)?;
                        symbols.insert(name.clone(), typ);
                        effects.union_with(&ef);
                    }
                    symbols.insert(format!("__omni_linear_binding::{name}"), Type::Bool);
                    last = None;
                }
                Stmt::Struct {
                    name,
                    fields,
                    is_linear,
                    ..
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
                    ..
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
                Stmt::ErrorSet { name, variants, .. } => {
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
                            is_sealed: false,
                        },
                    );
                    last = None;
                }
                Stmt::Impl {
                    target,
                    visibility: _,
                    type_params,
                    methods,
                    for_type,
                    ..
                } => {
                    let mut impl_symbols = symbols.clone();

                    if !type_params.is_empty() {
                        for (tp_name, tp_bounds) in type_params {
                            impl_symbols.insert(tp_name.clone(), Type::Generic(tp_name.clone()));
                            for bound in tp_bounds {
                                impl_symbols.insert(bound.clone(), Type::Generic(bound.clone()));
                            }
                        }
                    }

                    if let Some(bounds) = struct_bounds.get(target) {
                        for (tp_name, tp_bounds) in bounds {
                            if !impl_symbols.contains_key(tp_name) {
                                impl_symbols
                                    .insert(tp_name.clone(), Type::Generic(tp_name.clone()));
                            }
                            for bound in tp_bounds {
                                if !impl_symbols.contains_key(bound) {
                                    impl_symbols
                                        .insert(bound.clone(), Type::Generic(bound.clone()));
                                }
                            }
                        }
                    }

                    let (ret, ef) = check_stmts(
                        methods,
                        &mut impl_symbols,
                        ctx,
                        vis_ctx,
                        builtin_names,
                        struct_bounds,
                        None,
                    )?;
                    effects.union_with(&ef);
                    last = ret;
                    // Validate trait bounds for generic impl parameters
                    if let Some(ref impl_trait_name) = for_type {
                        for (tp_name, _tp_bounds) in type_params {
                            if let Some(param_type) = impl_symbols.get(tp_name) {
                                let resolved_type = ctx.resolve(param_type);
                                let _bound_ok = check_trait_bound_in_context(
                                    &resolved_type,
                                    impl_trait_name,
                                    &TraitSystem::new(),
                                    Some(tp_name),
                                );
                            }
                        }
                    }
                    symbols.insert(
                        target.clone(),
                        Type::Struct {
                            name: target.clone(),
                            fields: vec![],
                            is_linear: false,
                        },
                    );
                }
                Stmt::Trait { name, .. } => {
                    symbols.insert(
                        name.clone(),
                        Type::Struct {
                            name: name.clone(),
                            fields: vec![],
                            is_linear: false,
                        },
                    );
                    last = None;
                }
                Stmt::TypeAlias { name, target, .. } => {
                    symbols.insert(name.clone(), Type::Generic(target.clone()));
                    last = None;
                }
                Stmt::Use { path, alias, .. } => {
                    let name = alias.clone().unwrap_or_else(|| {
                        path.replace('.', "::")
                            .rsplit("::")
                            .next()
                            .unwrap_or(path.as_str())
                            .to_string()
                    });
                    symbols.insert(name, Type::String);
                    last = None;
                }
                Stmt::GcMode { .. } => {}
                Stmt::CancelToken { .. } => {}
                Stmt::EffectHandler { .. } => {}
                Stmt::Defer { .. } | Stmt::AsyncDefer { .. } => {}
                Stmt::Spawn { .. } => {}
                Stmt::Channel { elem_type, .. } => {
                    symbols.insert(
                        format!("Chan_{}", elem_type),
                        Type::Generic(format!("chan<{}>", elem_type)),
                    );
                    last = None;
                }
                Stmt::Actor { name, state, .. } => {
                    symbols.insert(name.clone(), Type::Generic(state.clone()));
                    last = None;
                }
                Stmt::WorkStealingExecutor { .. } => {}
                Stmt::DeterministicRuntime { .. } => {}
                Stmt::Tensor { shape, dtype, .. } => {
                    let shape_str = shape
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join("x");
                    symbols.insert(
                        format!("tensor_{}", shape_str),
                        Type::Generic(dtype.clone()),
                    );
                    last = None;
                }
                Stmt::Simd {
                    width, elem_type, ..
                } => {
                    symbols.insert(
                        format!("simd{}x{}", width, elem_type),
                        Type::Generic(elem_type.clone()),
                    );
                    last = None;
                }
                Stmt::DocComment { .. } => {
                    last = None;
                }
                Stmt::DebugSession { .. } => {
                    last = None;
                }
                Stmt::Capability { name, .. } => {
                    symbols.insert(name.clone(), Type::Generic("Capability".to_string()));
                    last = None;
                }
                Stmt::FfiSandbox { .. } => {
                    last = None;
                }
                Stmt::UseScoped {
                    path: _,
                    aliases,
                    body,
                    ..
                } => {
                    let mut local = symbols.clone();
                    for (alias, resolved_name) in aliases.iter() {
                        let name_to_insert = resolved_name.clone().unwrap_or_else(|| alias.clone());
                        if let Some(ty) = symbols.get(&name_to_insert) {
                            local.insert(alias.clone(), ty.clone());
                        } else {
                            local.insert(alias.clone(), Type::Generic(name_to_insert.clone()));
                        }
                    }
                    let (ret, ef) = check_stmts(
                        body,
                        &mut local,
                        ctx,
                        vis_ctx,
                        builtin_names,
                        struct_bounds,
                        expected_return,
                    )?;
                    effects.union_with(&ef);
                    last = ret;
                }
                Stmt::ContractRequires { condition, .. } => {
                    let (_, ef) = synthesize_expr(condition, symbols, ctx, vis_ctx)?;
                    effects.union_with(&ef);
                    last = None;
                }
                Stmt::ContractEnsures { condition, .. } => {
                    let (_, ef) = synthesize_expr(condition, symbols, ctx, vis_ctx)?;
                    effects.union_with(&ef);
                    last = None;
                }
                Stmt::ContractInvariant { condition, .. } => {
                    let (_, ef) = synthesize_expr(condition, symbols, ctx, vis_ctx)?;
                    effects.union_with(&ef);
                    last = None;
                }
                Stmt::ComptimeLimit { .. } => {
                    last = None;
                }
                Stmt::Annotation(_, _) | Stmt::Mod(_, _) | Stmt::ModBlock(_, _, _) => {
                    last = None;
                }
                Stmt::Print(expr, _) => {
                    let (_, ef) = synthesize_expr(expr, symbols, ctx, vis_ctx)?;
                    effects.union_with(&ef);
                    effects.add(Effect::Io);
                    last = None;
                }
                Stmt::ExprStmt(expr, _) => {
                    let (_, ef) = synthesize_expr(expr, symbols, ctx, vis_ctx)?;
                    effects.union_with(&ef);
                    last = None;
                }
                Stmt::Block(stmts, _) => {
                    let (ret, ef) = check_stmts(
                        stmts,
                        symbols,
                        ctx,
                        vis_ctx,
                        builtin_names,
                        struct_bounds,
                        expected_return,
                    )?;
                    effects.union_with(&ef);
                    last = ret;
                }
                Stmt::Fn {
                    name,
                    visibility,
                    params,
                    type_params,
                    body,
                    ret_type,
                    effects: declared_effects,
                    ..
                } => {
                    let mut local_symbols = symbols.clone();
                    let mut fctx = InferCtx::new();

                    let mut gen_inst: HashMap<String, Type> = HashMap::new();
                    for tp in type_params.iter() {
                        gen_inst.insert(tp.0.clone(), fctx.fresh_var());
                    }

                    let mut ptypes: Vec<Type> = Vec::new();
                    for (i, param) in params.iter().enumerate() {
                        let param_type = if let Some(annotation) = &param.1 {
                            if let Some(generic_type) = gen_inst.get(annotation) {
                                generic_type.clone()
                            } else {
                                parse_type_annotation_with_symbols(annotation, symbols)?
                            }
                        } else if !type_params.is_empty() {
                            let gname = if i < type_params.len() {
                                type_params[i].0.clone()
                            } else {
                                type_params[0].0.clone()
                            };
                            gen_inst
                                .get(&gname)
                                .cloned()
                                .unwrap_or_else(|| fctx.fresh_var())
                        } else {
                            fctx.fresh_var()
                        };
                        ptypes.push(param_type);
                    }

                    for (i, p) in params.iter().enumerate() {
                        local_symbols.insert(p.0.clone(), ptypes[i].clone());
                    }

                    let declared_ret = if let Some(rt) = ret_type {
                        if let Some(generic_type) = gen_inst.get(rt) {
                            generic_type.clone()
                        } else {
                            parse_type_annotation_with_symbols(rt, symbols)?
                        }
                    } else {
                        fctx.fresh_var()
                    };

                    if !builtin_names.contains(name) {
                        let (ret_opt, efmask) = check_stmts(
                            body,
                            &mut local_symbols,
                            &mut fctx,
                            vis_ctx,
                            builtin_names,
                            struct_bounds,
                            Some(&declared_ret),
                        )?;

                        // Return expressions are unified against one function-local
                        // return variable when no explicit annotation is present. This
                        // preserves the historical Stage-0 return-type inference while
                        // still rejecting conflicting return types.
                        let _ = ret_opt;
                        let resolved_params: Vec<Type> =
                            ptypes.iter().map(|ty| fctx.resolve(ty)).collect();
                        let resolved_declared_ret = fctx.resolve(&declared_ret);
                        let resolved_ret = if ret_type.is_none()
                            && matches!(resolved_declared_ret, Type::Var(_))
                        {
                            Type::Unit
                        } else {
                            resolved_declared_ret
                        };

                        let mut declared_mask = EffectSet::new();
                        for e in declared_effects {
                            match e.as_str() {
                                "io" => declared_mask.add(Effect::Io),
                                "pure" => declared_mask.add(Effect::Pure),
                                "async" => declared_mask.add(Effect::Async),
                                "panic" => declared_mask.add(Effect::Panic),
                                _ => {}
                            }
                        }

                        if !matches!(visibility, crate::ast::Visibility::Private)
                            && declared_mask.is_empty()
                        {
                            return Err(Diagnostic::error(
                                error_codes::TYPE_EFFECT_REQUIRED,
                                format!(
                                    "Public function '{}' must declare an explicit effect annotation",
                                    name
                                ),
                            ));
                        }

                        let final_effects = if declared_mask.is_empty() {
                            efmask
                        } else {
                            if !efmask.difference(&declared_mask).is_empty() {
                                return Err(Diagnostic::error(
                                    error_codes::TYPE_EFFECT_MISMATCH,
                                    format!("Function '{}' performs effects not included in declared effects", name),
                                ));
                            }
                            declared_mask
                        };
                        symbols.insert(
                            name.clone(),
                            Type::Fn {
                                params: resolved_params,
                                ret: Box::new(resolved_ret),
                                effects: final_effects,
                            },
                        );
                    }
                    last = None;
                }
                Stmt::If {
                    cond,
                    then_body,
                    else_body,
                    ..
                } => {
                    let (_, ef) = synthesize_expr(cond, symbols, ctx, vis_ctx)?;
                    effects.union_with(&ef);
                    let (_, ef1) = check_stmts(
                        then_body,
                        symbols,
                        ctx,
                        vis_ctx,
                        builtin_names,
                        struct_bounds,
                        expected_return,
                    )?;
                    effects |= ef1;
                    let (_, ef2) = check_stmts(
                        else_body,
                        symbols,
                        ctx,
                        vis_ctx,
                        builtin_names,
                        struct_bounds,
                        expected_return,
                    )?;
                    effects |= ef2;
                    last = None;
                }
                Stmt::Loop { body, .. } => {
                    let (_, ef) = check_stmts(
                        body,
                        symbols,
                        ctx,
                        vis_ctx,
                        builtin_names,
                        struct_bounds,
                        expected_return,
                    )?;
                    effects.union_with(&ef);
                    last = None;
                }
                Stmt::For { iterable, body, .. } => {
                    let (_, ef) = synthesize_expr(iterable, symbols, ctx, vis_ctx)?;
                    effects.union_with(&ef);
                    let (_, ef2) = check_stmts(
                        body,
                        symbols,
                        ctx,
                        vis_ctx,
                        builtin_names,
                        struct_bounds,
                        expected_return,
                    )?;
                    effects |= ef2;
                    last = None;
                }
                Stmt::While { cond, body, .. } => {
                    let (_, ef) = synthesize_expr(cond, symbols, ctx, vis_ctx)?;
                    effects.union_with(&ef);
                    let (_, ef2) = check_stmts(
                        body,
                        symbols,
                        ctx,
                        vis_ctx,
                        builtin_names,
                        struct_bounds,
                        expected_return,
                    )?;
                    effects |= ef2;
                    last = None;
                }
                Stmt::Return(expr, _) => {
                    let (typ, ef) = synthesize_expr(expr, symbols, ctx, vis_ctx)?;
                    effects.union_with(&ef);
                    if let Some(expected) = expected_return {
                        ctx.unify(&typ, expected).map_err(|e| {
                            Diagnostic::error(
                                error_codes::TYPE_MISMATCH,
                                format!("return type mismatch: {}", e),
                            )
                        })?;
                    }
                    last = Some(typ);
                }
                Stmt::Break(_) | Stmt::Continue(_) => {
                    last = None;
                }
                Stmt::Assign(name, expr, _) => {
                    let existing = symbols.get(name).cloned().ok_or_else(|| {
                        Diagnostic::error(
                            error_codes::TYPE_INFERENCE_FAILED,
                            format!("Assignment target '{}' is not defined", name),
                        )
                    })?;
                    let (typ, ef) = synthesize_expr(expr, symbols, ctx, vis_ctx)?;
                    effects.union_with(&ef);
                    ctx.unify(&typ, &existing).map_err(|_| {
                        Diagnostic::error(
                            error_codes::TYPE_MISMATCH,
                            format!(
                                "Assignment to '{}' changes its type from {:?} to {:?}",
                                name, existing, typ
                            ),
                        )
                    })?;
                    last = None;
                }
                Stmt::DerefAssign(reference, expr, _) => {
                    let (reference_ty, ef1) = synthesize_expr(reference, symbols, ctx, vis_ctx)?;
                    effects |= ef1;
                    let Type::Ref {
                        mutable: true,
                        inner,
                    } = ctx.resolve(&reference_ty)
                    else {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_MISMATCH,
                            "dereference assignment requires a mutable reference",
                        ));
                    };
                    let (value_ty, ef2) = synthesize_expr(expr, symbols, ctx, vis_ctx)?;
                    effects |= ef2;
                    ctx.unify(&value_ty, &inner).map_err(|_| {
                        Diagnostic::error(
                            error_codes::TYPE_MISMATCH,
                            format!(
                                "dereference assignment type mismatch: expected {:?}, found {:?}",
                                inner, value_ty
                            ),
                        )
                    })?;
                    last = None;
                }
                Stmt::ExprFieldAssign(base, field, expr, _) => {
                    let Expr::Var(base_name, _) = base.as_ref() else {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_INVALID_FIELD_ACCESS,
                            "nested/general field mutation is not yet qualified; v0.2.0.0 currently permits only direct linear-field reinitialization",
                        ));
                    };
                    if !symbols.contains_key(&format!("__omni_linear_binding::{base_name}")) {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_MISMATCH,
                            format!("field mutation of '{}' is not yet qualified; only reinitialization of a moved field on a linear binding is allowed", base_name),
                        ));
                    }
                    let (base_ty, ef1) = synthesize_expr(base, symbols, ctx, vis_ctx)?;
                    effects |= ef1;
                    let resolved_base = ctx.resolve(&base_ty);
                    let Type::Struct { name, .. } = resolved_base else {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_INVALID_FIELD_ACCESS,
                            format!(
                                "Field assignment '.{}' requires a struct base, got {:?}",
                                field, base_ty
                            ),
                        ));
                    };
                    let field_ty = symbols
                        .get(&struct_field_symbol(&name, field))
                        .cloned()
                        .ok_or_else(|| {
                            Diagnostic::error(
                                error_codes::TYPE_INVALID_FIELD_ACCESS,
                                format!("Struct '{}' has no field '{}'", name, field),
                            )
                        })?;
                    let (value_ty, ef2) = synthesize_expr(expr, symbols, ctx, vis_ctx)?;
                    effects |= ef2;
                    ctx.unify(&value_ty, &field_ty).map_err(|_| {
                        Diagnostic::error(
                            error_codes::TYPE_MISMATCH,
                            format!(
                                "Assignment to '{}.{}' expects {:?}, got {:?}",
                                name, field, field_ty, value_ty
                            ),
                        )
                    })?;
                    last = None;
                }
                Stmt::WhileIn { iterable, body, .. } => {
                    let (_, ef) = synthesize_expr(iterable, symbols, ctx, vis_ctx)?;
                    effects.union_with(&ef);
                    let (_, ef2) = check_stmts(
                        body,
                        symbols,
                        ctx,
                        vis_ctx,
                        builtin_names,
                        struct_bounds,
                        expected_return,
                    )?;
                    effects |= ef2;
                    last = None;
                }
                Stmt::Unsafe { body, .. } => {
                    let (_, ef) = check_stmts(
                        body,
                        symbols,
                        ctx,
                        vis_ctx,
                        builtin_names,
                        struct_bounds,
                        expected_return,
                    )?;
                    effects.union_with(&ef);
                    last = None;
                }
            }
        }
        Ok((last, effects))
    }

    let mut global_ctx = InferCtx::new();
    let vis_ctx = VisibilityCtx::new();
    check_stmts(
        &prog.stmts,
        &mut symbols,
        &mut global_ctx,
        &vis_ctx,
        &builtin_names,
        &struct_bounds,
        None,
    )?;
    Ok(symbols)
}

/// Type-check a program with optional module system support.
/// This is the entry point used by the driver when multi-file compilation is enabled.
pub fn type_check_program_with_modules(
    prog: &Program,
    module_system: Option<&crate::module_system::ModuleSystem>,
) -> Result<HashMap<String, Type>, Diagnostic> {
    let Some(module_system) = module_system else {
        return type_check_program(prog);
    };

    // Cross-module visibility is checked by the driver before this function is
    // called. Here we import only the signatures explicitly named by `use`
    // declarations so expression typing sees the same bindings as name
    // resolution. Bodies stay in their owning modules and are not re-checked in
    // the importing module.
    let mut imported_stubs = Vec::new();
    for stmt in &prog.stmts {
        let Stmt::Use { path, alias, .. } = stmt else {
            continue;
        };
        let normalized = path.replace('.', "::");
        let Some((module_path, symbol_name)) = normalized.rsplit_once("::") else {
            continue;
        };
        let Some(module) = module_system.modules.get(module_path) else {
            continue;
        };
        let Some(module_program) = &module.program else {
            continue;
        };
        let local_name = alias.as_deref().unwrap_or(symbol_name);
        for declaration in &module_program.stmts {
            if let Stmt::Fn {
                name,
                is_async,
                type_params,
                params,
                ret_type,
                effects,
                span,
                ..
            } = declaration
            {
                if name == symbol_name {
                    imported_stubs.push(Stmt::Fn {
                        name: local_name.to_string(),
                        visibility: crate::ast::Visibility::Private,
                        is_async: *is_async,
                        type_params: type_params.clone(),
                        params: params.clone(),
                        ret_type: ret_type.clone(),
                        effects: effects.clone(),
                        contracts: Vec::new(),
                        body: Vec::new(),
                        span: span.clone(),
                    });
                    break;
                }
            }
        }
    }

    if imported_stubs.is_empty() {
        return type_check_program(prog);
    }

    let mut augmented = Program {
        stmts: imported_stubs,
    };
    augmented.stmts.extend(
        prog.stmts
            .iter()
            .filter(|stmt| !matches!(stmt, Stmt::Use { .. }))
            .cloned(),
    );
    type_check_program(&augmented)
}

/// Check if a type satisfies a trait bound, using the trait system context.
/// For Type::Var (unresolved type variables from impl-level generics),
/// this checks resolved_bounds first: if the variable has declared bounds
/// that include the trait, return true. If it has declared bounds that
/// exclude the trait, return false. If no bounds are found, defer to true
/// (known false-positive trade-off for generic impl methods)./// Check if a type satisfies a trait bound, using the trait system context.
///
/// For `Type::Var` (unresolved type variables from impl-level generics),
/// this checks `resolved_bounds` using the optional type parameter name:
/// - If `type_param_name` is provided and bounds exist for that name, check
///   whether any bound includes the requested trait.
/// - If bounds exist but exclude the trait, return false (true negative).
/// - If no bounds are found, defer to true (known false-positive trade-off
///   for generic impl methods where the variable may satisfy the bound at
///   instantiation time).
///
/// For concrete types (`Struct`, `Enum`, `Option`, `Result`), this checks
/// the trait system's impl list directly.
///
/// For `Generic` types, checks `resolved_bounds` by name, then checks impls.
#[allow(dead_code)]
fn check_trait_bound_in_context(
    ty: &Type,
    trait_name: &str,
    trait_system: &TraitSystem,
    type_param_name: Option<&str>,
) -> bool {
    match ty {
        Type::Var(_) => {
            // Unresolved type variables from impl-level generic type parameters
            // cannot be fully checked yet. If we have a type parameter name,
            // look up its declared bounds in resolved_bounds.
            if let Some(param_name) = type_param_name {
                if let Some(bounds) = trait_system.resolved_bounds.get(param_name) {
                    // Variable has declared bounds: check if any include this trait.
                    return bounds.iter().any(|b| b.trait_name == trait_name);
                }
            }
            // No bounds found or no parameter name: defer to true.
            // This is a known false-positive trade-off.
            true
        }
        Type::Struct { .. } | Type::Enum { .. } => {
            // Concrete named types: check the trait system impl list directly.
            let concrete = ty.clone();
            trait_system.check_trait_bound(&concrete, trait_name)
        }
        Type::Option(_) | Type::Result(_, _) => {
            // Built-in enum types: check the trait system.
            trait_system.check_trait_bound(ty, trait_name)
        }
        Type::Generic(name) => {
            // Named generics: check resolved_bounds by name, then check impls.
            if let Some(bounds) = trait_system.resolved_bounds.get(name) {
                if bounds.iter().any(|b| b.trait_name == trait_name) {
                    return true;
                }
            }
            // Fallback: check the trait system impl list.
            trait_system.check_trait_bound(ty, trait_name)
        }
        _ => {
            // Other types (Int, Float, String, Bool, etc.): check directly.
            trait_system.check_trait_bound(ty, trait_name)
        }
    }
}
/// Check if a pattern is a wildcard or variable binding (catch-all).
/// Does NOT recurse into Or patterns for exhaustiveness purposes:
/// each Or branch must be checked independently.
fn has_wildcard_or_var(pat: &Pattern) -> bool {
    matches!(pat, Pattern::Wildcard | Pattern::Var(_))
}

/// Expand an Or pattern into its constituent patterns.
fn expand_or_patterns(patterns: &[Pattern]) -> Vec<&Pattern> {
    let mut result = Vec::new();
    for p in patterns {
        match p {
            Pattern::Or(subs) => {
                for sub in subs {
                    result.push(sub);
                }
            }
            _ => result.push(p),
        }
    }
    result
}

/// Check if a set of patterns is exhaustive for the given scrutinee type.
/// Returns Ok(()) if exhaustive, or Err(missing_description) if not.
fn is_exhaustive(patterns: &[&Pattern], scrutinee_type: &Type) -> Result<(), String> {
    // A wildcard or unbound variable covers everything.
    if patterns.iter().any(|p| has_wildcard_or_var(p)) {
        return Ok(());
    }

    match scrutinee_type {
        Type::Bool => {
            let has_true = patterns.iter().any(|p| matches!(p, Pattern::Literal(1)));
            let has_false = patterns.iter().any(|p| matches!(p, Pattern::Literal(0)));
            if has_true && has_false {
                Ok(())
            } else {
                let mut missing = Vec::new();
                if !has_true {
                    missing.push("true".to_string());
                }
                if !has_false {
                    missing.push("false".to_string());
                }
                Err(format!("missing patterns: {}", missing.join(", ")))
            }
        }
        Type::Int | Type::Float | Type::String | Type::Bytes | Type::Char | Type::Byte => {
            // Infinite types; only exhaustive with a wildcard.
            Err(format!(
                "missing wildcard pattern for {:?} type",
                scrutinee_type
            ))
        }
        Type::Enum { name, variants, .. } => {
            let mut missing = Vec::new();
            for variant in variants {
                let covered = patterns.iter().any(|p| match p {
                    Pattern::Struct(pattern_name, _) => {
                        enum_pattern_variant_name(name, pattern_name)
                            .is_some_and(|candidate| candidate == variant.name)
                    }
                    _ => false,
                });
                if !covered {
                    missing.push(variant.name.clone());
                }
            }
            if missing.is_empty() {
                Ok(())
            } else {
                Err(format!("missing variants: {}", missing.join(", ")))
            }
        }
        Type::Option(_) => {
            let has_some = patterns
                .iter()
                .any(|p| matches!(p, Pattern::Struct(name, _) if name == "Some"));
            let has_none = patterns
                .iter()
                .any(|p| matches!(p, Pattern::Struct(name, _) if name == "None"));
            if has_some && has_none {
                Ok(())
            } else {
                let mut missing = Vec::new();
                if !has_some {
                    missing.push("Some(_)".to_string());
                }
                if !has_none {
                    missing.push("None".to_string());
                }
                Err(format!("missing patterns: {}", missing.join(", ")))
            }
        }
        Type::Result(_, _) => {
            let has_ok = patterns
                .iter()
                .any(|p| matches!(p, Pattern::Struct(name, _) if name == "Ok"));
            let has_err = patterns
                .iter()
                .any(|p| matches!(p, Pattern::Struct(name, _) if name == "Err"));
            if has_ok && has_err {
                Ok(())
            } else {
                let mut missing = Vec::new();
                if !has_ok {
                    missing.push("Ok(_)".to_string());
                }
                if !has_err {
                    missing.push("Err(_)".to_string());
                }
                Err(format!("missing patterns: {}", missing.join(", ")))
            }
        }
        _ => {
            // Other types (Unit, Never, Generic, etc.) require a wildcard.
            Err("missing wildcard pattern".to_string())
        }
    }
}

/// Check match exhaustiveness for a match expression during type checking.
fn check_match_exhaustiveness(
    scrutinee_type: &Type,
    arms: &[MatchArm],
    ctx: &InferCtx,
) -> Result<(), Diagnostic> {
    let resolved = ctx.resolve(scrutinee_type);
    // Expand Or patterns in each arm
    let mut all_patterns: Vec<&Pattern> = Vec::new();
    for arm in arms {
        let expanded = expand_or_patterns(std::slice::from_ref(&arm.pattern));
        all_patterns.extend(expanded);
    }
    if let Err(missing) = is_exhaustive(&all_patterns, &resolved) {
        Err(Diagnostic::error(
            error_codes::TYPE_MISMATCH,
            format!(
                "Non-exhaustive match expression: {} for type {:?}",
                missing, resolved
            ),
        ))
    } else {
        Ok(())
    }
}

fn parse_type_annotation(type_str: &str) -> Result<Type, Diagnostic> {
    let trimmed = type_str.trim();
    if let Some(inner) = trimmed.strip_prefix("&mut ") {
        return Ok(Type::Ref {
            mutable: true,
            inner: Box::new(parse_type_annotation(inner.trim())?),
        });
    }
    if let Some(inner) = trimmed.strip_prefix('&') {
        return Ok(Type::Ref {
            mutable: false,
            inner: Box::new(parse_type_annotation(inner.trim())?),
        });
    }
    match trimmed.to_lowercase().as_str() {
        "int" | "i64" | "isize" => Ok(Type::Int),
        "float" | "f64" | "float64" => Ok(Type::Float),
        "char" => Ok(Type::Char),
        "byte" | "u8" => Ok(Type::Byte),
        "string" | "str" => Ok(Type::String),
        "bytes" => Ok(Type::Bytes),
        "bool" | "boolean" => Ok(Type::Bool),
        "unit" | "void" => Ok(Type::Unit),
        "never" => Ok(Type::Never),
        _ => {
            if type_str.starts_with("option<") && type_str.ends_with('>') {
                let inner = &type_str[7..type_str.len() - 1];
                let inner_type = parse_type_annotation(inner)?;
                Ok(Type::Option(Box::new(inner_type)))
            } else if type_str.starts_with("result<") && type_str.ends_with('>') {
                let inner = &type_str[7..type_str.len() - 1];
                let parts: Vec<&str> = inner.splitn(2, ',').collect();
                if parts.len() == 2 {
                    let ok_type = parse_type_annotation(parts[0].trim())?;
                    let err_type = parse_type_annotation(parts[1].trim())?;
                    Ok(Type::Result(Box::new(ok_type), Box::new(err_type)))
                } else {
                    Err(Diagnostic::error(
                        error_codes::TYPE_UNDEFINED_TYPE,
                        "Result type requires two parameters: 'Result<T, E>'".to_string(),
                    ))
                }
            } else if type_str.starts_with("errors<") && type_str.ends_with('>') {
                let name = &type_str[7..type_str.len() - 1];
                Ok(Type::ErrorSet(name.to_string()))
            } else {
                Err(Diagnostic::error(
                    error_codes::TYPE_UNDEFINED_TYPE,
                    format!("Unknown type '{}'", type_str),
                ))
            }
        }
    }
}

fn substitute_type(ty: &Type, gen_map: &HashMap<String, Type>) -> Type {
    match ty {
        Type::Var(id) => Type::Var(*id),
        Type::Generic(name) => gen_map
            .get(name)
            .cloned()
            .unwrap_or(Type::Generic(name.clone())),
        Type::Ref { mutable, inner } => Type::Ref {
            mutable: *mutable,
            inner: Box::new(substitute_type(inner, gen_map)),
        },
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
                effects: effects.clone(),
            }
        }
        Type::Int => Type::Int,
        Type::Float => Type::Float,
        Type::Char => Type::Char,
        Type::Byte => Type::Byte,
        Type::String => Type::String,
        Type::Bytes => Type::Bytes,
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
        Type::Option(inner) => Type::Option(Box::new(substitute_type(inner, gen_map))),
        Type::Result(ok, err) => Type::Result(
            Box::new(substitute_type(ok, gen_map)),
            Box::new(substitute_type(err, gen_map)),
        ),
        Type::ErrorSet(name) => Type::ErrorSet(name.clone()),
    }
}

/// Check linear types: ensure linear values are used exactly once.
fn check_linear_types(prog: &Program) -> Result<(), Diagnostic> {
    use std::collections::HashMap;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TrackedLinearState {
        Available,
        PartiallyMoved,
        Moved,
        MaybeMoved,
    }

    struct LinearTracker {
        states: HashMap<String, TrackedLinearState>,
        aggregate_fields: HashMap<String, Vec<String>>,
    }

    impl LinearTracker {
        fn new() -> Self {
            LinearTracker {
                states: HashMap::new(),
                aggregate_fields: HashMap::new(),
            }
        }

        fn join_state(
            left: Option<TrackedLinearState>,
            right: TrackedLinearState,
        ) -> TrackedLinearState {
            match left {
                None => right,
                Some(existing) if existing == right => existing,
                Some(_) => TrackedLinearState::MaybeMoved,
            }
        }

        fn merge_branch_states(&mut self, branches: &[HashMap<String, TrackedLinearState>]) {
            let baseline = self.states.clone();
            let mut keys: std::collections::HashSet<String> = baseline.keys().cloned().collect();
            for branch in branches {
                for name in branch.keys() {
                    let root = name.split('.').next().unwrap_or(name);
                    if baseline.contains_key(name) || baseline.contains_key(root) {
                        keys.insert(name.clone());
                    }
                }
            }

            let mut merged = HashMap::new();
            for name in keys {
                let root = name.split('.').next().unwrap_or(name.as_str());
                let is_projection = name.contains('.');
                let mut joined = None;
                for branch in branches {
                    let state = branch.get(&name).copied().or_else(|| {
                        if is_projection && baseline.contains_key(root) {
                            Some(TrackedLinearState::Available)
                        } else {
                            baseline.get(&name).copied()
                        }
                    });
                    if let Some(state) = state {
                        joined = Some(Self::join_state(joined, state));
                    }
                }
                if let Some(state) = joined {
                    merged.insert(name, state);
                }
            }
            self.states = merged;
        }

        fn expr_place_path(expr: &Expr) -> Option<String> {
            match expr {
                Expr::Var(name, _) => Some(name.clone()),
                Expr::FieldAccess { base, field, .. } => {
                    Some(format!("{}.{}", Self::expr_place_path(base)?, field))
                }
                _ => None,
            }
        }

        fn linear_root_name(expr: &Expr) -> Option<&str> {
            match expr {
                Expr::Var(name, _) => Some(name.as_str()),
                Expr::FieldAccess { base, .. } => Self::linear_root_name(base),
                _ => None,
            }
        }

        fn consume_linear_place(&mut self, place: &str) -> Result<(), Diagnostic> {
            let root = place.split('.').next().unwrap_or(place);
            let Some(root_state) = self.states.get(root).copied() else {
                return Ok(());
            };
            if place == root {
                match root_state {
                    TrackedLinearState::Available => {
                        self.states
                            .insert(root.to_string(), TrackedLinearState::Moved);
                        let prefix = format!("{}.", root);
                        let projections: Vec<String> = self
                            .states
                            .keys()
                            .filter(|name| name.starts_with(&prefix))
                            .cloned()
                            .collect();
                        for projection in projections {
                            self.states.insert(projection, TrackedLinearState::Moved);
                        }
                        return Ok(());
                    }
                    TrackedLinearState::PartiallyMoved => {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_MISMATCH,
                            format!("use of partially moved value '{}'", root),
                        ));
                    }
                    TrackedLinearState::Moved => {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_MISMATCH,
                            format!("use of moved value '{}'", root),
                        ));
                    }
                    TrackedLinearState::MaybeMoved => {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_MISMATCH,
                            format!("linear value '{}' is only conditionally available", root),
                        ));
                    }
                }
            }

            match root_state {
                TrackedLinearState::Moved => {
                    return Err(Diagnostic::error(
                        error_codes::TYPE_MISMATCH,
                        format!("use of moved value '{}'", root),
                    ));
                }
                TrackedLinearState::MaybeMoved => {
                    return Err(Diagnostic::error(
                        error_codes::TYPE_MISMATCH,
                        format!("linear value '{}' is only conditionally available", root),
                    ));
                }
                TrackedLinearState::Available | TrackedLinearState::PartiallyMoved => {}
            }

            match self.states.get(place).copied() {
                Some(TrackedLinearState::Moved | TrackedLinearState::PartiallyMoved) => {
                    return Err(Diagnostic::error(
                        error_codes::TYPE_MISMATCH,
                        format!("use of moved field '{}'", place),
                    ));
                }
                Some(TrackedLinearState::MaybeMoved) => {
                    return Err(Diagnostic::error(
                        error_codes::TYPE_MISMATCH,
                        format!("linear field '{}' is only conditionally available", place),
                    ));
                }
                _ => {}
            }
            self.states
                .insert(place.to_string(), TrackedLinearState::Moved);

            let all_fields_moved = self.aggregate_fields.get(root).is_some_and(|fields| {
                !fields.is_empty()
                    && fields.iter().all(|field| {
                        self.states.get(&format!("{}.{}", root, field))
                            == Some(&TrackedLinearState::Moved)
                    })
            });
            self.states.insert(
                root.to_string(),
                if all_fields_moved {
                    TrackedLinearState::Moved
                } else {
                    TrackedLinearState::PartiallyMoved
                },
            );
            Ok(())
        }

        fn reinitialize_linear_place(&mut self, place: &str) -> Result<(), Diagnostic> {
            let root = place.split('.').next().unwrap_or(place);
            let Some(root_state) = self.states.get(root).copied() else {
                return Ok(());
            };
            if place == root {
                return Err(Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!(
                        "whole-value linear reassignment for '{}' is not yet qualified",
                        root
                    ),
                ));
            }
            if matches!(root_state, TrackedLinearState::MaybeMoved) {
                return Err(Diagnostic::error(
                    error_codes::TYPE_MISMATCH,
                    format!(
                        "linear value '{}' is only conditionally available for reinitialization",
                        root
                    ),
                ));
            }
            let field = place
                .strip_prefix(root)
                .and_then(|suffix| suffix.strip_prefix('.'))
                .unwrap_or_default();
            if let Some(fields) = self.aggregate_fields.get(root) {
                if !fields.iter().any(|candidate| candidate == field) {
                    return Err(Diagnostic::error(
                        error_codes::TYPE_INVALID_FIELD_ACCESS,
                        format!("linear aggregate '{}' has no field '{}'", root, field),
                    ));
                }
            }
            match self.states.get(place).copied() {
                Some(TrackedLinearState::Moved) => {}
                Some(TrackedLinearState::MaybeMoved) => {
                    return Err(Diagnostic::error(
                        error_codes::TYPE_MISMATCH,
                        format!("linear field '{}' is only conditionally moved and cannot be reinitialized", place),
                    ));
                }
                Some(TrackedLinearState::Available | TrackedLinearState::PartiallyMoved) | None => {
                    return Err(Diagnostic::error(
                        error_codes::TYPE_MISMATCH,
                        format!("linear field '{}' is still initialized; live-field mutation is not yet qualified", place),
                    ));
                }
            }
            self.states
                .insert(place.to_string(), TrackedLinearState::Available);
            let any_missing = self.aggregate_fields.get(root).is_some_and(|fields| {
                fields.iter().any(|field| {
                    matches!(
                        self.states.get(&format!("{}.{}", root, field)),
                        Some(TrackedLinearState::Moved | TrackedLinearState::MaybeMoved)
                    )
                })
            });
            self.states.insert(
                root.to_string(),
                if any_missing {
                    TrackedLinearState::PartiallyMoved
                } else {
                    TrackedLinearState::Available
                },
            );
            Ok(())
        }

        fn check_scoped_stmts(&mut self, body: &[Stmt]) -> Result<(), Diagnostic> {
            let baseline: std::collections::HashSet<String> = self.states.keys().cloned().collect();
            self.check_stmts(body)?;
            let locals: Vec<String> = self
                .states
                .keys()
                .filter(|name| !baseline.contains(*name) && !name.contains('.'))
                .cloned()
                .collect();
            for name in &locals {
                match self.states.get(name).copied() {
                    Some(TrackedLinearState::Moved) => {}
                    Some(TrackedLinearState::Available) => {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_MISMATCH,
                            format!("linear variable '{}' leaves its lexical scope without being consumed", name),
                        ));
                    }
                    Some(TrackedLinearState::PartiallyMoved) => {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_MISMATCH,
                            format!("linear aggregate '{}' leaves its lexical scope only partially consumed", name),
                        ));
                    }
                    Some(TrackedLinearState::MaybeMoved) => {
                        return Err(Diagnostic::error(
                            error_codes::TYPE_MISMATCH,
                            format!("linear variable '{}' is not consumed on every path before leaving its lexical scope", name),
                        ));
                    }
                    None => {}
                }
            }
            for name in locals {
                self.states.remove(&name);
                self.aggregate_fields.remove(&name);
                let prefix = format!("{}.", name);
                self.states.retain(|place, _| !place.starts_with(&prefix));
            }
            Ok(())
        }

        fn check_loop_body(&mut self, body: &[Stmt]) -> Result<(), Diagnostic> {
            let pre_loop = self.states.clone();
            let mut body_tracker = LinearTracker {
                states: pre_loop.clone(),
                aggregate_fields: self.aggregate_fields.clone(),
            };
            body_tracker.check_stmts(body)?;

            for (name, state) in &body_tracker.states {
                if !pre_loop.contains_key(name) {
                    match state {
                        TrackedLinearState::Available => {
                            return Err(Diagnostic::error(
                                error_codes::TYPE_MISMATCH,
                                format!(
                                    "linear variable '{}' declared in loop body is not consumed before the next iteration boundary",
                                    name
                                ),
                            ));
                        }
                        TrackedLinearState::PartiallyMoved => {
                            return Err(Diagnostic::error(
                                error_codes::TYPE_MISMATCH,
                                format!(
                                    "linear aggregate '{}' declared in loop body is only partially consumed before the next iteration boundary",
                                    name
                                ),
                            ));
                        }
                        TrackedLinearState::MaybeMoved => {
                            return Err(Diagnostic::error(
                                error_codes::TYPE_MISMATCH,
                                format!(
                                    "linear variable '{}' declared in loop body is not consumed on every loop path",
                                    name
                                ),
                            ));
                        }
                        TrackedLinearState::Moved => {}
                    }
                }
            }

            let mut merged = HashMap::new();
            for (name, state) in &pre_loop {
                let after_one_iteration = body_tracker.states.get(name).copied().unwrap_or(*state);
                let joined = Self::join_state(Some(*state), after_one_iteration);
                merged.insert(name.clone(), joined);
            }
            self.states = merged;
            Ok(())
        }

        fn check_stmts(&mut self, stmts: &[Stmt]) -> Result<(), Diagnostic> {
            for stmt in stmts {
                self.check_stmt(stmt)?;
            }
            Ok(())
        }

        fn check_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
            match stmt {
                Stmt::LetLinear(name, _type_ann, expr, _) => {
                    self.check_expr(expr)?;
                    self.states
                        .insert(name.clone(), TrackedLinearState::Available);
                    if let Expr::StructLit { fields, .. } = expr {
                        self.aggregate_fields.insert(
                            name.clone(),
                            fields.iter().map(|(field, _)| field.clone()).collect(),
                        );
                    }
                }
                Stmt::Let(_name, _type_ann, _expr, _)
                | Stmt::LetMut(_name, _type_ann, _expr, _) => {
                    self.check_expr(_expr)?;
                }
                Stmt::Assign(_name, _expr, _) => {
                    self.check_expr(_expr)?;
                }
                Stmt::DerefAssign(reference, expr, _) => {
                    self.check_expr(reference)?;
                    self.check_expr(expr)?;
                }
                Stmt::ExprFieldAssign(base, field, expr, _) => {
                    self.check_expr(expr)?;
                    if let Some(mut place) = Self::expr_place_path(base) {
                        place.push('.');
                        place.push_str(field);
                        self.reinitialize_linear_place(&place)?;
                    } else {
                        self.check_expr(base)?;
                    }
                }
                Stmt::Print(expr, _) | Stmt::ExprStmt(expr, _) => {
                    self.check_expr(expr)?;
                }
                Stmt::Block(inner, _) => {
                    self.check_scoped_stmts(inner)?;
                }
                Stmt::If {
                    cond,
                    then_body,
                    else_body,
                    ..
                } => {
                    self.check_expr(cond)?;
                    let mut then_tracker = LinearTracker {
                        states: self.states.clone(),
                        aggregate_fields: self.aggregate_fields.clone(),
                    };
                    then_tracker.check_scoped_stmts(then_body)?;
                    let mut else_tracker = LinearTracker {
                        states: self.states.clone(),
                        aggregate_fields: self.aggregate_fields.clone(),
                    };
                    else_tracker.check_scoped_stmts(else_body)?;
                    self.merge_branch_states(&[then_tracker.states, else_tracker.states]);
                }
                Stmt::Loop { body, .. } => {
                    self.check_loop_body(body)?;
                }
                Stmt::For { iterable, body, .. } | Stmt::WhileIn { iterable, body, .. } => {
                    self.check_expr(iterable)?;
                    self.check_loop_body(body)?;
                }
                Stmt::While { cond, body, .. } => {
                    self.check_expr(cond)?;
                    self.check_loop_body(body)?;
                }
                Stmt::Return(expr, _) => {
                    self.check_expr(expr)?;
                }
                Stmt::Fn { body, .. } => {
                    let mut fn_tracker = LinearTracker::new();
                    fn_tracker.check_stmts(body)?;

                    for (name, state) in fn_tracker.states {
                        match state {
                            TrackedLinearState::Available => {
                                return Err(Diagnostic::error(
                                    error_codes::TYPE_MISMATCH,
                                    format!("linear variable '{}' not consumed", name),
                                ));
                            }
                            TrackedLinearState::PartiallyMoved => {
                                return Err(Diagnostic::error(
                                    error_codes::TYPE_MISMATCH,
                                    format!("linear aggregate '{}' only partially consumed", name),
                                ));
                            }
                            TrackedLinearState::MaybeMoved => {
                                return Err(Diagnostic::error(
                                    error_codes::TYPE_MISMATCH,
                                    format!(
                                        "linear variable '{}' not consumed on every path",
                                        name
                                    ),
                                ));
                            }
                            TrackedLinearState::Moved => {}
                        }
                    }
                }
                _ => {}
            }
            Ok(())
        }

        fn check_expr(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
            match expr {
                Expr::Var(name, _) => {
                    self.consume_linear_place(name)?;
                }
                Expr::Call(_, args, _) => {
                    for arg in args {
                        self.check_expr(arg)?;
                    }
                }
                Expr::BinaryOp { left, right, .. } => {
                    self.check_expr(left)?;
                    self.check_expr(right)?;
                }
                Expr::UnaryOp { inner, .. } => {
                    self.check_expr(inner)?;
                }
                Expr::IfExpr {
                    cond, then, else_, ..
                } => {
                    self.check_expr(cond)?;
                    let mut then_tracker = LinearTracker {
                        states: self.states.clone(),
                        aggregate_fields: self.aggregate_fields.clone(),
                    };
                    then_tracker.check_expr(then)?;
                    let mut else_tracker = LinearTracker {
                        states: self.states.clone(),
                        aggregate_fields: self.aggregate_fields.clone(),
                    };
                    else_tracker.check_expr(else_)?;
                    self.merge_branch_states(&[then_tracker.states, else_tracker.states]);
                }
                Expr::Block(stmts, _) => {
                    self.check_scoped_stmts(stmts)?;
                }
                Expr::Tuple(exprs, _) => {
                    for e in exprs {
                        self.check_expr(e)?;
                    }
                }
                Expr::Array(exprs, _) => {
                    for e in exprs {
                        self.check_expr(e)?;
                    }
                }
                Expr::Match { expr, arms, .. } => {
                    self.check_expr(expr)?;
                    if !arms.is_empty() {
                        let mut branch_states = Vec::with_capacity(arms.len());
                        for arm in arms {
                            let mut arm_tracker = LinearTracker {
                                states: self.states.clone(),
                                aggregate_fields: self.aggregate_fields.clone(),
                            };
                            arm_tracker.check_expr(&arm.body)?;
                            branch_states.push(arm_tracker.states);
                        }
                        self.merge_branch_states(&branch_states);
                    }
                }
                Expr::FieldAccess { base, field, .. } => {
                    if let Some(root) = Self::linear_root_name(base) {
                        if self.states.contains_key(root) {
                            let place = format!(
                                "{}.{}",
                                Self::expr_place_path(base).unwrap_or_else(|| root.to_string()),
                                field
                            );
                            self.consume_linear_place(&place)?;
                        } else {
                            self.check_expr(base)?;
                        }
                    } else {
                        self.check_expr(base)?;
                    }
                }
                Expr::Index(base, idx, _) => {
                    self.check_expr(base)?;
                    self.check_expr(idx)?;
                }
                Expr::Range { start, end, .. } => {
                    self.check_expr(start)?;
                    self.check_expr(end)?;
                }
                _ => {}
            }
            Ok(())
        }
    }

    let mut tracker = LinearTracker::new();
    tracker.check_stmts(&prog.stmts)
}
