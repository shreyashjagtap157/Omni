#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinearState {
    Available,
    Moved,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Effect {
    Pure,
    Io,
    Async,
    Throw(Box<Type>),
    Panic,
    Alloc,
    Rand,
    Time,
    Log,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EffectSet {
    pub effects: Vec<Effect>,
}

impl Default for EffectSet {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectSet {
    pub fn new() -> EffectSet {
        EffectSet {
            effects: Vec::new(),
        }
    }

    pub fn empty() -> EffectSet {
        EffectSet {
            effects: vec![Effect::Pure],
        }
    }

    pub fn with_io() -> EffectSet {
        let mut es = EffectSet::new();
        es.effects.push(Effect::Io);
        es
    }

    pub fn with_async() -> EffectSet {
        let mut es = EffectSet::new();
        es.effects.push(Effect::Async);
        es
    }

    pub fn with_pure() -> EffectSet {
        let mut es = EffectSet::new();
        es.effects.push(Effect::Pure);
        es
    }

    pub fn with_panic() -> EffectSet {
        let mut es = EffectSet::new();
        es.effects.push(Effect::Panic);
        es
    }

    pub fn with_alloc() -> EffectSet {
        let mut es = EffectSet::new();
        es.effects.push(Effect::Alloc);
        es
    }

    pub fn with_rand() -> EffectSet {
        let mut es = EffectSet::new();
        es.effects.push(Effect::Rand);
        es
    }

    pub fn with_time() -> EffectSet {
        let mut es = EffectSet::new();
        es.effects.push(Effect::Time);
        es
    }

    pub fn with_log() -> EffectSet {
        let mut es = EffectSet::new();
        es.effects.push(Effect::Log);
        es
    }

    pub fn with_throw(ty: Type) -> EffectSet {
        let mut es = EffectSet::new();
        es.effects.push(Effect::Throw(Box::new(ty)));
        es
    }

    pub fn with_custom(name: String) -> EffectSet {
        let mut es = EffectSet::new();
        es.effects.push(Effect::Custom(name));
        es
    }

    pub fn from_effect(effect: Effect) -> EffectSet {
        let mut es = EffectSet::new();
        es.effects.push(effect);
        es
    }

    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    pub fn add(&mut self, effect: Effect) {
        if !self.effects.contains(&effect) {
            self.effects.push(effect);
        }
    }

    pub fn contains(&self, effect: &Effect) -> bool {
        self.effects.contains(effect)
    }

    pub fn union(&self, other: &EffectSet) -> EffectSet {
        let mut result = self.clone();
        for e in &other.effects {
            result.add(e.clone());
        }
        result
    }

    pub fn union_with(&mut self, other: &EffectSet) {
        for e in &other.effects {
            self.add(e.clone());
        }
    }

    pub fn difference(&self, other: &EffectSet) -> EffectSet {
        let mut result = EffectSet::new();
        for e in &self.effects {
            if !other.contains(e) {
                result.add(e.clone());
            }
        }
        result
    }

    pub fn to_string_list(&self) -> String {
        self.effects_to_strings().join(", ")
    }

    pub fn non_pure_effect_strings(&self) -> Vec<String> {
        self.effects
            .iter()
            .filter(|e| !matches!(e, Effect::Pure))
            .map(|e| self.effect_to_string(e))
            .collect()
    }

    fn effects_to_strings(&self) -> Vec<String> {
        self.effects
            .iter()
            .map(|e| self.effect_to_string(e))
            .collect()
    }

    fn effect_to_string(&self, e: &Effect) -> String {
        match e {
            Effect::Pure => "pure".to_string(),
            Effect::Io => "io".to_string(),
            Effect::Async => "async".to_string(),
            Effect::Throw(t) => format!("throw<{:?}>", t),
            Effect::Panic => "panic".to_string(),
            Effect::Alloc => "alloc".to_string(),
            Effect::Rand => "rand".to_string(),
            Effect::Time => "time".to_string(),
            Effect::Log => "log".to_string(),
            Effect::Custom(s) => s.clone(),
        }
    }
}

impl std::ops::BitOr for EffectSet {
    type Output = EffectSet;
    fn bitor(self, rhs: EffectSet) -> Self::Output {
        self.union(&rhs)
    }
}

impl std::ops::BitOrAssign for EffectSet {
    fn bitor_assign(&mut self, rhs: EffectSet) {
        self.union_with(&rhs);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Int,
    Float,
    Char,
    Byte,
    String,
    Bytes,
    Bool,
    Var(u32),
    Generic(String),
    Ref {
        mutable: bool,
        inner: Box<Type>,
    },
    Fn {
        params: Vec<Type>,
        ret: Box<Type>,
        effects: EffectSet,
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
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    ErrorSet(String),
    Unit,
    Never,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<Type>,
}

// ---------------------------------------------------------------------------
// Generic type substitution and unification — v0.3.0 foundation
// ---------------------------------------------------------------------------

use std::collections::HashMap;

impl Type {
    /// Recursively replace every `Type::Generic(name)` that appears in `subst`
    /// with the mapped concrete type.  Types that are already concrete or whose
    /// generic name is not in the map are returned unchanged.
    pub fn apply_substitution(&self, subst: &HashMap<String, Type>) -> Type {
        match self {
            Type::Generic(name) => {
                if let Some(concrete) = subst.get(name) {
                    // The concrete type itself may contain generics that need
                    // substitution (e.g. when chaining substitutions).
                    concrete.apply_substitution(subst)
                } else {
                    self.clone()
                }
            }
            Type::Ref { mutable, inner } => Type::Ref {
                mutable: *mutable,
                inner: Box::new(inner.apply_substitution(subst)),
            },
            Type::Fn {
                params,
                ret,
                effects,
            } => Type::Fn {
                params: params.iter().map(|p| p.apply_substitution(subst)).collect(),
                ret: Box::new(ret.apply_substitution(subst)),
                effects: effects.clone(),
            },
            Type::Struct {
                name,
                fields,
                is_linear,
            } => Type::Struct {
                name: name.clone(),
                fields: fields.iter().map(|f| f.apply_substitution(subst)).collect(),
                is_linear: *is_linear,
            },
            Type::Enum {
                name,
                variants,
                is_sealed,
            } => Type::Enum {
                name: name.clone(),
                variants: variants
                    .iter()
                    .map(|v| EnumVariant {
                        name: v.name.clone(),
                        fields: v
                            .fields
                            .iter()
                            .map(|f| f.apply_substitution(subst))
                            .collect(),
                    })
                    .collect(),
                is_sealed: *is_sealed,
            },
            Type::Option(inner) => Type::Option(Box::new(inner.apply_substitution(subst))),
            Type::Result(ok, err) => Type::Result(
                Box::new(ok.apply_substitution(subst)),
                Box::new(err.apply_substitution(subst)),
            ),
            // Concrete leaf types — no substitution needed.
            Type::Int
            | Type::Float
            | Type::Char
            | Type::Byte
            | Type::String
            | Type::Bytes
            | Type::Bool
            | Type::Unit
            | Type::Never
            | Type::Var(_)
            | Type::ErrorSet(_) => self.clone(),
        }
    }

    /// Returns true when this type (or any sub-component) contains
    /// `Type::Generic(name)`.  Used by the occurs-check in unification.
    pub fn contains_generic(&self, name: &str) -> bool {
        match self {
            Type::Generic(n) => n == name,
            Type::Ref { inner, .. } => inner.contains_generic(name),
            Type::Fn { params, ret, .. } => {
                params.iter().any(|p| p.contains_generic(name)) || ret.contains_generic(name)
            }
            Type::Struct { fields, .. } => fields.iter().any(|f| f.contains_generic(name)),
            Type::Enum { variants, .. } => variants
                .iter()
                .any(|v| v.fields.iter().any(|f| f.contains_generic(name))),
            Type::Option(inner) => inner.contains_generic(name),
            Type::Result(ok, err) => ok.contains_generic(name) || err.contains_generic(name),
            _ => false,
        }
    }
}

/// Attempt to unify `expected` with `actual`, filling in `subst` with bindings
/// for generic type parameters.
///
/// When a `Type::Generic(name)` on either side is encountered:
///   * If `name` is already bound in `subst`, the bound type is used instead.
///   * Otherwise `name` is bound to the opposite type (after an occurs-check).
///
/// Concrete types unify only with themselves (nominal equality for structs/enums,
/// structural equality for references, functions, option, result).
///
/// Returns `Ok(())` on success or a human-readable error message on failure.
pub fn unify(
    expected: &Type,
    actual: &Type,
    subst: &mut HashMap<String, Type>,
) -> Result<(), String> {
    // Apply any existing substitutions before comparing.
    let e = expected.apply_substitution(subst);
    let a = actual.apply_substitution(subst);

    match (&e, &a) {
        // Identical concrete types always unify.
        _ if e == a => Ok(()),

        // Generic on the left — bind or check consistency.
        (Type::Generic(name), _) => bind_generic(name, &a, subst),

        // Generic on the right — bind or check consistency.
        (_, Type::Generic(name)) => bind_generic(name, &e, subst),

        // References — mutability must match, inner types unify.
        (
            Type::Ref {
                mutable: m1,
                inner: i1,
            },
            Type::Ref {
                mutable: m2,
                inner: i2,
            },
        ) => {
            if m1 != m2 {
                return Err(format!(
                    "reference mutability mismatch: expected {}, got {}",
                    if *m1 { "&mut" } else { "&" },
                    if *m2 { "&mut" } else { "&" },
                ));
            }
            unify(i1, i2, subst)
        }

        // Function types — parameter count, each param, return type.
        (
            Type::Fn {
                params: p1,
                ret: r1,
                ..
            },
            Type::Fn {
                params: p2,
                ret: r2,
                ..
            },
        ) => {
            if p1.len() != p2.len() {
                return Err(format!(
                    "function parameter count mismatch: expected {}, got {}",
                    p1.len(),
                    p2.len(),
                ));
            }
            for (a, b) in p1.iter().zip(p2.iter()) {
                unify(a, b, subst)?;
            }
            unify(r1, r2, subst)
        }

        // Structs — same name, field-wise unification.
        (
            Type::Struct {
                name: n1,
                fields: f1,
                ..
            },
            Type::Struct {
                name: n2,
                fields: f2,
                ..
            },
        ) => {
            if n1 != n2 {
                return Err(format!(
                    "struct type mismatch: expected '{}', got '{}'",
                    n1, n2,
                ));
            }
            if f1.len() != f2.len() {
                return Err(format!(
                    "struct '{}' field count mismatch: expected {}, got {}",
                    n1,
                    f1.len(),
                    f2.len(),
                ));
            }
            for (a, b) in f1.iter().zip(f2.iter()) {
                unify(a, b, subst)?;
            }
            Ok(())
        }

        // Option — inner unification.
        (Type::Option(i1), Type::Option(i2)) => unify(i1, i2, subst),

        // Result — ok and err unification.
        (Type::Result(o1, e1), Type::Result(o2, e2)) => {
            unify(o1, o2, subst)?;
            unify(e1, e2, subst)
        }

        // Everything else is a type mismatch.
        _ => Err(format!("type mismatch: expected {:?}, got {:?}", e, a)),
    }
}

/// Bind a generic name to a concrete type, performing an occurs-check to
/// prevent infinite types (e.g. `T = Option<T>`).
fn bind_generic(name: &str, ty: &Type, subst: &mut HashMap<String, Type>) -> Result<(), String> {
    // If `ty` is the same generic, it is trivially unified.
    if let Type::Generic(n) = ty {
        if n == name {
            return Ok(());
        }
    }
    // Occurs-check: `name` must not appear inside `ty`.
    if ty.contains_generic(name) {
        return Err(format!("infinite type: '{}' occurs within {:?}", name, ty,));
    }
    subst.insert(name.to_string(), ty.clone());
    Ok(())
}
