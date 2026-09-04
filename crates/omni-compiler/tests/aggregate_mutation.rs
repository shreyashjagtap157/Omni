use omni_compiler::compile_to_lir_text;

#[test]
fn test_mutable_field_mutation_lir() {
    let src = r#"
struct Point {
    x: i64,
    y: i64,
}

fn main() -> i64 {
    let mut p = Point { x: 10, y: 20 };
    p.x = 42;
    return p.x + p.y;
}
"#;
    let lir = compile_to_lir_text(src).expect("should compile to LIR");
    assert!(
        lir.contains("StoreOffset"),
        "LIR should contain StoreOffset for local field assign: {}",
        lir
    );
}

#[test]
fn test_immutable_field_mutation_fails() {
    let src = r#"
struct Point {
    x: i64,
    y: i64,
}

fn main() -> i64 {
    let p = Point { x: 10, y: 20 };
    p.x = 42;
    return p.x + p.y;
}
"#;
    let res = compile_to_lir_text(src);
    assert!(res.is_err(), "mutating immutable struct field should fail");
    let err = res.err().unwrap().to_string();
    assert!(
        err.contains("cannot mutate field of immutable binding 'p'"),
        "unexpected error: {}",
        err
    );
}
