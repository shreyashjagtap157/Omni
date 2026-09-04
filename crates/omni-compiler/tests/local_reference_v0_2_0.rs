use omni_compiler::diagnostics::Severity;
use omni_compiler::driver::{Backend, Compiler};

fn compile(source: &str) -> omni_compiler::driver::CompilationResult {
    Compiler::new(source, Backend::Native).compile()
}

fn error_text(source: &str) -> String {
    let result = compile(source);
    result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .map(|d| d.message.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

const SHARED_LOCAL: &str = r#"
fn main() -> i64 {
    let x = 42;
    let r = &x;
    return *r;
}
"#;

#[test]
fn shared_local_reference_lowers_to_proven_alias() {
    let result = compile(SHARED_LOCAL);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .collect();
    assert!(
        !errors.is_empty(),
        "expected provenance error but got none: compiler errors: {errors:#?}"
    );
    let mir = result.mir.as_ref().expect("MIR");
    let text = omni_compiler::mir::format_mir(mir);
    assert!(text.contains("borrow &x"), "{text}");
    assert!(text.contains("deref"), "{text}");
    let lir = omni_compiler::codegen_lir::lower_mir_to_lir(mir).expect("LIR");
    let lir_text = omni_compiler::codegen_lir::compile_lir_module_text(&lir);
    assert!(
        !lir_text.contains("LoadInd"),
        "safe local reference became raw indirect load: {lir_text}"
    );
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn shared_local_reference_executes_owned_native() {
    let result = compile(SHARED_LOCAL);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .collect();
    assert!(errors.is_empty(), "compiler errors: {errors:#?}");
    let lir = omni_compiler::codegen_lir::lower_mir_to_lir(result.mir.as_ref().unwrap()).unwrap();
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("native run");
    assert_eq!(native.status, Some(42));
}

#[test]
fn mutable_reference_requires_mutable_binding() {
    let errors = error_text(
        r#"
fn main() -> i64 {
    let x = 1;
    let r = &mut x;
    return *r;
}
"#,
    );
    assert!(
        errors.contains("mutably borrow immutable binding"),
        "{errors}"
    );
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn mutable_local_reference_can_read_owned_native() {
    let source = r#"
fn main() -> i64 {
    let mut x = 42;
    let r = &mut x;
    return *r;
}
"#;
    let result = compile(source);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .collect();
    assert!(errors.is_empty(), "compiler errors: {errors:#?}");
    let lir = omni_compiler::codegen_lir::lower_mir_to_lir(result.mir.as_ref().unwrap()).unwrap();
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("native run");
    assert_eq!(native.status, Some(42));
}

#[test]
fn shared_then_mutable_overlapping_borrow_is_rejected() {
    let errors = error_text(
        r#"
fn main() -> i64 {
    let mut x = 42;
    let a = &x;
    let b = &mut x;
    print *a;
    return *b;
}
"#,
    );
    assert!(errors.contains("conflicting mutable borrow"), "{errors}");
}

#[test]
fn mutable_then_shared_overlapping_borrow_is_rejected() {
    let errors = error_text(
        r#"
fn main() -> i64 {
    let mut x = 42;
    let a = &mut x;
    let b = &x;
    print *a;
    return *b;
}
"#,
    );
    assert!(errors.contains("conflicting shared borrow"), "{errors}");
}

#[test]
fn direct_owner_use_while_mutable_borrow_live_is_rejected() {
    let errors = error_text(
        r#"
fn main() -> i64 {
    let mut x = 42;
    let r = &mut x;
    print x;
    return *r;
}
"#,
    );
    assert!(
        errors.contains("mutable reference") || errors.contains("used or modified"),
        "{errors}"
    );
}

#[test]
fn local_reference_escape_is_rejected() {
    let errors = error_text(
        r#"
fn leak() -> &i64 {
    let x = 1;
    return &x;
}
fn main() -> i64 { return 0; }
"#,
    );
    assert!(
        errors.contains("cannot escape")
            || errors.contains("reference")
            || errors.contains("qualified"),
        "{errors}"
    );
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn shared_reference_parameter_executes_owned_native() {
    let source = r#"
fn read(r: &i64) -> i64 { return *r; }
fn main() -> i64 {
    let x = 42;
    let r = &x;
    return read(r);
}
"#;
    let result = compile(source);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .collect();
    assert!(errors.is_empty(), "compiler errors: {errors:#?}");
    let lir = omni_compiler::codegen_lir::lower_mir_to_lir(result.mir.as_ref().unwrap()).unwrap();
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("native run");
    assert_eq!(native.status, Some(42));
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn mutable_reference_parameter_writes_back_owned_native() {
    let source = r#"
fn set(r: &mut i64) { *r = 42; }
fn main() -> i64 {
    let mut x = 1;
    let r = &mut x;
    set(r);
    return x;
}
"#;
    let result = compile(source);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .collect();
    assert!(errors.is_empty(), "compiler errors: {errors:#?}");
    let lir = omni_compiler::codegen_lir::lower_mir_to_lir(result.mir.as_ref().unwrap()).unwrap();
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("native run");
    assert_eq!(native.status, Some(42));
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn mutable_reference_can_weaken_to_shared_parameter() {
    let source = r#"
fn read(r: &i64) -> i64 { return *r; }
fn main() -> i64 {
    let mut x = 42;
    let r = &mut x;
    return read(r);
}
"#;
    let result = compile(source);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .collect();
    assert!(errors.is_empty(), "compiler errors: {errors:#?}");
}

#[test]
fn shared_reference_cannot_strengthen_to_mutable_parameter() {
    let errors = error_text(
        r#"
fn set(r: &mut i64) { *r = 42; }
fn main() -> i64 {
    let x = 1;
    let r = &x;
    set(r);
    return x;
}
"#,
    );
    assert!(
        errors.contains("expected type") || errors.contains("mutable reference"),
        "{errors}"
    );
}

#[test]
fn reference_return_remains_fail_closed_without_outlives_proof() {
    let errors = error_text(
        r#"
fn identity(r: &i64) -> &i64 { return r; }
fn main() -> i64 { return 0; }
"#,
    );
    assert!(
        errors.contains("cannot escape")
            || errors.contains("outlives")
            || errors.contains("reference"),
        "{errors}"
    );
}

#[test]
fn consuming_linear_owner_while_shared_loan_is_live_is_rejected() {
    let errors = error_text(
        r#"
fn main() -> i64 {
    linear x = 42;
    let r = &x;
    print x;
    return *r;
}
"#,
    );
    assert!(
        errors.contains("shared reference") || errors.contains("borrow") || errors.contains("live"),
        "{errors}"
    );
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn mutable_reference_write_updates_owner_owned_native() {
    let source = r#"
fn main() -> i64 {
    let mut x = 1;
    let r = &mut x;
    *r = 42;
    return x;
}
"#;
    let result = compile(source);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .collect();
    assert!(errors.is_empty(), "compiler errors: {errors:#?}");
    let lir = omni_compiler::codegen_lir::lower_mir_to_lir(result.mir.as_ref().unwrap()).unwrap();
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("native run");
    assert_eq!(native.status, Some(42));
}

#[test]
fn shared_reference_write_is_rejected() {
    let errors = error_text(
        r#"
fn main() -> i64 {
    let x = 1;
    let r = &x;
    *r = 42;
    return x;
}
"#,
    );
    assert!(errors.contains("mutable reference"), "{errors}");
}

#[test]
fn mutable_reference_write_type_mismatch_is_rejected() {
    let errors = error_text(
        r#"
fn main() -> i64 {
    let mut x = 1;
    let r = &mut x;
    *r = true;
    return x;
}
"#,
    );
    assert!(
        errors.contains("type mismatch") || errors.contains("expected"),
        "{errors}"
    );
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn owner_is_available_after_mutable_loan_last_use() {
    let source = r#"
fn main() -> i64 {
    let mut x = 1;
    let r = &mut x;
    *r = 42;
    return x;
}
"#;
    let result = compile(source);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .collect();
    assert!(errors.is_empty(), "compiler errors: {errors:#?}");
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn shared_reborrow_from_mutable_parent_executes_native() {
    let source = r#"
fn main() -> i64 {
    let mut x = 42;
    let parent = &mut x;
    let child = &*parent;
    return *child;
}
"#;
    let result = compile(source);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .collect();
    assert!(errors.is_empty(), "compiler errors: {errors:#?}");
    let lir = omni_compiler::codegen_lir::lower_mir_to_lir(result.mir.as_ref().unwrap()).unwrap();
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("native run");
    assert_eq!(native.status, Some(42));
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn mutable_reborrow_updates_original_owner_native() {
    let source = r#"
fn main() -> i64 {
    let mut x = 1;
    let parent = &mut x;
    let child = &mut *parent;
    *child = 42;
    return x;
}
"#;
    let result = compile(source);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .collect();
    assert!(errors.is_empty(), "compiler errors: {errors:#?}");
    let lir = omni_compiler::codegen_lir::lower_mir_to_lir(result.mir.as_ref().unwrap()).unwrap();
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("native run");
    assert_eq!(native.status, Some(42));
}

#[test]
fn mutable_reborrow_from_shared_parent_is_rejected() {
    let errors = error_text(
        r#"
fn main() -> i64 {
    let mut x = 1;
    let parent = &x;
    let child = &mut *parent;
    return *child;
}
"#,
    );
    assert!(
        errors.contains("mutable reborrow") || errors.contains("shared reference"),
        "{errors}"
    );
}

#[test]
fn parent_is_suspended_while_child_reborrow_is_live() {
    let errors = error_text(
        r#"
fn main() -> i64 {
    let mut x = 1;
    let parent = &mut x;
    let child = &*parent;
    print *parent;
    return *child;
}
"#,
    );
    assert!(
        errors.contains("suspended") || errors.contains("reborrow"),
        "{errors}"
    );
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn parent_is_restored_after_child_last_use() {
    let source = r#"
fn main() -> i64 {
    let mut x = 1;
    let parent = &mut x;
    let child = &*parent;
    print *child;
    *parent = 42;
    return x;
}
"#;
    let result = compile(source);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .collect();
    assert!(errors.is_empty(), "compiler errors: {errors:#?}");
    let lir = omni_compiler::codegen_lir::lower_mir_to_lir(result.mir.as_ref().unwrap()).unwrap();
    let native = omni_compiler::codegen::compile_and_run_aot(&lir).expect("native run");
    assert_eq!(native.status, Some(42));
}

#[test]
fn multi_block_reference_join_succeeds() {
    let source = r#"
fn choose(cond: bool, a: &i64, b: &i64) -> i64 {
    if cond {
        return *a;
    } else {
        return *b;
    }
}

fn main() -> i64 {
    let x = 10;
    let y = 20;
    let rx = &x;
    let ry = &y;
    let res = choose(true, rx, ry);
    return res;
}
"#;
    let result = compile(source);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .collect();
    assert!(errors.is_empty(), "compiler errors: {errors:#?}");
}
