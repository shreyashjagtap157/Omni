use omni_compiler;
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
fn vector_hashmap_string_smoke() {
    let src = r#"
fn inc(x) -> int
    return x + 1

let v = vector_new()
let _ = vector_push(v, 10)
let _ = vector_push(v, 20)
let len = vector_len(v)
let e0 = vector_get(v, 0)
let e1 = vector_pop(v)
let m = hashmap_new()
let _ = hashmap_insert(m, "key", 123)
let ok = hashmap_contains(m, "key")
let s = string_concat("ab", "cd")
let l = str_len(s)
print len
print e0
print e1
print ok
print l
"#;

    // prefix stdlib
    let full_src = format!("{}\n{}", load_bootstrap_stdlib(), src);

    let mut lexer = omni_compiler::lexer::Lexer::new(&full_src);
    let tokens = lexer.tokenize().expect("lex failed");
    let mut parser = omni_compiler::parser::Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
    interpreter::run_program(&prog).expect("interpreter error");
}

#[test]
fn hashset_smoke() {
    let src = r#"
let s = hashset_new()
let _ = hashset_insert(s, 1)
let contains1 = hashset_contains(s, 1)
let len = hashset_len(s)
print contains1
print len
let _ = hashset_clear(s)
print hashset_len(s)
"#;

    let full_src = format!("{}\n{}", load_bootstrap_stdlib(), src);

    let mut lexer = omni_compiler::lexer::Lexer::new(&full_src);
    let tokens = lexer.tokenize().expect("lex failed");
    let mut parser = omni_compiler::parser::Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
    interpreter::run_program(&prog).expect("interpreter error");
}

#[test]
fn option_result_smoke() {
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

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
    interpreter::run_program(&prog).expect("interpreter error");
}

#[test]
fn lir_smoke() {
    let src = r#"
fn inc(x) -> int
    return x + 1
"#;

    let combined_stdlib = load_bootstrap_stdlib();

    // write combined file and call emit_lir_file to exercise MIR->LIR pipeline
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    use std::io::Write;
    write!(tmp, "{}\n{}", combined_stdlib, src).unwrap();
    let path = tmp.path();

    let lir = omni_compiler::emit_lir_file(path).expect("emit LIR failed");
    assert!(!lir.is_empty());
}
