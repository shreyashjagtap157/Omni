use omni_compiler::ast::{Expr, Program, Stmt};
use omni_compiler::async_effects::{
    check_async_compatibility, compose_effects, make_async, make_generator, AsyncContext,
    EffectPolymorphism, FutureState, FutureType, EF_ASYNC,
};
use omni_compiler::comptime::ComptimeContext;
use omni_compiler::macros::{
    FragmentReplacement, FragmentSpecifier, MacroArg, MacroDefinition, MacroExpansionContext,
    MacroPattern, MacroRule, MacroToken,
};
use omni_compiler::traits::{ImplMethod, MethodSignature, TraitDefinition, TraitImpl, TraitSystem};
use omni_compiler::type_checker::{Type, EF_IO};

#[test]
fn comptime_evaluates_basic_expression() {
    let program = Program {
        stmts: vec![Stmt::ExprStmt(Expr::BinaryOp {
            op: omni_compiler::lexer::TokenKind::Plus,
            left: Box::new(Expr::Number(2)),
            right: Box::new(Expr::Number(3)),
        })],
    };

    let mut context = ComptimeContext::new();
    let value = context.eval_program(&program).expect("comptime failed");
    assert_eq!(value, omni_compiler::comptime::ComptimeValue::Int(5));
}

#[test]
fn comptime_match_expression_evaluates() {
    use omni_compiler::ast::MatchArm;
    use omni_compiler::ast::Pattern;

    let program = Program {
        stmts: vec![Stmt::ExprStmt(Expr::Match {
            expr: Box::new(Expr::Number(1)),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Literal(0),
                    guard: None,
                    body: Box::new(Expr::Number(0)),
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: Box::new(Expr::Number(7)),
                },
            ],
        })],
    };

    let mut context = ComptimeContext::new();
    let value = context
        .eval_program(&program)
        .expect("comptime match failed");
    assert_eq!(value, omni_compiler::comptime::ComptimeValue::Int(7));
}

#[test]
fn trait_system_registers_impls() {
    let mut system = TraitSystem::new();

    let printable = TraitDefinition {
        name: "Printable".to_string(),
        type_params: vec!["Self".to_string()],
        bounds: vec![],
        supertraits: vec![],
        methods: vec![MethodSignature {
            name: "fmt_string".to_string(),
            params: vec![],
            return_type: Type::String,
            effect: 0,
        }],
        required_methods: vec!["fmt_string".to_string()],
        is_sealed: false,
    };
    system.add_trait(printable).expect("trait add failed");

    let impl_def = TraitImpl {
        trait_name: "Printable".to_string(),
        impl_type: Type::String,
        methods: vec![ImplMethod {
            name: "fmt_string".to_string(),
            body: vec![],
        }],
    };
    system.add_impl(impl_def).expect("impl add failed");

    assert!(system.check_trait_bound(&Type::String, "Printable"));
    assert_eq!(system.get_impls_for_type(&Type::String).len(), 1);
}

#[test]
fn trait_upcasting_negative_bounds_and_implied_bounds_work() {
    let mut system = TraitSystem::new();

    let readable = TraitDefinition {
        name: "Readable".to_string(),
        type_params: vec!["Self".to_string()],
        bounds: vec![],
        supertraits: vec![],
        methods: vec![],
        required_methods: vec![],
        is_sealed: false,
    };
    system.add_trait(readable).expect("readable add failed");

    let seekable = TraitDefinition {
        name: "Seekable".to_string(),
        type_params: vec!["Self".to_string()],
        bounds: vec![],
        supertraits: vec!["Readable".to_string()],
        methods: vec![],
        required_methods: vec![],
        is_sealed: false,
    };
    system.add_trait(seekable).expect("seekable add failed");

    let impl_def = TraitImpl {
        trait_name: "Seekable".to_string(),
        impl_type: Type::String,
        methods: vec![],
    };
    system.add_impl(impl_def).expect("seekable impl add failed");

    assert!(system.can_upcast_trait("Seekable", "Readable"));
    assert!(!system.can_upcast_trait("Readable", "Seekable"));
    assert!(system.satisfies_negative_bound(&Type::Bool, "Seekable"));

    let implied = system.implied_bounds_for_type(&Type::String);
    assert!(implied.iter().any(|b| b.trait_name == "Seekable"));
    assert!(implied.iter().any(|b| b.trait_name == "Readable"));
}

#[test]
fn macro_expansion_matches_fragment_bindings() {
    let mut ctx = MacroExpansionContext::new();
    let definition = MacroDefinition {
        name: "identity_expr".to_string(),
        rules: vec![MacroRule {
            pattern: vec![MacroPattern::Fragment(
                "value".to_string(),
                FragmentSpecifier::Expr,
            )],
            template: vec![MacroToken::Fragment(
                "value".to_string(),
                FragmentReplacement::Expr,
            )],
        }],
        is_macro_rules: true,
    };
    ctx.add_macro(definition);

    let expanded = ctx
        .expand_macro("identity_expr", &[MacroArg::Expr(Expr::Number(9))])
        .expect("macro expansion failed");
    assert!(matches!(
        expanded.as_slice(),
        [Stmt::ExprStmt(Expr::Number(9))]
    ));
}

#[test]
fn async_scope_and_generator_behave() {
    let mut context = AsyncContext::new();
    let future = FutureType {
        inner_type: Type::Int,
        state: FutureState::Pending,
    };

    let task_name;
    {
        let mut scope = context.spawn_scope();
        task_name = scope.spawn("work".to_string(), future);
        assert_eq!(scope.finish().expect("scope join failed"), Type::Unit);
    }

    assert!(!context.tasks.contains_key(&task_name));
    assert_eq!(context.poll(&task_name), None);

    let values: Vec<_> = make_generator(vec![1, 2, 3]).collect();
    assert_eq!(values, vec![1, 2, 3]);

    assert_eq!(compose_effects(&[EF_IO, EF_ASYNC]), EF_IO | EF_ASYNC);
    let poly = EffectPolymorphism::new();
    assert_eq!(
        poly.unify_effects(EF_IO, EF_ASYNC)
            .expect("effect unify failed"),
        EF_IO | EF_ASYNC
    );
}

#[test]
fn async_scope_drop_cleans_up_spawned_tasks() {
    let mut context = AsyncContext::new();
    let future = FutureType {
        inner_type: Type::Int,
        state: FutureState::Pending,
    };

    let task_name;
    {
        let mut scope = context.spawn_scope();
        task_name = scope.spawn("background".to_string(), future);
    }

    assert!(!context.tasks.contains_key(&task_name));
    assert_eq!(context.poll(&task_name), None);
}

#[test]
fn async_context_tracks_tasks() {
    let mut context = AsyncContext::new();
    let future = FutureType {
        inner_type: Type::Int,
        state: FutureState::Pending,
    };

    let task_name = context.spawn("work".to_string(), future);
    assert_eq!(context.poll(&task_name), Some(Type::Int));
    assert!(check_async_compatibility(make_async(0), make_async(0)).is_ok());
    assert!(check_async_compatibility(0, make_async(0)).is_err());
}

#[test]
fn variadic_generic_tuple_types_work() {
    use omni_compiler::async_effects::VariadicGeneric;

    let types = vec![Type::Int, Type::String, Type::Bool];

    let variadic = VariadicGeneric::from_types(&types);
    assert_eq!(variadic.len(), 3);
    assert_eq!(variadic.get(0), Some(&Type::Int));
    assert_eq!(variadic.get(1), Some(&Type::String));
    assert_eq!(variadic.get(2), Some(&Type::Bool));
    assert!(variadic.get(3).is_none());
}

#[test]
fn variadic_generic_empty_works() {
    use omni_compiler::async_effects::VariadicGeneric;

    let variadic = VariadicGeneric::new();
    assert!(variadic.is_empty());
    assert_eq!(variadic.len(), 0);
    assert!(variadic.get(0).is_none());
}

#[test]
fn variadic_generic_iteration_works() {
    use omni_compiler::async_effects::VariadicGeneric;

    let types = vec![Type::Int, Type::String];
    let variadic = VariadicGeneric::from_types(&types);

    let collected: Vec<_> = variadic.iter().collect();
    assert_eq!(collected.len(), 2);
}

#[test]
fn make_variadic_fn_generates_correct_params() {
    use omni_compiler::async_effects::make_variadic_fn;

    let types = vec![Type::Int, Type::String, Type::Bool];
    let (name, params) = make_variadic_fn("foo", "arg", &types);

    assert_eq!(name, "foo_variadic_3");
    assert_eq!(params.len(), 3);
    assert_eq!(params[0].0, "arg_0");
    assert_eq!(params[1].0, "arg_1");
    assert_eq!(params[2].0, "arg_2");
    assert_eq!(params[0].1, Type::Int);
    assert_eq!(params[1].1, Type::String);
    assert_eq!(params[2].1, Type::Bool);
}

#[test]
fn trait_specialization_for_concrete_types() {
    use omni_compiler::traits::{
        ImplMethod, MethodSignature, TraitDefinition, TraitImpl, TraitSystem,
    };
    use omni_compiler::type_checker::Type;

    let mut system = TraitSystem::new();

    // Add a generic trait
    let trait_def = TraitDefinition {
        name: "Double".to_string(),
        type_params: vec!["Self".to_string()],
        bounds: vec![],
        supertraits: vec![],
        methods: vec![MethodSignature {
            name: "double".to_string(),
            params: vec![],
            return_type: Type::Int,
            effect: 0,
        }],
        required_methods: vec!["double".to_string()],
        is_sealed: false,
    };
    system.add_trait(trait_def).expect("trait add failed");

    // Add generic impl
    let generic_impl = TraitImpl {
        trait_name: "Double".to_string(),
        impl_type: Type::Generic("T".to_string()),
        methods: vec![ImplMethod {
            name: "double".to_string(),
            body: vec![],
        }],
    };
    system.add_impl(generic_impl).expect("impl add failed");

    // Add specialized impl for i32
    let i32_impl = TraitImpl {
        trait_name: "Double".to_string(),
        impl_type: Type::Int,
        methods: vec![ImplMethod {
            name: "double".to_string(),
            body: vec![],
        }],
    };
    system.add_impl(i32_impl).expect("i32 impl add failed");

    // Check that we can find both impls
    let all_impls = system.get_impls_for_type(&Type::Generic("T".to_string()));
    assert!(!all_impls.is_empty());

    let i32_impls = system.get_impls_for_type(&Type::Int);
    assert!(!i32_impls.is_empty());
}

#[test]
fn trait_supertrait_resolution() {
    use omni_compiler::traits::{MethodSignature, TraitDefinition, TraitSystem};
    use omni_compiler::type_checker::Type;

    let mut system = TraitSystem::new();

    // Add Printable trait (not already built-in)
    let printable = TraitDefinition {
        name: "Printable".to_string(),
        type_params: vec!["Self".to_string()],
        bounds: vec![],
        supertraits: vec![],
        methods: vec![MethodSignature {
            name: "to_string".to_string(),
            params: vec![],
            return_type: Type::String,
            effect: 0,
        }],
        required_methods: vec!["to_string".to_string()],
        is_sealed: false,
    };
    system.add_trait(printable).expect("Printable add failed");

    // Add Showable that has Printable as supertrait
    let showable = TraitDefinition {
        name: "Showable".to_string(),
        type_params: vec!["Self".to_string()],
        bounds: vec![],
        supertraits: vec!["Printable".to_string()],
        methods: vec![MethodSignature {
            name: "show".to_string(),
            params: vec![],
            return_type: Type::String,
            effect: 0,
        }],
        required_methods: vec!["show".to_string()],
        is_sealed: false,
    };
    system.add_trait(showable).expect("Showable add failed");

    // Verify Showable has Printable as supertrait
    let showable_trait = system.traits.get("Showable").expect("Showable not found");
    assert_eq!(showable_trait.supertraits.len(), 1);
    assert_eq!(showable_trait.supertraits[0], "Printable");
}

#[test]
fn range_parses_exclusive() {
    use omni_compiler::lexer::Lexer;

    let src = "1..5";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("tokenize failed");

    assert!(tokens
        .iter()
        .any(|t| matches!(t.kind, omni_compiler::lexer::TokenKind::Number)));
    assert!(tokens
        .iter()
        .any(|t| t.kind == omni_compiler::lexer::TokenKind::DotDot));
    assert!(tokens
        .iter()
        .any(|t| matches!(t.kind, omni_compiler::lexer::TokenKind::Number)));
}

#[test]
fn range_parses_inclusive() {
    use omni_compiler::lexer::Lexer;

    let src = "1...5";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("tokenize failed");

    assert!(tokens
        .iter()
        .any(|t| t.kind == omni_compiler::lexer::TokenKind::DotDotDot));
}
