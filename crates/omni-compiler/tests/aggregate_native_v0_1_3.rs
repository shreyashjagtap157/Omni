#![allow(dead_code)]
//! Test constants for aggregate native v0.1.3.

use omni_compiler::driver::{Backend, Compiler};

const STRUCT_SOURCE: &str = r#"
struct Point { x: i64; y: i64; }
fn main() -> i64 {
    let p = Point { x: 40, y: 2 };
    return p.x + p.y;
}
"#;

const TUPLE_SOURCE: &str = r#"
fn main() -> i64 {
    let pair = (7, 42);
    return pair[1];
}
"#;

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

#[test]
fn source_struct_reaches_aggregate_lir() {
    let lir = compile_lir(STRUCT_SOURCE);
    let text = omni_compiler::codegen_lir::compile_lir_module_text(&lir);
    assert!(
        text.contains("StoreOffset"),
        "missing struct stores: {text}"
    );
    assert!(text.contains("LoadOffset"), "missing struct loads: {text}");
}

#[test]
fn source_tuple_reaches_aggregate_lir() {
    let lir = compile_lir(TUPLE_SOURCE);
    let text = omni_compiler::codegen_lir::compile_lir_module_text(&lir);
    assert!(text.contains("StoreOffset"), "missing tuple stores: {text}");
    assert!(
        text.contains("LoadOffset"),
        "missing tuple indexed load: {text}"
    );
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn source_struct_executes_owned_native_and_returns_42() {
    let lir = compile_lir(STRUCT_SOURCE);
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("owned native run");
    assert_eq!(native.status, Some(42));
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn source_tuple_executes_owned_native_and_returns_42() {
    let lir = compile_lir(TUPLE_SOURCE);
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("owned native run");
    assert_eq!(native.status, Some(42));
}

const ARRAY_SOURCE: &str = r#"
fn main() -> i64 {
    let values = [10, 20, 42];
    let index = 1 + 1;
    return values[index];
}
"#;

const ARRAY_OOB_SOURCE: &str = r#"
fn main() -> i64 {
    let values = [10, 20, 42];
    let index = 1 + 2;
    return values[index];
}
"#;

#[test]
fn source_array_reaches_bounds_checked_lir() {
    let lir = compile_lir(ARRAY_SOURCE);
    let text = omni_compiler::codegen_lir::compile_lir_module_text(&lir);
    assert!(text.contains("StoreOffset"), "missing array stores: {text}");
    assert!(
        text.contains("LoadIndex"),
        "missing safe dynamic array load: {text}"
    );
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn source_array_dynamic_index_executes_owned_native_and_returns_42() {
    let lir = compile_lir(ARRAY_SOURCE);
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("owned native run");
    assert_eq!(native.status, Some(42));
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn source_array_dynamic_out_of_bounds_faults_before_memory_access() {
    let lir = compile_lir(ARRAY_OOB_SOURCE);
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("owned native run");
    assert_eq!(native.status, Some(codegen_native::BOUNDS_FAULT_EXIT));
}

const SLICE_SOURCE: &str = r#"
fn main() -> i64 {
    let values = [7, 9, 42, 8];
    let window = values[1..3];
    let index = 0 + 1;
    return window[index];
}
"#;

const SLICE_INCLUSIVE_SOURCE: &str = r#"
fn main() -> i64 {
    let values = [7, 9, 42, 8];
    let window = values[1...2];
    return window[1];
}
"#;

const SLICE_OOB_SOURCE: &str = r#"
fn main() -> i64 {
    let values = [7, 9, 42];
    let window = values[1..4];
    return window[0];
}
"#;

#[test]
fn source_constant_slice_view_reaches_safe_index_lir() {
    let lir = compile_lir(SLICE_SOURCE);
    let text = omni_compiler::codegen_lir::compile_lir_module_text(&lir);
    assert!(
        text.contains("LoadIndex"),
        "missing safe slice indexed load: {text}"
    );
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn source_constant_slice_view_executes_owned_native_and_returns_42() {
    let lir = compile_lir(SLICE_SOURCE);
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("owned native run");
    assert_eq!(native.status, Some(42));
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn source_inclusive_slice_view_executes_owned_native_and_returns_42() {
    let lir = compile_lir(SLICE_INCLUSIVE_SOURCE);
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("owned native run");
    assert_eq!(native.status, Some(42));
}

#[test]
fn source_constant_slice_out_of_bounds_is_rejected_before_native_emission() {
    let result = Compiler::new(SLICE_OOB_SOURCE, Backend::Native).compile();
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, omni_compiler::diagnostics::Severity::Error))
        .collect();
    assert!(errors.is_empty(), "unexpected frontend errors: {errors:#?}");
    let err = omni_compiler::codegen_lir::lower_mir_to_lir(result.mir.as_ref().expect("MIR"))
        .expect_err("out-of-bounds constant slice must fail before native emission");
    assert!(
        err.contains("out of bounds"),
        "unexpected diagnostic: {err}"
    );
}

const ENUM_SOURCE: &str = r#"
enum Choice {
    variant Left[value: i64],
    variant Right[value: i64],
}
fn main() -> i64 {
    let choice = Choice::Right(42);
    return match choice {
        Choice::Left[value] => value,
        Choice::Right[value] => value,
    };
}
"#;

const ENUM_FIELDLESS_SOURCE: &str = r#"
enum Flag {
    variant Off,
    variant On,
}
fn main() -> i64 {
    let flag = Flag::On();
    return match flag {
        Flag::Off[] => 7,
        Flag::On[] => 42,
    };
}
"#;

#[test]
fn source_enum_match_reaches_tagged_local_layout_lir() {
    let lir = compile_lir(ENUM_SOURCE);
    let text = omni_compiler::codegen_lir::compile_lir_module_text(&lir);
    assert!(
        text.matches("StoreOffset").count() >= 2,
        "missing enum tag/payload stores: {text}"
    );
    assert!(
        text.matches("LoadOffset").count() >= 2,
        "missing enum tag/payload loads: {text}"
    );
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn source_enum_payload_match_executes_owned_native_and_returns_42() {
    let lir = compile_lir(ENUM_SOURCE);
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("owned native run");
    assert_eq!(native.status, Some(42));
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn source_fieldless_enum_match_executes_owned_native_and_returns_42() {
    let lir = compile_lir(ENUM_FIELDLESS_SOURCE);
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("owned native run");
    assert_eq!(native.status, Some(42));
}

const ENUM_NON_EXHAUSTIVE_SOURCE: &str = r#"
enum Choice {
    variant Left[value: i64],
    variant Right[value: i64],
}
fn main() -> i64 {
    let choice = Choice::Right(42);
    return match choice {
        Choice::Left[value] => value,
    };
}
"#;

const ENUM_WRONG_ARITY_SOURCE: &str = r#"
enum Choice {
    variant Left[value: i64],
    variant Right[value: i64],
}
fn main() -> i64 {
    let choice = Choice::Right();
    return match choice {
        Choice::Left[value] => value,
        Choice::Right[value] => value,
    };
}
"#;

fn frontend_errors(source: &str) -> Vec<String> {
    Compiler::new(source, Backend::Native)
        .compile()
        .diagnostics
        .into_iter()
        .filter(|d| matches!(d.severity, omni_compiler::diagnostics::Severity::Error))
        .map(|d| d.message)
        .collect()
}

#[test]
fn source_non_exhaustive_enum_match_is_rejected_before_mir_execution() {
    let errors = frontend_errors(ENUM_NON_EXHAUSTIVE_SOURCE);
    assert!(
        errors
            .iter()
            .any(|message| message.to_ascii_lowercase().contains("non-exhaustive")),
        "expected non-exhaustive enum diagnostic, got {errors:#?}"
    );
}

#[test]
fn source_enum_constructor_wrong_payload_arity_is_rejected() {
    let errors = frontend_errors(ENUM_WRONG_ARITY_SOURCE);
    assert!(
        errors.iter().any(|message| {
            let lower = message.to_ascii_lowercase();
            lower.contains("argument") || lower.contains(" args")
        }),
        "expected constructor arity diagnostic, got {errors:#?}"
    );
}

const AGGREGATE_ARGUMENT_SOURCE: &str = r#"
struct Point { x: i64; y: i64; }
fn consume(point: Point) -> i64 { return 42; }
fn main() -> i64 {
    let point = Point { x: 40, y: 2 };
    return consume(point);
}
"#;

const AGGREGATE_MUTATION_SOURCE: &str = r#"
struct Point { x: i64; y: i64; }
fn main() -> i64 {
    let point = Point { x: 40, y: 2 };
    point.x = 42;
    return point.x;
}
"#;

#[test]
fn historical_aggregate_argument_case_advances_to_v0_1_4_bounded_indirect_abi() {
    let result = Compiler::new(AGGREGATE_ARGUMENT_SOURCE, Backend::Native).compile();
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, omni_compiler::diagnostics::Severity::Error))
        .collect();
    assert!(errors.is_empty(), "unexpected frontend errors: {errors:#?}");
    let lir = omni_compiler::codegen_lir::lower_mir_to_lir(result.mir.as_ref().expect("MIR"))
        .expect("v0.1.4 aggregate argument ABI");
    let text = omni_compiler::codegen_lir::compile_lir_module_text(&lir);
    assert!(
        text.contains("Ptr(2)"),
        "missing bounded aggregate parameter ABI: {text}"
    );
    assert!(
        text.contains("GetAddr"),
        "caller did not pass owned aggregate storage: {text}"
    );
}

#[test]
fn ownership_sensitive_aggregate_mutation_remains_fail_closed() {
    let errors = frontend_errors(AGGREGATE_MUTATION_SOURCE);
    assert!(
        !errors.is_empty(),
        "field mutation syntax must remain unavailable until ownership semantics are qualified"
    );
}
