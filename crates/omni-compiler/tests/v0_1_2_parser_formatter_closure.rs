use omni_compiler::ast::Stmt;
use omni_compiler::complete_lexer::tokenize_complete;
use omni_compiler::formatter::{format_program, format_program_with_config, FormatterConfig};
use omni_compiler::parser::Parser;

fn parse(src: &str) -> omni_compiler::ast::Program {
    let tokens = tokenize_complete(src).expect("lex");
    let mut parser = Parser::new(tokens);
    parser.parse_program().expect("parse")
}

#[test]
fn brace_if_while_loop_and_for_bodies_are_not_dropped() {
    let src = r#"
fn main() -> i64 {
    let x: i64 = 0;
    if true { print 1; } else { print 2; }
    while false { print 3; }
    loop { break; }
    for item in 1 { print item; }
    return x;
}
"#;
    let program = parse(src);
    let Stmt::Fn { body, .. } = &program.stmts[0] else {
        panic!("expected fn")
    };
    assert!(
        matches!(&body[1], Stmt::If { then_body, else_body, .. } if then_body.len() == 1 && else_body.len() == 1)
    );
    assert!(matches!(&body[2], Stmt::While { body, .. } if body.len() == 1));
    assert!(matches!(&body[3], Stmt::Loop { body, .. } if body.len() == 1));
    assert!(matches!(&body[4], Stmt::For { body, .. } if body.len() == 1));
}

#[test]
fn assignment_formatting_does_not_turn_mutation_into_declaration() {
    let program = parse("fn main() -> i64 { let x: i64 = 1; x = 2; return x; }\n");
    let formatted = format_program(&program);
    assert!(formatted.contains("x = 2"));
    assert!(!formatted.contains("let x = 2"));
    let reparsed = parse(&formatted);
    let Stmt::Fn { body, .. } = &reparsed.stmts[0] else {
        panic!("expected fn")
    };
    assert!(matches!(body[1], Stmt::Assign(_, _, _)));
}

#[test]
fn while_in_formatting_preserves_while_in_ast() {
    let program = parse("fn probe() { while item in 3 { print item; } }\n");
    let formatted = format_program(&program);
    assert!(formatted.contains("while item in 3"));
    let reparsed = parse(&formatted);
    let Stmt::Fn { body, .. } = &reparsed.stmts[0] else {
        panic!("expected fn")
    };
    assert!(matches!(body[0], Stmt::WhileIn { .. }));
}

#[test]
fn formatter_preserves_local_type_annotations_and_effects() {
    let program = parse("fn main() -> i64 / io + panic { let x: i64 = 1; return x; }\n");
    let formatted = format_program(&program);
    assert!(formatted.contains("let x: i64 = 1"));
    assert!(formatted.contains("/ io + panic"));
}

#[test]
fn strict_formatter_is_deterministic_and_idempotent_for_supported_ast() {
    let program = parse("use zed\nuse alpha\nfn main() -> i64 { return 0; }\n");
    let config = FormatterConfig::new().with_strict_mode(true);
    let once = format_program_with_config(&program, &config);
    let twice_program = parse(&once);
    let twice = format_program_with_config(&twice_program, &config);
    assert_eq!(once, twice);
    assert!(once.find("use alpha").unwrap() < once.find("use zed").unwrap());
}

#[test]
fn formatter_preserves_visibility_async_and_contracts() {
    let src = r#"pub async fn compute(x: i64) -> i64 / io + panic @requires(x > 0, "positive") @ensures(x > 0) @invariant(x > 0) @comptime_limit(ops: 42) { return x; }"#;
    let program = parse(src);
    let formatted = format_program(&program);
    assert!(formatted.contains("pub async fn compute"));
    assert!(formatted.contains("/ io + panic"));
    assert!(formatted.contains("@requires(x > 0, \"positive\")"));
    assert!(formatted.contains("@ensures(x > 0)"));
    assert!(formatted.contains("@invariant(x > 0)"));
    assert!(formatted.contains("@comptime_limit(ops: 42)"));
    let reparsed = parse(&formatted);
    let Stmt::Fn {
        visibility,
        is_async,
        contracts,
        ..
    } = &reparsed.stmts[0]
    else {
        panic!("expected fn")
    };
    assert!(matches!(visibility, omni_compiler::ast::Visibility::Pub));
    assert!(*is_async);
    assert_eq!(contracts.len(), 4);
}

#[test]
fn formatter_roundtrips_brace_impl_and_trait() {
    let src = r#"
pub impl Thing {
    pub fn value() -> i64 { return 1; }
}
pub trait Readable {
    @diagnostic::on_unimplemented(message = "missing", label = "Readable")
    pub fn read() -> i64 { return 1; }
}
"#;
    let program = parse(src);
    assert!(matches!(&program.stmts[0], Stmt::Impl { methods, .. } if methods.len() == 1));
    assert!(
        matches!(&program.stmts[1], Stmt::Trait { methods, diagnostic_attrs, .. } if methods.len() == 1 && diagnostic_attrs.len() == 1)
    );
    let once = format_program(&program);
    let twice = format_program(&parse(&once));
    assert_eq!(once, twice);
}

#[test]
fn formatter_preserves_declaration_visibility_and_error_sets() {
    let src = r#"
pub struct Point { x: i64 }
pub enum Choice { A }
pub type Count = i64
pub(friend: core) error set Errors { variant Bad }
"#;
    let program = parse(src);
    let formatted = format_program(&program);
    assert!(formatted.contains("pub struct Point"));
    assert!(formatted.contains("pub enum Choice"));
    assert!(formatted.contains("pub type Count = i64"));
    assert!(formatted.contains("pub(friend: core) error set Errors"));
    let reparsed = parse(&formatted);
    assert_eq!(program.stmts.len(), reparsed.stmts.len());
}

#[test]
fn historical_linear_struct_spelling_is_preserved_semantically() {
    let program = parse("linear struct Buffer { value: i64 }\n");
    assert!(matches!(
        &program.stmts[0],
        Stmt::Struct {
            is_linear: true,
            ..
        }
    ));
    let formatted = format_program(&program);
    let reparsed = parse(&formatted);
    assert!(matches!(
        &reparsed.stmts[0],
        Stmt::Struct {
            is_linear: true,
            ..
        }
    ));
}

#[test]
fn literal_values_are_not_fabricated() {
    let program = parse(
        "fn main() { let a = 1.5; let b = 0x2a; let c = 0o52; let d = 0b101010; let e = '\\n'; }",
    );
    let rendered = format_program(&program);
    assert!(
        rendered.contains("1.5"),
        "float literal must retain its value: {rendered}"
    );
    assert!(
        rendered.contains("42"),
        "radix literals must retain their numeric value: {rendered}"
    );
    assert!(
        rendered.contains("'\\n'"),
        "character escapes must round-trip canonically: {rendered}"
    );
}

#[test]
fn truncated_delimited_expressions_are_rejected() {
    for source in [
        "fn main() { let x = (1 + 2; }",
        "fn main() { let x = fn(a: i64 { a; }; }",
        "fn main() { let x = Point { value: 1; }",
        "fn main() { let x = match 1 { | 1 => 2;",
    ] {
        let mut lexer = omni_compiler::complete_lexer::CompleteLexer::new(source);
        let tokens = lexer
            .tokenize()
            .expect("lexing malformed delimiter case must still succeed");
        let mut parser = Parser::new(tokens);
        assert!(
            parser.parse_program().is_err(),
            "parser accepted malformed source: {source}"
        );
    }
}

#[test]
fn historical_byte_string_gate_now_preserves_binary_ast_semantics() {
    let source = r#"fn main() { let x = b"A\x00\xFF"; }"#;
    let mut lexer = omni_compiler::complete_lexer::CompleteLexer::new(source);
    let tokens = lexer.tokenize().expect("byte string lexes");
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().expect("v0.1.4 byte-string AST");
    let rendered = format_program(&program);
    assert!(rendered.contains(r#"b"A\0\xFF""#), "{rendered}");
    let mut lexer = omni_compiler::complete_lexer::CompleteLexer::new(&rendered);
    let tokens = lexer.tokenize().expect("formatted byte string re-lexes");
    let mut parser = Parser::new(tokens);
    let rerendered = format_program(
        &parser
            .parse_program()
            .expect("formatted byte string reparses"),
    );
    assert_eq!(rendered, rerendered);
}

#[test]
fn interpolated_strings_roundtrip_without_losing_expressions() {
    for source in [
        r#"fn main() { let name = "Omni"; let x = f"Hello ${name}"; }"#,
        r#"fn main() { let name = "Omni"; let x = f"Hello {name}"; }"#,
    ] {
        let program = parse(source);
        let rendered = format_program(&program);
        assert!(
            rendered.contains("f\"Hello ${name}\""),
            "canonical interpolation missing: {rendered}"
        );
        let reparsed = parse(&rendered);
        let Stmt::Fn { body, .. } = &reparsed.stmts[0] else {
            panic!("expected fn")
        };
        let Stmt::Let(_, _, omni_compiler::ast::Expr::Interpolated(parts, _), _) = &body[1] else {
            panic!("expected interpolated let")
        };
        assert_eq!(parts.len(), 2);
    }
}

#[test]
fn raw_strings_and_raw_identifiers_do_not_corrupt_tokens() {
    let program = parse(r##"fn main() { let r#type = r"plain"; let other = r#"hash"#; }"##);
    let rendered = format_program(&program);
    assert!(
        rendered.contains("let r#type"),
        "raw keyword identifier must be re-escaped: {rendered}"
    );
    assert!(rendered.contains("plain"));
    assert!(rendered.contains("hash"));
    let _ = parse(&rendered);
}

#[test]
fn mismatched_and_unterminated_statement_blocks_are_rejected() {
    for source in [
        "fn main() { let x = 1;",
        "fn main() { if true { return; ] }",
        "fn main() { loop { break; ] }",
        "fn main() { while true { break; ] }",
        "fn main() { unsafe { return; ] }",
    ] {
        let tokens = tokenize_complete(source).expect("lexer should expose parser delimiter error");
        let mut parser = Parser::new(tokens);
        assert!(
            parser.parse_program().is_err(),
            "parser accepted malformed block: {source}"
        );
    }
}

#[test]
fn identifier_control_conditions_are_not_misparsed_as_struct_literals() {
    let program =
        parse("fn choose(flag: bool) -> i64 { if flag { return 1; } else { return 2; } }\n");
    let Stmt::Fn { body, .. } = &program.stmts[0] else {
        panic!("expected function")
    };
    let Stmt::If { cond, .. } = &body[0] else {
        panic!("expected if")
    };
    assert!(matches!(cond.as_ref(), omni_compiler::ast::Expr::Var(name, _) if name == "flag"));
}

#[test]
fn enum_constructor_and_variant_patterns_survive_formatter_roundtrip() {
    let src = r#"
enum Flag { variant Off, variant On, }
fn main() -> i64 {
    let flag = Flag::On();
    return match flag {
        Flag::Off[] => 7,
        Flag::On[] => 42,
    };
}
"#;
    let program = parse(src);
    let once = format_program(&program);
    assert!(
        once.contains("Flag::On()"),
        "constructor identity lost: {once}"
    );
    assert!(
        once.contains("Flag::Off[]"),
        "fieldless variant pattern lost: {once}"
    );
    assert!(
        once.contains("Flag::On[]"),
        "fieldless variant pattern lost: {once}"
    );
    let twice = format_program(&parse(&once));
    assert_eq!(once, twice, "enum formatting must be idempotent");
}
