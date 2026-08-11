use omni_compiler::driver::{Backend, Compiler};

const SOURCE: &str = r#"
fn main() -> i64 {
    let x: i64 = 0;
    while x < 6 {
        x = x + 1;
        if x == 2 {
            continue;
        }
        if x == 4 {
            break;
        }
    }
    print x;
    return x;
}
"#;

#[test]
fn break_and_continue_lower_without_unsupported_sentinels() {
    let result = Compiler::new(SOURCE, Backend::Native).compile();
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, omni_compiler::diagnostics::Severity::Error))
        .collect();
    assert!(errors.is_empty(), "compiler errors: {errors:#?}");
    let mir = result.mir.expect("MIR");
    let lir = omni_compiler::codegen_lir::lower_mir_to_lir(&mir).expect("LIR lowering");
    let debug = format!("{lir:#?}");
    assert!(!debug.contains("__omni_unsupported_break"));
    assert!(!debug.contains("__omni_unsupported_continue"));
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn break_and_continue_execute_through_owned_native_backend() {
    let result = Compiler::new(SOURCE, Backend::Native).compile();
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, omni_compiler::diagnostics::Severity::Error))
        .collect();
    assert!(errors.is_empty(), "compiler errors: {errors:#?}");
    let lir = omni_compiler::codegen_lir::lower_mir_to_lir(result.mir.as_ref().expect("MIR"))
        .expect("LIR lowering");
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("owned native run");
    assert_eq!(native.stdout, b"4\n");
    assert_eq!(native.status, Some(4));
}
