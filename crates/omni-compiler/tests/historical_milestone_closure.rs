use omni_compiler::ast::{Stmt, Visibility};
use omni_compiler::complete_lexer::tokenize_complete;
use omni_compiler::formatter::{format_program_with_config, FormatterConfig};
use omni_compiler::parser::Parser;

fn parse(source: &str) -> omni_compiler::ast::Program {
    let tokens = tokenize_complete(source).expect("historical lexer contract");
    let mut parser = Parser::new(tokens);
    parser.parse_program().expect("historical parser contract")
}

#[test]
fn v0_1_1_parses_visibility_effects_contracts_scoped_imports_and_error_sets() {
    let source = r#"
use core.math in:
    let local: i64 = 1

pub(cap: network) fn fetch(x: i64) -> i64 / io + panic @requires(x > 0, "positive") @ensures(x > 0) {
    return x;
}

pub(friend: sibling) error NetworkErrors [Timeout, Closed]
"#;
    let program = parse(source);
    assert!(matches!(&program.stmts[0], Stmt::UseScoped { body, .. } if body.len() == 1));
    assert!(matches!(
        &program.stmts[1],
        Stmt::Fn {
            visibility: Visibility::PubCap(cap),
            effects,
            contracts,
            ..
        } if cap == "network"
            && effects.iter().any(|effect| effect == "io")
            && effects.iter().any(|effect| effect == "panic")
            && contracts.len() == 2
    ));
    assert!(matches!(
        &program.stmts[2],
        Stmt::ErrorSet { visibility: Visibility::PubFriend(friend), variants, .. }
            if friend == "sibling" && variants.len() == 2
    ));
}

#[test]
fn v0_1_2_strict_formatter_is_canonical_for_historical_surface() {
    let source = r#"
use zed
use alpha
pub fn probe(x: i64) -> i64 / io @requires(x > 0, "positive") {
    return x;
}
"#;
    let program = parse(source);
    let config = FormatterConfig::new().with_strict_mode(true);
    let once = format_program_with_config(&program, &config);
    let twice = format_program_with_config(&parse(&once), &config);
    assert_eq!(once, twice);
    assert!(once.find("use alpha").unwrap() < once.find("use zed").unwrap());
    assert!(once.contains("/ io"));
    assert!(once.contains("@requires("));
}
