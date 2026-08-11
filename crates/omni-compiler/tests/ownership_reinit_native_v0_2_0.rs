use omni_compiler::driver::{Backend, Compiler};

const REINIT_SOURCE: &str = r#"
struct Pair { x: i64; y: i64; }
fn main() -> i64 {
    linear p = Pair { x: 7, y: 35 };
    print p.x;
    p.x = 42;
    return p.x + p.y;
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
fn linear_field_reinitialization_reaches_checked_store_offset() {
    let lir = compile_lir(REINIT_SOURCE);
    let text = omni_compiler::codegen_lir::compile_lir_module_text(&lir);
    assert!(
        text.matches("StoreOffset").count() >= 3,
        "expected aggregate initialization plus reinitialization stores: {text}"
    );
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn linear_field_reinitialization_executes_owned_native() {
    let lir = compile_lir(REINIT_SOURCE);
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("owned native run");
    assert_eq!(native.status, Some(77));
}
