use omni_compiler::types::{unify, EffectSet, Type};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Type::apply_substitution tests
// ---------------------------------------------------------------------------

#[test]
fn subst_generic_to_int() {
    let mut subst = HashMap::new();
    subst.insert("T".to_string(), Type::Int);
    assert_eq!(
        Type::Generic("T".to_string()).apply_substitution(&subst),
        Type::Int
    );
}

#[test]
fn subst_unbound_generic_unchanged() {
    let subst = HashMap::new();
    let ty = Type::Generic("U".to_string());
    assert_eq!(ty.apply_substitution(&subst), ty);
}

#[test]
fn subst_concrete_unchanged() {
    let mut subst = HashMap::new();
    subst.insert("T".to_string(), Type::Int);
    assert_eq!(Type::Bool.apply_substitution(&subst), Type::Bool);
    assert_eq!(Type::String.apply_substitution(&subst), Type::String);
    assert_eq!(Type::Unit.apply_substitution(&subst), Type::Unit);
}

#[test]
fn subst_ref_inner() {
    let mut subst = HashMap::new();
    subst.insert("T".to_string(), Type::Int);
    let ty = Type::Ref {
        mutable: false,
        inner: Box::new(Type::Generic("T".to_string())),
    };
    let expected = Type::Ref {
        mutable: false,
        inner: Box::new(Type::Int),
    };
    assert_eq!(ty.apply_substitution(&subst), expected);
}

#[test]
fn subst_fn_type() {
    let mut subst = HashMap::new();
    subst.insert("T".to_string(), Type::Int);
    let ty = Type::Fn {
        params: vec![Type::Generic("T".to_string())],
        ret: Box::new(Type::Generic("T".to_string())),
        effects: EffectSet::new(),
    };
    let expected = Type::Fn {
        params: vec![Type::Int],
        ret: Box::new(Type::Int),
        effects: EffectSet::new(),
    };
    assert_eq!(ty.apply_substitution(&subst), expected);
}

#[test]
fn subst_option() {
    let mut subst = HashMap::new();
    subst.insert("T".to_string(), Type::Bool);
    let ty = Type::Option(Box::new(Type::Generic("T".to_string())));
    let expected = Type::Option(Box::new(Type::Bool));
    assert_eq!(ty.apply_substitution(&subst), expected);
}

#[test]
fn subst_result() {
    let mut subst = HashMap::new();
    subst.insert("T".to_string(), Type::Int);
    subst.insert("E".to_string(), Type::String);
    let ty = Type::Result(
        Box::new(Type::Generic("T".to_string())),
        Box::new(Type::Generic("E".to_string())),
    );
    let expected = Type::Result(Box::new(Type::Int), Box::new(Type::String));
    assert_eq!(ty.apply_substitution(&subst), expected);
}

#[test]
fn subst_chained() {
    // T -> Option<U>,  U -> Int  =>  Generic("T") becomes Option(Int)
    let mut subst = HashMap::new();
    subst.insert(
        "T".to_string(),
        Type::Option(Box::new(Type::Generic("U".to_string()))),
    );
    subst.insert("U".to_string(), Type::Int);
    let result = Type::Generic("T".to_string()).apply_substitution(&subst);
    assert_eq!(result, Type::Option(Box::new(Type::Int)));
}

// ---------------------------------------------------------------------------
// Type::contains_generic tests
// ---------------------------------------------------------------------------

#[test]
fn contains_generic_direct() {
    assert!(Type::Generic("T".to_string()).contains_generic("T"));
    assert!(!Type::Generic("T".to_string()).contains_generic("U"));
}

#[test]
fn contains_generic_nested() {
    let ty = Type::Option(Box::new(Type::Generic("T".to_string())));
    assert!(ty.contains_generic("T"));
    assert!(!ty.contains_generic("U"));
}

#[test]
fn contains_generic_concrete() {
    assert!(!Type::Int.contains_generic("T"));
    assert!(!Type::Unit.contains_generic("T"));
}

// ---------------------------------------------------------------------------
// unify tests
// ---------------------------------------------------------------------------

#[test]
fn unify_same_concrete() {
    let mut subst = HashMap::new();
    assert!(unify(&Type::Int, &Type::Int, &mut subst).is_ok());
    assert!(subst.is_empty());
}

#[test]
fn unify_concrete_mismatch() {
    let mut subst = HashMap::new();
    assert!(unify(&Type::Int, &Type::Bool, &mut subst).is_err());
}

#[test]
fn unify_generic_left_binds() {
    let mut subst = HashMap::new();
    assert!(unify(&Type::Generic("T".to_string()), &Type::Int, &mut subst).is_ok());
    assert_eq!(subst.get("T"), Some(&Type::Int));
}

#[test]
fn unify_generic_right_binds() {
    let mut subst = HashMap::new();
    assert!(unify(&Type::Int, &Type::Generic("T".to_string()), &mut subst).is_ok());
    assert_eq!(subst.get("T"), Some(&Type::Int));
}

#[test]
fn unify_generic_consistent() {
    // T = Int, then T must also match Int
    let mut subst = HashMap::new();
    subst.insert("T".to_string(), Type::Int);
    assert!(unify(&Type::Generic("T".to_string()), &Type::Int, &mut subst).is_ok());
}

#[test]
fn unify_generic_inconsistent() {
    // T = Int, then T vs Bool should fail
    let mut subst = HashMap::new();
    subst.insert("T".to_string(), Type::Int);
    assert!(unify(&Type::Generic("T".to_string()), &Type::Bool, &mut subst).is_err());
}

#[test]
fn unify_two_generics() {
    // T vs U — one binds to the other
    let mut subst = HashMap::new();
    assert!(unify(
        &Type::Generic("T".to_string()),
        &Type::Generic("U".to_string()),
        &mut subst
    )
    .is_ok());
    // T is bound to U (or vice versa)
    assert!(subst.contains_key("T") || subst.contains_key("U"));
}

#[test]
fn unify_fn_type_identity() {
    // fn(T) -> T  vs  fn(Int) -> Int  =>  T = Int
    let mut subst = HashMap::new();
    let generic_fn = Type::Fn {
        params: vec![Type::Generic("T".to_string())],
        ret: Box::new(Type::Generic("T".to_string())),
        effects: EffectSet::new(),
    };
    let concrete_fn = Type::Fn {
        params: vec![Type::Int],
        ret: Box::new(Type::Int),
        effects: EffectSet::new(),
    };
    assert!(unify(&generic_fn, &concrete_fn, &mut subst).is_ok());
    assert_eq!(subst.get("T"), Some(&Type::Int));
}

#[test]
fn unify_fn_type_mismatch() {
    // fn(T) -> T  vs  fn(Int) -> Bool  should fail (T can't be both Int and Bool)
    let mut subst = HashMap::new();
    let generic_fn = Type::Fn {
        params: vec![Type::Generic("T".to_string())],
        ret: Box::new(Type::Generic("T".to_string())),
        effects: EffectSet::new(),
    };
    let concrete_fn = Type::Fn {
        params: vec![Type::Int],
        ret: Box::new(Type::Bool),
        effects: EffectSet::new(),
    };
    assert!(unify(&generic_fn, &concrete_fn, &mut subst).is_err());
}

#[test]
fn unify_option_inner() {
    let mut subst = HashMap::new();
    assert!(unify(
        &Type::Option(Box::new(Type::Generic("T".to_string()))),
        &Type::Option(Box::new(Type::Int)),
        &mut subst,
    )
    .is_ok());
    assert_eq!(subst.get("T"), Some(&Type::Int));
}

#[test]
fn unify_result_binds_both() {
    let mut subst = HashMap::new();
    assert!(unify(
        &Type::Result(
            Box::new(Type::Generic("T".to_string())),
            Box::new(Type::Generic("E".to_string())),
        ),
        &Type::Result(Box::new(Type::Int), Box::new(Type::String)),
        &mut subst,
    )
    .is_ok());
    assert_eq!(subst.get("T"), Some(&Type::Int));
    assert_eq!(subst.get("E"), Some(&Type::String));
}

#[test]
fn unify_ref_mutability_mismatch() {
    let mut subst = HashMap::new();
    let immut = Type::Ref {
        mutable: false,
        inner: Box::new(Type::Int),
    };
    let muta = Type::Ref {
        mutable: true,
        inner: Box::new(Type::Int),
    };
    assert!(unify(&immut, &muta, &mut subst).is_err());
}

#[test]
fn unify_occurs_check() {
    // T vs Option<T> should fail (infinite type)
    let mut subst = HashMap::new();
    assert!(unify(
        &Type::Generic("T".to_string()),
        &Type::Option(Box::new(Type::Generic("T".to_string()))),
        &mut subst,
    )
    .is_err());
}

#[test]
fn unify_fn_arity_mismatch() {
    let mut subst = HashMap::new();
    let f1 = Type::Fn {
        params: vec![Type::Int],
        ret: Box::new(Type::Int),
        effects: EffectSet::new(),
    };
    let f2 = Type::Fn {
        params: vec![Type::Int, Type::Bool],
        ret: Box::new(Type::Int),
        effects: EffectSet::new(),
    };
    assert!(unify(&f1, &f2, &mut subst).is_err());
}
