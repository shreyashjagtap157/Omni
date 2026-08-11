use codegen_native::{compile_and_run_native, ARITHMETIC_FAULT_EXIT};
use lir::{Function, Instr, Module, Type};

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn native_exit_42() {
    let result = compile_and_run_native(&lir::example_module()).expect("native compile/run");
    assert_eq!(result.status, Some(42));
    assert!(result.stdout.is_empty());
    assert!(result.stderr.is_empty());
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn native_print_string_is_not_a_vm() {
    let mut module = Module::new();
    module.add_function(Function::new(
        "main",
        vec![],
        Type::I64,
        vec![
            Instr::PrintStr("hello native".into()),
            Instr::Const(0),
            Instr::Ret,
        ],
        vec![],
    ));
    let result = compile_and_run_native(&module).expect("native compile/run");
    assert_eq!(result.status, Some(0));
    assert_eq!(result.stdout, b"hello native\n");
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn native_user_function_call_uses_real_parameters() {
    let mut module = Module::new();
    module.add_function(Function::new(
        "add_two",
        vec![Type::I64, Type::I64],
        Type::I64,
        vec![
            Instr::Store(1),
            Instr::Store(0),
            Instr::Load(0),
            Instr::Load(1),
            Instr::Add,
            Instr::Ret,
        ],
        vec![],
    ));
    module.add_function(Function::new(
        "main",
        vec![],
        Type::I64,
        vec![
            Instr::Const(40),
            Instr::Const(2),
            Instr::Call("add_two".into()),
            Instr::Ret,
        ],
        vec![],
    ));
    let result = compile_and_run_native(&module).expect("native compile/run");
    assert_eq!(result.status, Some(42));
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn native_checked_overflow_has_defined_fault_status() {
    let mut module = Module::new();
    module.add_function(Function::new(
        "main",
        vec![],
        Type::I64,
        vec![
            Instr::Const(i64::MAX),
            Instr::Const(1),
            Instr::Add,
            Instr::Ret,
        ],
        vec![],
    ));
    let result = compile_and_run_native(&module).expect("native compile/run");
    assert_eq!(result.status, Some(ARITHMETIC_FAULT_EXIT));
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn native_rejects_lir_stack_underflow_before_emission() {
    let mut module = Module::new();
    module.add_function(Function::new(
        "main",
        vec![],
        Type::I64,
        vec![Instr::Add, Instr::Ret],
        vec![],
    ));
    let mut path = std::env::temp_dir();
    path.push(format!("omni-invalid-stack-{}", std::process::id()));
    let err = codegen_native::compile_to_target(
        &module,
        &path,
        codegen_native::NativeTarget::X86_64Linux,
    )
    .expect_err("malformed LIR must be rejected");
    assert!(
        err.contains("stack underflow"),
        "unexpected diagnostic: {err}"
    );
    assert!(!path.exists(), "invalid artifact must not be written");
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn native_rejects_duplicate_function_names() {
    let mut module = Module::new();
    for _ in 0..2 {
        module.add_function(Function::new(
            "main",
            vec![],
            Type::I64,
            vec![Instr::Const(0), Instr::Ret],
            vec![],
        ));
    }
    let mut path = std::env::temp_dir();
    path.push(format!("omni-duplicate-fn-{}", std::process::id()));
    let err = codegen_native::compile_to_target(
        &module,
        &path,
        codegen_native::NativeTarget::X86_64Linux,
    )
    .expect_err("duplicate functions must be rejected");
    assert!(
        err.contains("duplicate LIR function"),
        "unexpected diagnostic: {err}"
    );
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn native_nested_call_preserves_outer_evaluation_value() {
    let mut module = Module::new();
    module.add_function(Function::new(
        "id",
        vec![Type::I64],
        Type::I64,
        vec![Instr::Store(0), Instr::Load(0), Instr::Ret],
        vec![],
    ));
    module.add_function(Function::new(
        "main",
        vec![],
        Type::I64,
        vec![
            Instr::Const(40),
            Instr::Const(2),
            Instr::Call("id".into()),
            Instr::Add,
            Instr::Ret,
        ],
        vec![],
    ));
    let result = compile_and_run_native(&module).expect("native compile/run");
    assert_eq!(result.status, Some(42));
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn native_aggregate_local_offsets_round_trip() {
    let mut module = Module::new();
    module.add_function(Function::new(
        "main",
        vec![],
        Type::I64,
        vec![
            Instr::Const(40),
            Instr::StoreOffset(1, 0),
            Instr::Const(2),
            Instr::StoreOffset(1, 8),
            Instr::LoadOffset(1, 0),
            Instr::LoadOffset(1, 8),
            Instr::Add,
            Instr::Ret,
        ],
        vec![],
    ));
    let result = compile_and_run_native(&module).expect("native aggregate offset compile/run");
    assert_eq!(result.status, Some(42));
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn native_rejects_aggregate_offset_escape() {
    let mut module = Module::new();
    module.add_function(Function::new(
        "main",
        vec![],
        Type::I64,
        vec![Instr::LoadOffset(0, 8), Instr::Ret],
        vec![],
    ));
    let mut path = std::env::temp_dir();
    path.push(format!(
        "omni-invalid-aggregate-offset-{}",
        std::process::id()
    ));
    let err = codegen_native::compile_to_target(
        &module,
        &path,
        codegen_native::NativeTarget::X86_64Linux,
    )
    .expect_err("out-of-frame aggregate offset must be rejected");
    assert!(
        err.contains("escapes local frame"),
        "unexpected diagnostic: {err}"
    );
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn native_bounds_check_preserves_valid_index() {
    let mut module = Module::new();
    module.add_function(Function::new(
        "main",
        vec![],
        Type::I64,
        vec![Instr::Const(1), Instr::BoundsCheck(2), Instr::Ret],
        vec![],
    ));
    let result = compile_and_run_native(&module).expect("native bounds-check compile/run");
    assert_eq!(result.status, Some(1));
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn native_bounds_check_faults_on_upper_bound() {
    let mut module = Module::new();
    module.add_function(Function::new(
        "main",
        vec![],
        Type::I64,
        vec![Instr::Const(2), Instr::BoundsCheck(2), Instr::Ret],
        vec![],
    ));
    let result = compile_and_run_native(&module).expect("native bounds-check compile/run");
    assert_eq!(result.status, Some(codegen_native::BOUNDS_FAULT_EXIT));
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn native_load_index_reads_contiguous_local_cell() {
    let mut module = Module::new();
    module.add_function(Function::new(
        "main",
        vec![],
        Type::I64,
        vec![
            Instr::Const(10),
            Instr::StoreOffset(2, 0),
            Instr::Const(20),
            Instr::StoreOffset(2, 8),
            Instr::Const(42),
            Instr::StoreOffset(2, 16),
            Instr::Const(2),
            Instr::LoadIndex { base: 2, len: 3 },
            Instr::Ret,
        ],
        vec![],
    ));
    let result = compile_and_run_native(&module).expect("native indexed load compile/run");
    assert_eq!(result.status, Some(42));
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn native_load_index_faults_before_out_of_bounds_memory_access() {
    let mut module = Module::new();
    module.add_function(Function::new(
        "main",
        vec![],
        Type::I64,
        vec![
            Instr::Const(10),
            Instr::StoreOffset(1, 0),
            Instr::Const(20),
            Instr::StoreOffset(1, 8),
            Instr::Const(-1),
            Instr::LoadIndex { base: 1, len: 2 },
            Instr::Ret,
        ],
        vec![],
    ));
    let result = compile_and_run_native(&module).expect("native indexed load compile/run");
    assert_eq!(result.status, Some(codegen_native::BOUNDS_FAULT_EXIT));
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn native_rejects_misaligned_aggregate_offset_before_emission() {
    let mut module = Module::new();
    module.add_function(Function::new(
        "main",
        vec![],
        Type::I64,
        vec![Instr::LoadOffset(1, 4), Instr::Ret],
        vec![],
    ));
    let mut path = std::env::temp_dir();
    path.push(format!(
        "omni-invalid-aggregate-alignment-{}",
        std::process::id()
    ));
    let err = codegen_native::compile_to_target(
        &module,
        &path,
        codegen_native::NativeTarget::X86_64Linux,
    )
    .expect_err("misaligned aggregate access must be rejected");
    assert!(
        err.contains("multiple of 8"),
        "unexpected diagnostic: {err}"
    );
    assert!(!path.exists(), "invalid artifact must not be written");
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn native_rejects_zero_length_index_target_before_emission() {
    let mut module = Module::new();
    module.add_function(Function::new(
        "main",
        vec![],
        Type::I64,
        vec![
            Instr::Const(0),
            Instr::LoadIndex { base: 0, len: 0 },
            Instr::Ret,
        ],
        vec![],
    ));
    let mut path = std::env::temp_dir();
    path.push(format!("omni-zero-length-index-{}", std::process::id()));
    let err = codegen_native::compile_to_target(
        &module,
        &path,
        codegen_native::NativeTarget::X86_64Linux,
    )
    .expect_err("zero-length indexed aggregate must be rejected");
    assert!(
        err.contains("zero-length aggregate"),
        "unexpected diagnostic: {err}"
    );
    assert!(!path.exists(), "invalid artifact must not be written");
}
