use omni_compiler::ast::Stmt;
use omni_compiler::complete_lexer::tokenize_complete;
use omni_compiler::parser::Parser;

fn parse(source: &str) -> omni_compiler::ast::Program {
    let tokens = tokenize_complete(source).expect("lex");
    let mut parser = Parser::new(tokens);
    parser.parse_program().expect("parse")
}

#[test]
fn negative_generic_bounds_remain_parseable() {
    let program = parse("fn probe[T: !Copy](x: T) { return; }\n");
    let Stmt::Fn { type_params, .. } = &program.stmts[0] else {
        panic!("fn")
    };
    assert_eq!(
        type_params,
        &vec![("T".to_string(), vec!["!Copy".to_string()])]
    );
}

#[test]
fn sealed_enum_prefix_is_preserved_as_transitional_v0_1_x_syntax() {
    let program = parse("sealed enum ResultKind { Ok, Err }\n");
    assert!(matches!(
        &program.stmts[0],
        Stmt::Enum {
            is_sealed: true,
            ..
        }
    ));
}

#[test]
fn stage0_module_declaration_is_non_executable_and_preserved() {
    let program = parse("module historical.exit42;\nfn main() -> i64 { return 42; }\n");
    assert!(matches!(&program.stmts[0], Stmt::Mod(name, _) if name == "historical.exit42"));
    let diagnostics = omni_compiler::control_flow::validate(&program);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}
