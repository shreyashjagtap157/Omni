#[allow(unused_imports)]
use omni_compiler::complete_lexer;

#[test]
fn dump_core_tokens() {
    let core_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("omni")
        .join("stdlib")
        .join("core.omni");
    let src = std::fs::read_to_string(&core_path).expect("read core.omni");
    let tokens = omni_compiler::complete_lexer::tokenize_complete(&src).expect("lex failed");
    for t in tokens.iter() {
        println!("{:?} {}:{} {:?}", t.kind, t.line, t.col, t.text);
    }
    assert!(!tokens.is_empty());
}
