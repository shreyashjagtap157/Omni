use lir::Module;

pub fn is_llvm_available() -> bool {
    std::env::var("LLVM_SYS_191_PREFIX").is_ok()
        || std::env::var("LLVM_SYS_PREFIX").is_ok()
}

pub fn get_llvm_version() -> String {
    if let Ok(v) = std::env::var("LLVM_VERSION") {
        return v;
    }
    if std::env::var("LLVM_SYS_191_PREFIX").is_ok() {
        return "19.1".to_string();
    }
    "not configured".to_string()
}

/// Check if we can use the real LLVM backend
pub fn can_use_real_llvm() -> bool {
    is_llvm_available()
}

/// Development bridge for the `use_llvm` feature.
///
/// The workspace keeps the Cranelift JIT available as the default runtime,
/// while the feature-gated LLVM path handles real toolchain-backed runs.
#[cfg(not(feature = "real_llvm"))]
pub fn compile_and_run_with_llvm(module: &Module) -> Result<Vec<i64>, String> {
    codegen_cranelift::compile_and_run_with_jit(module)
}

#[cfg(all(feature = "real_llvm", feature = "with_inkwell"))]
pub fn compile_and_run_with_llvm(module: &Module) -> Result<Vec<i64>, String> {
    use inkwell::context::Context;
    use inkwell::types::BasicMetadataTypeEnum;
    use inkwell::values::{BasicMetadataValueEnum, FunctionValue, IntValue, PointerValue};
    use inkwell::{AddressSpace, IntPredicate, OptimizationLevel};
    use std::collections::HashMap;

    #[derive(Clone, Copy)]
    struct FunctionMeta<'ctx> {
        value: FunctionValue<'ctx>,
        params: usize,
        rets: usize,
    }

    fn stack_slot_ptr<'ctx>(
        builder: &inkwell::builder::Builder<'ctx>,
        stack_ptr: PointerValue<'ctx>,
        stack_ty: inkwell::types::ArrayType<'ctx>,
        idx: IntValue<'ctx>,
        i32t: inkwell::types::IntType<'ctx>,
    ) -> Result<PointerValue<'ctx>, String> {
        unsafe {
            builder
                .build_in_bounds_gep(
                    stack_ty,
                    stack_ptr,
                    &[i32t.const_int(0, false).into(), idx.into()],
                    "stack_slot",
                )
                .map_err(|e| format!("inkwell gep error: {}", e.to_string()))
        }
    }

    fn stack_push<'ctx>(
        builder: &inkwell::builder::Builder<'ctx>,
        stack_ptr: PointerValue<'ctx>,
        stack_ty: inkwell::types::ArrayType<'ctx>,
        sp_ptr: PointerValue<'ctx>,
        i32t: inkwell::types::IntType<'ctx>,
        value: IntValue<'ctx>,
    ) -> Result<(), String> {
        let sp = builder
            .build_load(i32t, sp_ptr, "sp_val")
            .map_err(|e| e.to_string())?
            .into_int_value();
        let slot = stack_slot_ptr(builder, stack_ptr, stack_ty, sp, i32t)?;
        builder
            .build_store(slot, value)
            .map_err(|e| e.to_string())?;
        let new_sp = builder
            .build_int_add(sp, i32t.const_int(1, false), "sp_inc")
            .map_err(|e| e.to_string())?;
        builder
            .build_store(sp_ptr, new_sp)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn stack_pop<'ctx>(
        builder: &inkwell::builder::Builder<'ctx>,
        stack_ptr: PointerValue<'ctx>,
        stack_ty: inkwell::types::ArrayType<'ctx>,
        i64t: inkwell::types::IntType<'ctx>,
        sp_ptr: PointerValue<'ctx>,
        i32t: inkwell::types::IntType<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        let sp = builder
            .build_load(i32t, sp_ptr, "sp_val")
            .map_err(|e| e.to_string())?
            .into_int_value();
        let new_sp = builder
            .build_int_sub(sp, i32t.const_int(1, false), "sp_dec")
            .map_err(|e| e.to_string())?;
        builder
            .build_store(sp_ptr, new_sp)
            .map_err(|e| e.to_string())?;
        let slot = stack_slot_ptr(builder, stack_ptr, stack_ty, new_sp, i32t)?;
        Ok(builder
            .build_load(i64t, slot, "stack_pop")
            .map_err(|e| e.to_string())?
            .into_int_value())
    }

    let context = Context::create();
    let module_ir = context.create_module("omni_jit");
    let builder = context.create_builder();
    let i64t = context.i64_type();
    let i32t = context.i32_type();
    let ptr_i64 = i64t.ptr_type(AddressSpace::default());

    let mut functions: HashMap<String, FunctionMeta<'_>> = HashMap::new();
    for f in &module.functions {
        let mut params: Vec<BasicMetadataTypeEnum> = Vec::new();
        if f.rets.len() > 1 {
            params.push(ptr_i64.into());
        }
        for _ in &f.params {
            params.push(i64t.into());
        }

        let fn_type = if f.rets.len() == 1 {
            i64t.fn_type(&params, false)
        } else {
            context.void_type().fn_type(&params, false)
        };
        let value = module_ir.add_function(&f.name, fn_type, None);
        functions.insert(
            f.name.clone(),
            FunctionMeta {
                value,
                params: f.params.len(),
                rets: f.rets.len(),
            },
        );
    }

    let print_fn = {
        let sig = context.void_type().fn_type(&[i64t.into()], false);
        module_ir.add_function("print", sig, None)
    };
    functions.insert(
        "print".to_string(),
        FunctionMeta {
            value: print_fn,
            params: 1,
            rets: 0,
        },
    );

    for f in &module.functions {
        let meta = functions
            .get(&f.name)
            .ok_or_else(|| format!("missing metadata for function '{}'", f.name))?
            .to_owned();

        let entry = context.append_basic_block(meta.value, "entry");
        let dispatch = context.append_basic_block(meta.value, "dispatch");
        let case_blocks: Vec<_> = (0..f.body.len())
            .map(|i| context.append_basic_block(meta.value, &format!("bb{i}")))
            .collect();
        let default_block = context.append_basic_block(meta.value, "default");

        builder.position_at_end(entry);

        let stack_capacity = (f.body.len() + f.params.len() + 8) as u32;
        let stack_ty = i64t.array_type(stack_capacity);
        let stack_ptr = builder
            .build_alloca(stack_ty, "stack")
            .map_err(|e| e.to_string())?;
        let sp_ptr = builder
            .build_alloca(i32t, "sp")
            .map_err(|e| e.to_string())?;
        let pc_ptr = builder
            .build_alloca(i32t, "pc")
            .map_err(|e| e.to_string())?;
        builder
            .build_store(sp_ptr, i32t.const_int(0, false))
            .map_err(|e| e.to_string())?;
        builder
            .build_store(pc_ptr, i32t.const_int(0, false))
            .map_err(|e| e.to_string())?;

        let max_slot = f
            .body
            .iter()
            .filter_map(|instr| match instr {
                lir::Instr::Load(slot) | lir::Instr::Store(slot) | lir::Instr::Drop(slot) => {
                    Some(*slot as usize)
                }
                _ => None,
            })
            .max()
            .unwrap_or(0);
        let mut slot_allocas: Vec<PointerValue> = Vec::new();
        for slot in 0..=max_slot {
            let ptr = builder
                .build_alloca(i64t, &format!("slot_{slot}"))
                .map_err(|e| e.to_string())?;
            builder
                .build_store(ptr, i64t.const_zero())
                .map_err(|e| e.to_string())?;
            slot_allocas.push(ptr);
        }

        let out_ptr_alloca = if meta.rets > 1 {
            let ptr = builder
                .build_alloca(ptr_i64, "out_ptr")
                .map_err(|e| e.to_string())?;
            Some(ptr)
        } else {
            None
        };

        let mut param_index = 0usize;
        if let Some(out_alloca) = out_ptr_alloca {
            let out_param = meta
                .value
                .get_nth_param(0)
                .ok_or_else(|| format!("missing hidden return buffer for function '{}'", f.name))?;
            builder
                .build_store(out_alloca, out_param.into_pointer_value())
                .map_err(|e| e.to_string())?;
            param_index = 1;
        }

        for (i, _param_name) in f.params.iter().enumerate() {
            let param_val = meta
                .value
                .get_nth_param((param_index + i) as u32)
                .ok_or_else(|| format!("missing parameter {} for function '{}'", i, f.name))?
                .into_int_value();
            let slot = stack_slot_ptr(
                &builder,
                stack_ptr,
                stack_ty,
                i32t.const_int(i as u64, false),
                i32t,
            )?;
            builder
                .build_store(slot, param_val)
                .map_err(|e| e.to_string())?;
        }
        builder
            .build_store(sp_ptr, i32t.const_int(f.params.len() as u64, false))
            .map_err(|e| e.to_string())?;

        if f.body.is_empty() {
            builder.position_at_end(entry);
            builder.build_unconditional_branch(default_block);
        } else {
            builder.position_at_end(entry);
            builder.build_unconditional_branch(dispatch);
        }

        builder.position_at_end(dispatch);
        let pc_val = builder
            .build_load(i32t, pc_ptr, "pc_load")
            .map_err(|e| e.to_string())?
            .into_int_value();
        let cases: Vec<_> = case_blocks
            .iter()
            .enumerate()
            .map(|(idx, block)| (i32t.const_int(idx as u64, false), *block))
            .collect();
        builder
            .build_switch(pc_val, default_block, &cases)
            .map_err(|e| e.to_string())?;

        for (idx, instr) in f.body.iter().enumerate() {
            let block = case_blocks[idx];
            builder.position_at_end(block);

            let mut terminated = false;
            match instr {
                lir::Instr::Const(v) => {
                    stack_push(
                        &builder,
                        stack_ptr,
                        stack_ty,
                        sp_ptr,
                        i32t,
                        i64t.const_int(*v as u64, true),
                    )?;
                }
                lir::Instr::Add => {
                    let b = stack_pop(&builder, stack_ptr, stack_ty, i64t, sp_ptr, i32t)?;
                    let a = stack_pop(&builder, stack_ptr, stack_ty, i64t, sp_ptr, i32t)?;
                    stack_push(
                        &builder,
                        stack_ptr,
                        stack_ty,
                        sp_ptr,
                        i32t,
                        builder
                            .build_int_add(a, b, "addtmp")
                            .map_err(|e| e.to_string())?,
                    )?;
                }
                lir::Instr::Sub => {
                    let b = stack_pop(&builder, stack_ptr, stack_ty, i64t, sp_ptr, i32t)?;
                    let a = stack_pop(&builder, stack_ptr, stack_ty, i64t, sp_ptr, i32t)?;
                    stack_push(
                        &builder,
                        stack_ptr,
                        stack_ty,
                        sp_ptr,
                        i32t,
                        builder
                            .build_int_sub(a, b, "subtmp")
                            .map_err(|e| e.to_string())?,
                    )?;
                }
                lir::Instr::Mul => {
                    let b = stack_pop(&builder, stack_ptr, stack_ty, i64t, sp_ptr, i32t)?;
                    let a = stack_pop(&builder, stack_ptr, stack_ty, i64t, sp_ptr, i32t)?;
                    stack_push(
                        &builder,
                        stack_ptr,
                        stack_ty,
                        sp_ptr,
                        i32t,
                        builder
                            .build_int_mul(a, b, "multmp")
                            .map_err(|e| e.to_string())?,
                    )?;
                }
                lir::Instr::Div => {
                    let b = stack_pop(&builder, stack_ptr, stack_ty, i64t, sp_ptr, i32t)?;
                    let a = stack_pop(&builder, stack_ptr, stack_ty, i64t, sp_ptr, i32t)?;
                    stack_push(
                        &builder,
                        stack_ptr,
                        stack_ty,
                        sp_ptr,
                        i32t,
                        builder
                            .build_int_signed_div(a, b, "divtmp")
                            .map_err(|e| e.to_string())?,
                    )?;
                }
                lir::Instr::Load(slot) => {
                    let slot_ptr = *slot_allocas
                        .get(*slot as usize)
                        .ok_or_else(|| format!("invalid slot {}", slot))?;
                    let v = builder
                        .build_load(i64t, slot_ptr, "load_slot")
                        .map_err(|e| e.to_string())?
                        .into_int_value();
                    stack_push(&builder, stack_ptr, stack_ty, sp_ptr, i32t, v)?;
                }
                lir::Instr::Store(slot) => {
                    let val = stack_pop(&builder, stack_ptr, stack_ty, i64t, sp_ptr, i32t)?;
                    let slot_ptr = *slot_allocas
                        .get(*slot as usize)
                        .ok_or_else(|| format!("invalid slot {}", slot))?;
                    builder
                        .build_store(slot_ptr, val)
                        .map_err(|e| e.to_string())?;
                }
                lir::Instr::Call(name) => {
                    let callee_meta = functions
                        .get(name)
                        .ok_or_else(|| format!("unsupported call '{}' in LLVM backend", name))?
                        .to_owned();
                    let target = callee_meta.value;

                    if name == "print" {
                        let arg = stack_pop(&builder, stack_ptr, stack_ty, i64t, sp_ptr, i32t)?;
                        let call_args: Vec<BasicMetadataValueEnum> = vec![arg.into()];
                        let _ = builder
                            .build_call(target, &call_args, "print_call")
                            .map_err(|e| e.to_string())?;
                    } else {
                        let mut call_args: Vec<BasicMetadataValueEnum> = Vec::new();
                        if callee_meta.rets > 1 {
                            let retbuf_ty = i64t.array_type(callee_meta.rets as u32);
                            let retbuf = builder
                                .build_alloca(retbuf_ty, "call_retbuf")
                                .map_err(|e| e.to_string())?;
                            let retbuf_ptr = unsafe {
                                builder
                                    .build_in_bounds_gep(
                                        retbuf_ty,
                                        retbuf,
                                        &[
                                            i32t.const_int(0, false).into(),
                                            i32t.const_int(0, false).into(),
                                        ],
                                        "call_retbuf_ptr",
                                    )
                                    .map_err(|e| e.to_string())?
                            };
                            call_args.push(retbuf_ptr.into());
                            let mut args = Vec::new();
                            for _ in 0..callee_meta.params {
                                args.push(stack_pop(&builder, stack_ptr, stack_ty, i64t, sp_ptr, i32t)?);
                            }
                            args.reverse();
                            for arg in args {
                                call_args.push(arg.into());
                            }
                            let _ = builder
                                .build_call(target, &call_args, "call_multi")
                                .map_err(|e| e.to_string())?;
                            for i in 0..callee_meta.rets {
                                let gep = unsafe {
                                    builder
                                        .build_in_bounds_gep(
                                            i64t,
                                            retbuf_ptr,
                                            &[i32t.const_int(i as u64, false).into()],
                                            "call_ret_gep",
                                        )
                                        .map_err(|e| e.to_string())?
                                };
                                let v = builder
                                    .build_load(i64t, gep, "call_ret")
                                    .map_err(|e| e.to_string())?
                                    .into_int_value();
                                stack_push(&builder, stack_ptr, stack_ty, sp_ptr, i32t, v)?;
                            }
                        } else {
                            let mut args = Vec::new();
                            for _ in 0..callee_meta.params {
                                args.push(stack_pop(&builder, stack_ptr, stack_ty, i64t, sp_ptr, i32t)?);
                            }
                            args.reverse();
                            for arg in args {
                                call_args.push(arg.into());
                            }
                                let call = builder
                                    .build_call(target, &call_args, "calltmp")
                                    .map_err(|e| e.to_string())?;
                            if callee_meta.rets == 1 {
                                let ret = match call.try_as_basic_value() {
                                    inkwell::values::ValueKind::Basic(value) => value.into_int_value(),
                                    inkwell::values::ValueKind::Instruction(_) => {
                                        return Err(format!("call '{}' did not produce a value", name));
                                    }
                                };
                                stack_push(&builder, stack_ptr, stack_ty, sp_ptr, i32t, ret)?;
                            }
                        }
                    }
                }
                lir::Instr::Jump(target) => {
                    if *target >= case_blocks.len() {
                        return Err(format!("invalid jump target {}", target));
                    }
                    builder
                        .build_store(pc_ptr, i32t.const_int(*target as u64, false))
                        .map_err(|e| e.to_string())?;
                    builder.build_unconditional_branch(dispatch);
                    terminated = true;
                }
                lir::Instr::CondJump { if_true, if_false } => {
                    let cond = stack_pop(&builder, stack_ptr, stack_ty, i64t, sp_ptr, i32t)?;
                    let cond_bool = builder.build_int_compare(
                        IntPredicate::NE,
                        cond,
                        i64t.const_zero(),
                        "cond",
                    ).map_err(|e| e.to_string())?;

                    let true_block =
                        context.append_basic_block(meta.value, &format!("bb{idx}_true"));
                    let false_block =
                        context.append_basic_block(meta.value, &format!("bb{idx}_false"));
                    builder.build_conditional_branch(cond_bool, true_block, false_block);

                    builder.position_at_end(true_block);
                    builder
                        .build_store(pc_ptr, i32t.const_int(*if_true as u64, false))
                        .map_err(|e| e.to_string())?;
                    builder.build_unconditional_branch(dispatch);

                    builder.position_at_end(false_block);
                    builder
                        .build_store(pc_ptr, i32t.const_int(*if_false as u64, false))
                        .map_err(|e| e.to_string())?;
                    builder.build_unconditional_branch(dispatch);

                    terminated = true;
                }
                lir::Instr::Drop(_) | lir::Instr::Nop => {}
                lir::Instr::Ret => {
                    match f.rets.len() {
                        0 => {
                            builder.build_return(None);
                        }
                        1 => {
                            let ret = stack_pop(&builder, stack_ptr, stack_ty, i64t, sp_ptr, i32t)?;
                            builder.build_return(Some(&ret));
                        }
                        _ => {
                            let out_ptr = out_ptr_alloca.ok_or_else(|| {
                                "missing return buffer for multi-return function".to_string()
                            })?;
                            let out_ptr_val = builder
                                .build_load(ptr_i64, out_ptr, "out_ptr")
                                .map_err(|e| e.to_string())?
                                .into_pointer_value();
                            let mut rets = Vec::new();
                            for _ in 0..f.rets.len() {
                                rets.push(stack_pop(&builder, stack_ptr, stack_ty, i64t, sp_ptr, i32t)?);
                            }
                            rets.reverse();
                            for (i, val) in rets.into_iter().enumerate() {
                                let gep = unsafe {
                                    builder
                                        .build_in_bounds_gep(
                                            i64t,
                                            out_ptr_val,
                                            &[i32t.const_int(i as u64, false).into()],
                                            "retbuf_gep",
                                        )
                                        .map_err(|e| e.to_string())?
                                };
                                builder.build_store(gep, val).map_err(|e| e.to_string())?;
                            }
                            builder.build_return(None);
                        }
                    }
                    terminated = true;
                }
            }

            if terminated {
                continue;
            }

            let next_pc = if idx + 1 < case_blocks.len() {
                i32t.const_int((idx + 1) as u64, false)
            } else {
                i32t.const_int(0, false)
            };
            builder
                .build_store(pc_ptr, next_pc)
                .map_err(|e| e.to_string())?;
            builder.build_unconditional_branch(dispatch);
        }

        builder.position_at_end(default_block);
        if f.rets.len() == 1 {
            let zero_ret = i64t.const_zero();
            builder.build_return(Some(&zero_ret));
        } else {
            builder.build_return(None);
        }
    }

    let ee = module_ir
        .create_jit_execution_engine(OptimizationLevel::None)
        .map_err(|e| format!("JIT error: {}", e.to_string()))?;

    let entry_name = module
        .functions
        .iter()
        .find(|f| f.name == "main")
        .or_else(|| module.functions.first())
        .ok_or_else(|| "no functions in module".to_string())?
        .name
        .clone();
    let entry_meta = functions
        .get(&entry_name)
        .ok_or_else(|| format!("entry function '{}' not declared", entry_name))?;

    unsafe {
        if entry_meta.params != 0 {
            return Err("real_llvm backend entry wrapper currently supports zero-argument entry functions only".to_string());
        }

        match entry_meta.rets {
            0 => {
                type MainFn = unsafe extern "C" fn();
                let main_fn = ee
                    .get_function::<MainFn>(&entry_name)
                    .map_err(|e| format!("missing main: {}", e.to_string()))?;
                main_fn.call();
                Ok(Vec::new())
            }
            1 => {
                type MainFn = unsafe extern "C" fn() -> i64;
                let main_fn = ee
                    .get_function::<MainFn>(&entry_name)
                    .map_err(|e| format!("missing main: {}", e.to_string()))?;
                Ok(vec![main_fn.call()])
            }
            count => {
                type MainFn = unsafe extern "C" fn(*mut i64);
                let main_fn = ee
                    .get_function::<MainFn>(&entry_name)
                    .map_err(|e| format!("missing main: {}", e.to_string()))?;
                let mut buffer = vec![0i64; count];
                main_fn.call(buffer.as_mut_ptr());
                Ok(buffer)
            }
        }
    }
}
#[cfg(all(
    feature = "real_llvm",
    not(feature = "with_inkwell"),
    not(feature = "with_inkwell_stub")
))]
pub fn compile_and_run_with_llvm(_module: &Module) -> Result<Vec<i64>, String> {
    Err("real_llvm backend not available: compile with feature 'with_inkwell' and install a compatible LLVM toolchain, or enable 'with_inkwell_stub' for local development".to_string())
}

// When building without the real LLVM toolchain, keep the public API usable
// by routing execution through the Cranelift backend.
#[cfg(all(feature = "real_llvm", feature = "with_inkwell_stub"))]
pub fn compile_and_run_with_llvm(module: &Module) -> Result<Vec<i64>, String> {
    // Mirror the real LLVM path while keeping the runtime deterministic.
    codegen_cranelift::compile_and_run_with_jit(module)
}
