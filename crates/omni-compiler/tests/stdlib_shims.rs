use omni_compiler::resolver;
use omni_compiler::type_checker;
use omni_compiler::interpreter;

fn load_bootstrap_stdlib() -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let core_path = root.join("omni").join("stdlib").join("core.omni");
    let collections_path = root.join("omni").join("stdlib").join("collections.omni");

    let mut src = String::new();
    if let Ok(core_src) = std::fs::read_to_string(&core_path) {
        src.push_str(&core_src);
        src.push('\n');
    }
    if let Ok(collections_src) = std::fs::read_to_string(&collections_path) {
        src.push_str(&collections_src);
        src.push('\n');
    }
    src
}

#[test]
fn result_map_and_option_map_smoke() {
    // Use textual parsing so stdlib stubs are included automatically.
    let src = r#"
fn inc(x) -> int
    return x + 1

let m = hashmap_new()
let _ = hashmap_insert(m, "value", 41)
let r = result_map(m, "inc")
let v = hashmap_get(r, "value")
print v
"#;

    // include the bootstrap stdlib surface to ensure resolver sees builtin declarations
    let full_src = format!("{}\n{}", load_bootstrap_stdlib(), src);
    let mut lexer = omni_compiler::lexer::Lexer::new(&full_src);
    let tokens = lexer.tokenize().expect("lex failed");
    let mut parser = omni_compiler::parser::Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    match resolver::resolve_program(&prog) {
        Ok(_) => {}
        Err(es) => panic!("resolver errors: {:?}", es),
    }
    if let Err(e) = type_checker::type_check_program(&prog) { panic!("typecheck error: {}", e); }
    if let Err(e) = interpreter::run_program(&prog) { panic!("interpreter error: {}", e); }
}

#[test]
fn option_map_and_and_smoke() {
    let src = r#"
fn is_zero(x) -> bool
    return x == 0

let o = hashmap_new()
let _ = hashmap_insert(o, "value", 0)
let r = option_map(o, "is_zero")
let a = option_and(o, r)
print a
"#;

    let full_src = format!("{}\n{}", load_bootstrap_stdlib(), src);
    let mut lexer = omni_compiler::lexer::Lexer::new(&full_src);
    let tokens = lexer.tokenize().expect("lex failed");
    let mut parser = omni_compiler::parser::Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    match resolver::resolve_program(&prog) {
        Ok(_) => {}
        Err(es) => panic!("resolver errors: {:?}", es),
    }
    if let Err(e) = type_checker::type_check_program(&prog) { panic!("typecheck error: {}", e); }
    if let Err(e) = interpreter::run_program(&prog) { panic!("interpreter error: {}", e); }
}
