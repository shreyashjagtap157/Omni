use omni_compiler::driver::{Backend, Compiler};

fn compile_lir(source: &str) -> lir::Module {
    let result = Compiler::new(source, Backend::Native).compile();
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, omni_compiler::diagnostics::Severity::Error))
        .collect();
    assert!(errors.is_empty(), "compiler errors: {errors:#?}");
    omni_compiler::codegen_lir::lower_mir_to_lir(result.mir.as_ref().expect("MIR"))
        .expect("LIR lowering")
}

const STRUCT_ARG: &str = r#"
struct Point { x: i64; y: i64; }
fn sum(point: Point) -> i64 {
    return point.x + point.y;
}
fn main() -> i64 {
    let point = Point { x: 40, y: 2 };
    return sum(point);
}
"#;

const STRUCT_RETURN: &str = r#"
struct Point { x: i64; y: i64; }
fn make_point() -> Point {
    return Point { x: 40, y: 2 };
}
fn main() -> i64 {
    let point = make_point();
    return point.x + point.y;
}
"#;

const STRUCT_FORWARD: &str = r#"
struct Point { x: i64; y: i64; }
fn sum(point: Point) -> i64 { return point.x + point.y; }
fn forward(point: Point) -> i64 { return sum(point); }
fn main() -> i64 {
    let point = Point { x: 40, y: 2 };
    return forward(point);
}
"#;

#[test]
fn aggregate_argument_uses_bounded_indirect_lir() {
    let lir = compile_lir(STRUCT_ARG);
    let text = omni_compiler::codegen_lir::compile_lir_module_text(&lir);
    assert!(
        text.contains("Ptr(2)"),
        "missing bounded pointer ABI: {text}"
    );
    assert!(
        text.contains("LoadPtrOffset"),
        "callee is not reading through indirect ABI: {text}"
    );
    assert!(
        text.contains("GetAddr"),
        "caller is not passing owned local address: {text}"
    );
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn aggregate_argument_executes_native_and_returns_42() {
    let lir = compile_lir(STRUCT_ARG);
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("owned native run");
    assert_eq!(native.status, Some(42));
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn aggregate_return_uses_caller_storage_and_returns_42() {
    let lir = compile_lir(STRUCT_RETURN);
    let text = omni_compiler::codegen_lir::compile_lir_module_text(&lir);
    assert!(
        text.contains("StorePtrOffset"),
        "callee did not populate caller storage: {text}"
    );
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("owned native run");
    assert_eq!(native.status, Some(42));
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn aggregate_parameter_can_be_forwarded_without_frame_escape() {
    let lir = compile_lir(STRUCT_FORWARD);
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("owned native run");
    assert_eq!(native.status, Some(42));
}

const STRING_ROUNDTRIP: &str = r#"
fn echo(value: String) -> String { return value; }
fn main() -> i64 {
    let value = echo("123456789012345678901234567890123456789012");
    return value.len;
}
"#;

const STRING_PRINT_PARAM: &str = r#"
fn show(value: String) -> i64 {
    print value;
    return value.len;
}
fn main() -> i64 {
    return show("hello");
}
"#;

#[test]
fn string_call_abi_uses_two_cell_descriptor() {
    let lir = compile_lir(STRING_ROUNDTRIP);
    let text = omni_compiler::codegen_lir::compile_lir_module_text(&lir);
    assert!(
        text.contains("StringRef"),
        "missing literal data reference: {text}"
    );
    assert!(
        text.contains("Ptr(2)"),
        "missing two-cell String ABI: {text}"
    );
    assert!(
        text.contains("StorePtrOffset"),
        "missing String return descriptor copy: {text}"
    );
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn string_roundtrip_across_call_returns_utf8_byte_length_42() {
    let lir = compile_lir(STRING_ROUNDTRIP);
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("owned native run");
    assert_eq!(native.status, Some(42));
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn string_parameter_is_printable_as_runtime_value() {
    let lir = compile_lir(STRING_PRINT_PARAM);
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("owned native run");
    assert_eq!(native.status, Some(5));
    assert_eq!(native.stdout, b"hello\n".to_vec());
}

const BYTES_ROUNDTRIP: &str = r#"
fn echo(value: bytes) -> bytes { return value; }
fn main() -> i64 {
    let value = echo(b"A\xFFZ");
    print value;
    return value.len;
}
"#;

const BYTE_SCALAR: &str = r#"
fn identity(value: byte) -> byte { return value; }
fn main() -> i64 {
    print identity(b'*');
    return 42;
}
"#;

#[test]
fn bytes_call_abi_uses_binary_safe_descriptor() {
    let lir = compile_lir(BYTES_ROUNDTRIP);
    let text = omni_compiler::codegen_lir::compile_lir_module_text(&lir);
    assert!(
        text.contains("BytesRef"),
        "missing binary literal reference: {text}"
    );
    assert!(
        text.contains("Ptr(2)"),
        "missing two-cell Bytes ABI: {text}"
    );
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn arbitrary_bytes_roundtrip_and_print_without_utf8_conversion() {
    let lir = compile_lir(BYTES_ROUNDTRIP);
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("owned native run");
    assert_eq!(native.status, Some(3));
    assert_eq!(native.stdout, vec![b'A', 0xff, b'Z', b'\n']);
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn byte_scalar_is_range_validated_by_literal_and_crosses_call_abi() {
    let lir = compile_lir(BYTE_SCALAR);
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("owned native run");
    assert_eq!(native.status, Some(42));
    assert_eq!(native.stdout, b"42\n".to_vec());
}

#[test]
fn byte_literal_formatter_is_canonical_and_binary_safe() {
    let source =
        "fn main() -> i64 { let x: byte = b'\\xFF'; let y = b\"A\\x00\\xFF\"; return 42; }";
    let tokens = omni_compiler::complete_lexer::tokenize_complete(source).expect("lex");
    let mut parser = omni_compiler::parser::Parser::new(tokens);
    let program = parser.parse_program().expect("parse");
    let formatted = omni_compiler::formatter::format_program(&program);
    assert!(formatted.contains("b'\\xFF'"), "{formatted}");
    assert!(formatted.contains("b\"A\\0\\xFF\""), "{formatted}");
    let tokens = omni_compiler::complete_lexer::tokenize_complete(&formatted).expect("relex");
    let mut parser = omni_compiler::parser::Parser::new(tokens);
    let reformatted =
        omni_compiler::formatter::format_program(&parser.parse_program().expect("reparse"));
    assert_eq!(formatted, reformatted);
}

const BYTES_INDEX: &str = r#"
fn main() -> i64 {
    let value = b"A\x00\xFF";
    let selected = value[1 + 1];
    print selected;
    return 42;
}
"#;

const BYTES_OOB: &str = r#"
fn main() -> i64 {
    let value = b"ABC";
    let selected = value[1 + 2];
    print selected;
    return 42;
}
"#;

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn bytes_dynamic_index_is_bounds_checked_and_binary_safe() {
    let lir = compile_lir(BYTES_INDEX);
    let text = omni_compiler::codegen_lir::compile_lir_module_text(&lir);
    assert!(
        text.contains("LoadByteIndex"),
        "missing byte-index LIR: {text}"
    );
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("owned native run");
    assert_eq!(native.status, Some(42));
    assert_eq!(native.stdout, b"255\n".to_vec());
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn bytes_dynamic_index_oob_uses_bounds_fault() {
    let lir = compile_lir(BYTES_OOB);
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("owned native run");
    assert_eq!(native.status, Some(codegen_native::BOUNDS_FAULT_EXIT));
    assert!(native.stdout.is_empty());
}

#[test]
fn string_indexing_fails_closed_instead_of_treating_utf8_as_bytes() {
    let result = Compiler::new(
        r#"fn main() -> i64 { let s = "é"; let x = s[0]; return 42; }"#,
        Backend::Native,
    )
    .compile();
    assert!(
        result.diagnostics.iter().any(|d| {
            matches!(d.severity, omni_compiler::diagnostics::Severity::Error)
                && d.message.contains("UTF-8")
        }),
        "expected UTF-8 indexing diagnostic: {:#?}",
        result.diagnostics
    );
}

const ENUM_ROUNDTRIP: &str = r#"
enum Choice {
    variant Left[value: i64],
    variant Right[value: i64],
}
fn echo(value: Choice) -> Choice { return value; }
fn main() -> i64 {
    let choice = echo(Choice::Right(42));
    return match choice {
        Choice::Left[value] => value,
        Choice::Right[value] => value,
    };
}
"#;

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn enum_payload_value_crosses_indirect_call_and_return_abi() {
    let lir = compile_lir(ENUM_ROUNDTRIP);
    let text = omni_compiler::codegen_lir::compile_lir_module_text(&lir);
    assert!(
        text.contains("Ptr(2)"),
        "missing enum tag/payload ABI: {text}"
    );
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("owned native run");
    assert_eq!(native.status, Some(42));
}
