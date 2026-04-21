use omni_compiler::lexer::Lexer;

#[test]
fn dump_core_tokens() {
    let core_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("omni")
        .join("stdlib")
        .join("core.omni");
    let src = std::fs::read_to_string(&core_path).expect("read core.omni");
    let mut lexer = Lexer::new(&src);
    let tokens = lexer.tokenize().expect("lex failed");
    for t in tokens.iter() {
        println!("{:?} {}:{} {:?}", t.kind, t.line, t.col, t.text);
    }
    assert!(true);
}
