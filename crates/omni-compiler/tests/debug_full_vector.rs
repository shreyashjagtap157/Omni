
#[test]
fn dump_full_vector_tokens() {
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

    let core_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("omni")
        .join("stdlib")
        .join("core.omni");
    let core_src = std::fs::read_to_string(&core_path).unwrap_or_else(|_| "".to_string());
    let full_src = format!("{}\n{}", core_src, src);

    let mut lexer = omni_compiler::lexer::Lexer::new(&full_src);
    let tokens = lexer.tokenize().expect("lex failed");
    for t in tokens.iter() {
        println!("{:?} {}:{} \"{}\"", t.kind, t.line, t.col, t.text);
    }
}
