#[cfg(feature = "real_llvm")]
use codegen_llvm::compile_and_run_with_llvm;
#[cfg(feature = "real_llvm")]
use lir::example_module;

#[cfg(feature = "real_llvm")]
use std::hint::black_box;

#[cfg(feature = "real_llvm")]
use std::time::Instant;

#[cfg(feature = "real_llvm")]
use lir::{Function, Instr, Module, Type};

#[cfg(feature = "real_llvm")]
#[test]
fn real_llvm_runs_example_module() {
    let m = example_module();
    let res = compile_and_run_with_llvm(&m).expect("real_llvm run failed");
    assert_eq!(res, vec![42]);
}

#[cfg(feature = "real_llvm")]
fn build_perf_module(term_count: usize) -> Module {
    let mut body = Vec::new();
    let total_terms = term_count.max(2);

    body.push(Instr::Const(0));
    for term in 1..total_terms {
        body.push(Instr::Const(term as i64));
        body.push(Instr::Add);
    }
    body.push(Instr::Ret);

    let mut module = Module::new();
    module.add_function(Function::new("main", vec![], Type::I64, body));
    module
}

#[cfg(feature = "real_llvm")]
#[test]
#[ignore = "toolchain-backed Step 11 performance gate"]
fn real_llvm_acceptance_perf_smoke() {
    let module = build_perf_module(256);
    let reference = codegen_cranelift::compile_and_run_with_jit(&module)
        .expect("reference JIT run failed");

    let start = Instant::now();
    let result = compile_and_run_with_llvm(&module).expect("real_llvm perf run failed");
    let elapsed = start.elapsed();

    assert_eq!(result, reference);
    println!("real_llvm perf gate elapsed: {:?}", elapsed);
    black_box(elapsed);
}
