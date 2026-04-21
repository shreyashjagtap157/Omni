use omni_compiler::lexer::Lexer;
use std::fs;

fn main() {
    let core_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("omni")
        .join("stdlib")
        .join("core.omni");
    let core_src = fs::read_to_string(&core_path).expect("read core");
    // Also attempt to parse a combined file (core + sample) to reproduce test parsing
    let sample = r#"
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

    let full = format!("{}\n{}", core_src, sample);
    let mut lexer = Lexer::new(&full);
    match lexer.tokenize() {
        Ok(tokens) => {
            for t in tokens.iter().take(200) {
                println!("{:?} {}:{} {:?}", t.kind, t.line, t.col, t.text);
            }
            println!("... total tokens: {}", tokens.len());
            // try parsing
            let mut parser = omni_compiler::parser::Parser::new(tokens);
            match parser.parse_program() {
                Ok(prog) => println!("parse ok: {} top-level stmts", prog.stmts.len()),
                Err(e) => println!("parse failed: {}", e),
            }
        }
        Err(e) => println!("lex err: {}", e),
    }
}
