use omni_compiler::interpreter;
use omni_compiler::resolver;
use omni_compiler::type_checker;

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

    let tokens = omni_compiler::complete_lexer::tokenize_complete(&full_src).unwrap();
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

    let tokens = omni_compiler::complete_lexer::tokenize_complete(&full_src).unwrap();
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

    let tokens = omni_compiler::complete_lexer::tokenize_complete(&full_src).unwrap();
    let mut parser = omni_compiler::parser::Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");

    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
    interpreter::run_program(&prog).expect("interpreter error");
}

#[test]
fn option_combinators() {
    let src = r#"
fn double(x) -> int
    return x * 2

fn some_pred(x) -> bool
    return x > 0

fn none_pred(_x) -> bool
    return 0 == 1

fn forty_two() -> int
    return 42

// Build Some(5)
let s = hashmap_new()
let _ = hashmap_insert(s, "value", 5)

// Build None
let n = hashmap_new()

// option_map on Some
let mapped = option_map(s, "double")
// option_flat_map on Some
let flat_mapped = option_flat_map(s, "double")
// option_or_else on Some (should keep the same Some)
let orelse = option_or_else(s, "double")
// option_filter with true predicate
let filtered_true = option_filter(s, "some_pred")
// option_filter with false predicate
let filtered_false = option_filter(s, "none_pred")
// option_zip: two Somes
let zipped = option_zip(s, mapped)

// option_map on None (should stay None)
let mapped_none = option_map(n, "double")
// option_or_else on None (should call forty_two which returns 42)
let orelse_none = option_or_else(n, "forty_two")
// option_flat_map on None (should stay None)
let flat_mapped_none = option_flat_map(n, "double")

print mapped
print flat_mapped
print orelse
print filtered_true
print filtered_false
print zipped
print mapped_none
print orelse_none
print flat_mapped_none
"#;

    let full_src = format!("{}\n{}", load_bootstrap_stdlib(), src);
    let tokens = omni_compiler::complete_lexer::tokenize_complete(&full_src).unwrap();
    let mut parser = omni_compiler::parser::Parser::new(tokens);
    let prog = parser.parse_program().expect("parse failed");
    resolver::resolve_program(&prog).expect("resolve failed");
    type_checker::type_check_program(&prog).expect("typecheck failed");
    interpreter::run_program(&prog).expect("interpreter error");
}

#[test]
fn result_combinators() {
    let src = r#"
fn double(x) -> int
    return x * 2

fn fallback(x) -> int
    return x + 100

// Create Ok(5)
let ok_val = hashmap_new()
let _ = hashmap_insert(ok_val, "value", 5)

// Create Err(-1)
let err_val = hashmap_new()
let _ = hashmap_insert(err_val, "err", -1)

// result_map on Ok
let mapped = result_map(ok_val, "double")
// result_map_err on Ok (no-op, should stay Ok(5))
let mapped_err_ok = result_map_err(ok_val, "double")
// result_map_err on Err (transforms the error)
let mapped_err = result_map_err(err_val, "double")
// result_flat_map on Ok: Ok(5) -> calls double(5) -> expects Ok value
let flat_mapped = result_flat_map(ok_val, "double")
// result_or_else on Ok (should keep Ok(5))
let orelse_ok = result_or_else(ok_val, "fallback")
// result_or_else on Err (should call fallback with -1 -> 99)
let orelse_err = result_or_else(err_val, "fallback")
// result_map on Err (should stay Err)
let mapped_err_same = result_map(err_val, "double")

print mapped
print mapped_err_ok
print mapped_err
print flat_mapped
print orelse_ok
print orelse_err
print mapped_err_same
"#;

    let full_src = format!("{}\n{}", load_bootstrap_stdlib(), src);
    let tokens = omni_compiler::complete_lexer::tokenize_complete(&full_src).unwrap();
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
