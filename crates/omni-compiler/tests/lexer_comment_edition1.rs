use omni_compiler::complete_lexer::{tokenize_complete, TokenKind};
use omni_compiler::cst::build_cst;
use omni_compiler::formatter::format_cst_source;

#[test]
fn edition1_line_and_doc_comments_are_classified() {
    let src = "// ordinary\n/// outer doc\n//! inner doc\n";
    let tokens = tokenize_complete(src).expect("lex");
    assert!(tokens
        .iter()
        .any(|t| t.kind == TokenKind::LineComment && t.text == " ordinary"));
    assert_eq!(
        tokens
            .iter()
            .filter(|t| t.kind == TokenKind::DocComment)
            .count(),
        2
    );
}

#[test]
fn edition1_block_comments_nest() {
    let src = "/* outer /* nested */ tail */\nprint 1\n";
    let tokens = tokenize_complete(src).expect("nested block comments must lex");
    let comment = tokens
        .iter()
        .find(|t| t.kind == TokenKind::BlockComment)
        .expect("comment");
    assert!(comment.text.contains("/* nested */"));
}

#[test]
fn unterminated_block_comment_is_an_error() {
    let err = tokenize_complete("/* never closes").expect_err("must reject unterminated comment");
    assert_eq!(err.code.0, "1006");
}

#[test]
fn cst_formatter_preserves_comment_content_with_canonical_markers() {
    let src = "// line\n/* block */\n/// docs\n";
    let tokens = tokenize_complete(src).expect("lex");
    let cst = build_cst(&tokens);
    let formatted = format_cst_source(&cst);
    assert!(formatted.contains("// line"));
    assert!(formatted.contains("/* block */"));
    assert!(formatted.contains("/// docs"));
}
