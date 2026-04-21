// Minimal scaffold for future Cranelift-based codegen crate.
use lir::{Instr as LirInstr, Module};
use std::collections::HashMap;

/// Textual stub: render the LIR for debugging.
pub fn compile_lir_stub(module: &Module) -> String {
    let mut out = String::new();
    for func in &module.functions {
        out.push_str(&format!("fn {}() -> {:?}\n", func.name, func.rets));
        for (i, instr) in func.body.iter().enumerate() {
            out.push_str(&format!("  {}: {:?}\n", i, instr));
        }
    }
    out
}

/// Result of running a LIR function.
pub struct RunResult {
    // Support multiple return values from the entry function
    pub return_values: Vec<i64>,
    pub prints: Vec<String>,
}

/// Simple stack-based interpreter for the LIR module. This provides a
/// deterministic, dependency-free execution path suitable for tests and
/// as a fallback when Cranelift isn't enabled.
pub fn run_lir_interpreter(module: &Module) -> Result<RunResult, String> {
    if module.functions.is_empty() {
        return Err("no functions in module".into());
    }

    // Build a name -> index map for functions
    let mut name_map: HashMap<String, usize> = HashMap::new();
    for (i, f) in module.functions.iter().enumerate() {
        name_map.insert(f.name.clone(), i);
    }

    let mut prints: Vec<String> = Vec::new();

    // Recursive interpreter for a function index given argument values.
    fn interp_fn(
        module: &Module,
        name_map: &HashMap<String, usize>,
        idx: usize,
        mut args: Vec<i64>,
        prints: &mut Vec<String>,
    ) -> Result<Vec<i64>, String> {
        let f = &module.functions[idx];
        let body = &f.body;
        let mut locals: HashMap<u32, i64> = HashMap::new();
        let mut stack: Vec<i64> = Vec::new();

        // initial stack contains parameters in order
        for a in args.drain(..) {
            stack.push(a);
        }

        let mut ip: usize = 0;
        while ip < body.len() {
            match &body[ip] {
                LirInstr::Const(v) => stack.push(*v),
                LirInstr::Add => {
                    let b = stack.pop().unwrap_or(0);
                    let a = stack.pop().unwrap_or(0);
                    stack.push(a + b);
                }
                LirInstr::Sub => {
                    let b = stack.pop().unwrap_or(0);
                    let a = stack.pop().unwrap_or(0);
                    stack.push(a - b);
                }
                LirInstr::Mul => {
                    let b = stack.pop().unwrap_or(0);
                    let a = stack.pop().unwrap_or(0);
                    stack.push(a * b);
                }
                LirInstr::Div => {
                    let b = stack.pop().unwrap_or(0);
                    let a = stack.pop().unwrap_or(0);
                    let res = if b == 0 { 0 } else { a / b };
                    stack.push(res);
                }
                LirInstr::Load(slot) => {
                    let v = *locals.get(slot).unwrap_or(&0);
                    stack.push(v);
                }
                LirInstr::Store(slot) => {
                    let v = stack.pop().unwrap_or(0);
                    locals.insert(*slot, v);
                }
                LirInstr::Call(name) => {
                    if name == "print" {
                        let v = stack.pop().unwrap_or(0);
                        prints.push(format!("{}", v));
                    } else {
                        let callee_idx = *name_map
                            .get(name)
                            .ok_or_else(|| format!("unknown function '{}'", name))?;
                        let target = &module.functions[callee_idx];
                        let mut cargs: Vec<i64> = Vec::new();
                        for _ in 0..target.params.len() {
                            cargs.push(stack.pop().unwrap_or(0));
                        }
                        cargs.reverse();
                        let rets = interp_fn(module, name_map, callee_idx, cargs, prints)?;
                        for v in rets {
                            stack.push(v);
                        }
                    }
                }
                LirInstr::Ret => {
                    let ret_count = f.rets.len();
                    let mut rets: Vec<i64> = Vec::new();
                    for _ in 0..ret_count {
                        rets.push(stack.pop().unwrap_or(0));
                    }
                    rets.reverse();
                    return Ok(rets);
                }
                LirInstr::Jump(target) => {
                    ip = *target;
                    continue;
                }
                LirInstr::CondJump { if_true, if_false } => {
                    let cond = stack.pop().unwrap_or(0);
                    ip = if cond != 0 { *if_true } else { *if_false };
                    continue;
                }
                LirInstr::Drop(_slot) => {
                    // no-op for interpreter
                }
                LirInstr::Nop => {}
            }
            ip += 1;
        }

        // fallthrough: no explicit return
        Ok(Vec::new())
    }

    let entry_idx = module
        .functions
        .iter()
        .position(|f| f.name == "main")
        .unwrap_or(0);
    let rets = interp_fn(module, &name_map, entry_idx, Vec::new(), &mut prints)?;
    Ok(RunResult {
        return_values: rets,
        prints,
    })
}

mod cranelift_backend {
    use super::*;
    use cranelift_codegen::ir::InstBuilder;
    use cranelift_codegen::ir::{types, AbiParam, Block, Value};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
    use cranelift_jit::{JITBuilder, JITModule};
    use cranelift_module::{Linkage, Module as CraneliftModule};

    extern "C" fn print_i64(v: i64) {
        // Keep printing simple and host-side.
        println!("{}", v);
    }

    /// Compile and run the first function in the module using Cranelift JIT.
    /// This supports integer arithmetic, local slots, `print` (imported),
    /// and functions with multiple return values for callees. The entry
    /// function must return either 0 or 1 value for direct JIT invocation.
    pub fn compile_and_run_with_jit(module: &Module) -> Result<Vec<i64>, String> {
        // Build JIT with the `print` symbol available to the generated code.
        let mut jit_builder = JITBuilder::new(cranelift_module::default_libcall_names())
            .map_err(|e| e.to_string())?;
        jit_builder.symbol("print", print_i64 as *const u8);
        let mut jit = JITModule::new(jit_builder);

        use cranelift_codegen::ir::Signature as CraneliftSig;
        use cranelift_module::FuncId;

        // Pre-declare all functions in the module with proper signatures so
        // we can reference them during lowering. Also declare the imported
        // `print(i64)` symbol.
        let mut func_id_map: HashMap<String, FuncId> = HashMap::new();
        let mut sig_map: HashMap<String, CraneliftSig> = HashMap::new();

        for f in &module.functions {
            let mut sig = jit.make_signature();
            for _ in &f.params {
                sig.params.push(AbiParam::new(types::I64));
            }
            // Support multiple return values
            for _ in &f.rets {
                sig.returns.push(AbiParam::new(types::I64));
            }
            let id = jit
                .declare_function(&f.name, Linkage::Local, &sig)
                .map_err(|e| e.to_string())?;
            func_id_map.insert(f.name.clone(), id);
            sig_map.insert(f.name.clone(), sig);
        }

        // Imported print
        let mut sig_print = jit.make_signature();
        sig_print.params.push(AbiParam::new(types::I64));
        let print_id = jit
            .declare_function("print", Linkage::Import, &sig_print)
            .map_err(|e| e.to_string())?;
        func_id_map.insert("print".to_string(), print_id);
        sig_map.insert("print".to_string(), sig_print);

        // Lower and define each function.
        for f in &module.functions {
            let func_id = *func_id_map
                .get(&f.name)
                .ok_or_else(|| "missing func id".to_string())?;
            let sig = sig_map
                .get(&f.name)
                .ok_or_else(|| "missing signature".to_string())?
                .clone();

            let mut ctx = jit.make_context();
            ctx.func.signature = sig;

            let mut func_ctx = FunctionBuilderContext::new();
            let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);

            // Pre-create variables for local slots for this function
            let max_slot = f
                .body
                .iter()
                .cloned()
                .filter_map(|instr| match instr {
                    LirInstr::Store(slot) | LirInstr::Load(slot) | LirInstr::Drop(slot) => {
                        Some(slot as usize)
                    }
                    _ => None,
                })
                .max()
                .map(|s| s + 1)
                .unwrap_or(0);

            let mut var_map: HashMap<u32, Variable> = HashMap::new();
            for s in 0..max_slot {
                let v = Variable::with_u32(s as u32);
                builder.declare_var(v, types::I64);
                var_map.insert(s as u32, v);
            }

            // Rely on Cranelift's register allocator to preserve locals
            // across calls where appropriate; avoid explicit caller-side
            // stack-store/stack-load sequences which duplicate register
            // allocator work and increase code complexity.

            let has_cf = f
                .body
                .iter()
                .any(|i| matches!(i, LirInstr::Jump(_) | LirInstr::CondJump { .. }));

            if !has_cf {
                let entry = builder.create_block();
                // function params become initial stack values
                for _ in &f.params {
                    builder.append_block_param(entry, types::I64);
                }
                builder.switch_to_block(entry);
                builder.seal_block(entry);

                let mut vstack: Vec<Value> = builder.block_params(entry).to_vec();

                for instr in &f.body {
                    match instr {
                        LirInstr::Const(v) => {
                            let val = builder.ins().iconst(types::I64, *v);
                            vstack.push(val);
                        }
                        LirInstr::Add => {
                            let b = vstack.pop().ok_or("stack underflow")?;
                            let a = vstack.pop().ok_or("stack underflow")?;
                            let r = builder.ins().iadd(a, b);
                            vstack.push(r);
                        }
                        LirInstr::Sub => {
                            let b = vstack.pop().ok_or("stack underflow")?;
                            let a = vstack.pop().ok_or("stack underflow")?;
                            let r = builder.ins().isub(a, b);
                            vstack.push(r);
                        }
                        LirInstr::Mul => {
                            let b = vstack.pop().ok_or("stack underflow")?;
                            let a = vstack.pop().ok_or("stack underflow")?;
                            let r = builder.ins().imul(a, b);
                            vstack.push(r);
                        }
                        LirInstr::Div => {
                            let b = vstack.pop().ok_or("stack underflow")?;
                            let a = vstack.pop().ok_or("stack underflow")?;
                            let r = builder.ins().sdiv(a, b);
                            vstack.push(r);
                        }
                        LirInstr::Load(slot) => {
                            let v = *var_map
                                .get(slot)
                                .ok_or_else(|| "invalid slot".to_string())?;
                            let val = builder.use_var(v);
                            vstack.push(val);
                        }
                        LirInstr::Store(slot) => {
                            let val = vstack.pop().ok_or("stack underflow")?;
                            let var = *var_map
                                .get(slot)
                                .ok_or_else(|| "invalid slot".to_string())?;
                            builder.def_var(var, val);
                        }
                        LirInstr::Call(name) => {
                            if name == "print" {
                                let arg = vstack.pop().ok_or("stack underflow")?;
                                let print_id = *func_id_map
                                    .get("print")
                                    .ok_or_else(|| "print not declared".to_string())?;
                                let callee = jit.declare_func_in_func(print_id, builder.func);
                                builder.ins().call(callee, &[arg]);
                            } else if let Some(callee_id) = func_id_map.get(name) {
                                let target = module
                                    .functions
                                    .iter()
                                    .find(|ff| &ff.name == name)
                                    .ok_or_else(|| "callee signature missing".to_string())?;
                                let mut args: Vec<Value> = Vec::new();
                                for _ in 0..target.params.len() {
                                    args.push(
                                        vstack
                                            .pop()
                                            .ok_or("stack underflow during call lowering")?,
                                    );
                                }
                                args.reverse();
                                let callee_ref =
                                    jit.declare_func_in_func(*callee_id, builder.func);
                                let call_inst = builder.ins().call(callee_ref, &args);
                                // Push all returned values (support multi-return callees)
                                let call_results = builder.inst_results(call_inst);
                                for rv in call_results {
                                    vstack.push(*rv);
                                }
                            } else {
                                return Err(format!(
                                    "unsupported call '{}' in Cranelift backend",
                                    name
                                ));
                            }
                        }
                        LirInstr::Ret => {
                            let ret_count = f.rets.len();
                            let mut ret_vals: Vec<Value> = Vec::new();
                            for _ in 0..ret_count {
                                let val =
                                    vstack.pop().unwrap_or(builder.ins().iconst(types::I64, 0));
                                ret_vals.push(val);
                            }
                            ret_vals.reverse();
                            builder.ins().return_(&ret_vals);
                        }
                        _ => {}
                    }
                }
            } else {
                // Control-flow lowering with block parameters. Incoming[0]
                // seeded from function parameters length so params are
                // available on the initial stack.

                let body = &f.body;
                let n = body.len();
                let mut incoming: Vec<Option<usize>> = vec![None; n];
                if n > 0 {
                    incoming[0] = Some(f.params.len());
                }

                use std::collections::VecDeque;
                let mut q: VecDeque<usize> = VecDeque::new();
                if n > 0 {
                    q.push_back(0);
                }

                let stack_delta = |idx: usize| -> Result<isize, String> {
                    match &body[idx] {
                        LirInstr::Const(_) => Ok(1),
                        LirInstr::Add | LirInstr::Sub | LirInstr::Mul | LirInstr::Div => Ok(-1),
                        LirInstr::Load(_) => Ok(1),
                        LirInstr::Store(_) => Ok(-1),
                        LirInstr::Call(name) => {
                            if name == "print" {
                                Ok(-1)
                            } else if let Some(target) =
                                module.functions.iter().find(|ff| &ff.name == name)
                            {
                                let params = target.params.len() as isize;
                                let ret = target.rets.len() as isize;
                                Ok(-params + ret)
                            } else {
                                Err(format!("unknown call '{}' during stack analysis", name))
                            }
                        }
                        LirInstr::Ret => Ok(-1),
                        LirInstr::Jump(_) => Ok(0),
                        LirInstr::CondJump { .. } => Ok(-1),
                        LirInstr::Drop(_) => Ok(-1),
                        LirInstr::Nop => Ok(0),
                    }
                };

                while let Some(i) = q.pop_front() {
                    let depth = incoming[i].unwrap();
                    let delta = stack_delta(i)?;
                    let outgoing = if delta < 0 {
                        depth
                            .checked_sub(delta.unsigned_abs())
                            .ok_or("stack underflow in analysis".to_string())?
                    } else {
                        depth + (delta as usize)
                    };

                    let succs: Vec<usize> = match &body[i] {
                        LirInstr::Jump(target) => vec![*target],
                        LirInstr::CondJump { if_true, if_false } => vec![*if_true, *if_false],
                        LirInstr::Ret => vec![],
                        _ => {
                            if i + 1 < n {
                                vec![i + 1]
                            } else {
                                vec![]
                            }
                        }
                    };

                    for s in succs {
                        if s >= n {
                            return Err("invalid successor in stack analysis".into());
                        }
                        match incoming[s] {
                            None => {
                                incoming[s] = Some(outgoing);
                                q.push_back(s);
                            }
                            Some(existing) => {
                                if existing != outgoing {
                                    return Err(
                                        "inconsistent stack heights across control-flow edges"
                                            .into(),
                                    );
                                }
                            }
                        }
                    }
                }

                // Create blocks and also synthesize per-block slot parameters
                // for slots that are live-in to each instruction. We compute
                // slot liveness so stored values that are read across blocks
                // are passed via block parameters and assigned to variables
                // on entry.
                use std::collections::HashSet;

                let body = &f.body;
                let n = body.len();

                // Build use/def sets per instruction for slots
                let mut use_set: Vec<HashSet<u32>> = vec![HashSet::new(); n];
                let mut def_set: Vec<HashSet<u32>> = vec![HashSet::new(); n];
                for (i, instr) in body.iter().enumerate() {
                    match instr {
                        LirInstr::Load(slot) | LirInstr::Drop(slot) => {
                            use_set[i].insert(*slot);
                        }
                        LirInstr::Store(slot) => {
                            def_set[i].insert(*slot);
                        }
                        _ => {}
                    }
                }

                // Liveness analysis for slots (backward dataflow)
                let mut live_in: Vec<HashSet<u32>> = vec![HashSet::new(); n];
                let mut live_out: Vec<HashSet<u32>> = vec![HashSet::new(); n];
                let mut changed = true;
                while changed {
                    changed = false;
                    for i in (0..n).rev() {
                        // successors
                        let succs: Vec<usize> = match &body[i] {
                            LirInstr::Jump(target) => vec![*target],
                            LirInstr::CondJump { if_true, if_false } => vec![*if_true, *if_false],
                            LirInstr::Ret => vec![],
                            _ => {
                                if i + 1 < n {
                                    vec![i + 1]
                                } else {
                                    vec![]
                                }
                            }
                        };

                        // out = union succs in
                        let mut out: HashSet<u32> = HashSet::new();
                        for s in succs.iter() {
                            for v in &live_in[*s] {
                                out.insert(*v);
                            }
                        }

                        if out != live_out[i] {
                            live_out[i] = out.clone();
                            changed = true;
                        }

                        // in = use U (out - def)
                        let mut in_set = live_out[i].clone();
                        for d in &def_set[i] {
                            in_set.remove(d);
                        }
                        for u in &use_set[i] {
                            in_set.insert(*u);
                        }

                        if in_set != live_in[i] {
                            live_in[i] = in_set;
                            changed = true;
                        }
                    }
                }

                // slot params per block: deterministic ordering
                let mut slot_params_by_block: Vec<Vec<u32>> = vec![Vec::new(); n];
                for i in 0..n {
                    let mut v: Vec<u32> = live_in[i].iter().cloned().collect();
                    v.sort_unstable();
                    slot_params_by_block[i] = v;
                }

                // Create blocks
                let mut blocks: Vec<Block> = Vec::new();
                for _ in 0..n {
                    blocks.push(builder.create_block());
                }

                // Append stack params then slot params per block
                for i in 0..n {
                    if let Some(d) = incoming[i] {
                        for _ in 0..d {
                            builder.append_block_param(blocks[i], types::I64);
                        }
                    }
                    for _ in &slot_params_by_block[i] {
                        builder.append_block_param(blocks[i], types::I64);
                    }
                }

                for (idx, instr) in body.iter().enumerate() {
                    let block = blocks[idx];
                    builder.switch_to_block(block);

                    let params = builder.block_params(block).to_vec();
                    let stack_params = incoming[idx].unwrap_or(0);
                    // initial vstack contains only the stack params
                    let mut vstack: Vec<Value> = params[..stack_params].to_vec();

                    // bind slot params to variables
                    for (k, slot) in slot_params_by_block[idx].iter().enumerate() {
                        let param_val = params[stack_params + k];
                        let var = *var_map
                            .get(slot)
                            .ok_or_else(|| "invalid slot".to_string())?;
                        builder.def_var(var, param_val);
                    }

                    match instr {
                        LirInstr::Const(v) => {
                            let val = builder.ins().iconst(types::I64, *v);
                            vstack.push(val);
                            if idx + 1 < n {
                                let td = incoming[idx + 1].unwrap_or(0);
                                let start = vstack.len().saturating_sub(td);
                                let mut args: Vec<Value> = vstack[start..].to_vec();
                                // append slot args for successor
                                for slot in &slot_params_by_block[idx + 1] {
                                    let var = *var_map
                                        .get(slot)
                                        .ok_or_else(|| "invalid slot".to_string())?;
                                    args.push(builder.use_var(var));
                                }
                                builder.ins().jump(blocks[idx + 1], &args);
                            }
                        }
                        LirInstr::Add => {
                            let b = vstack.pop().ok_or("stack underflow during lowering")?;
                            let a = vstack.pop().ok_or("stack underflow during lowering")?;
                            let r = builder.ins().iadd(a, b);
                            vstack.push(r);
                            if idx + 1 < n {
                                let td = incoming[idx + 1].unwrap_or(0);
                                let start = vstack.len().saturating_sub(td);
                                let mut args: Vec<Value> = vstack[start..].to_vec();
                                for slot in &slot_params_by_block[idx + 1] {
                                    let var = *var_map
                                        .get(slot)
                                        .ok_or_else(|| "invalid slot".to_string())?;
                                    args.push(builder.use_var(var));
                                }
                                builder.ins().jump(blocks[idx + 1], &args);
                            }
                        }
                        LirInstr::Sub => {
                            let b = vstack.pop().ok_or("stack underflow during lowering")?;
                            let a = vstack.pop().ok_or("stack underflow during lowering")?;
                            let r = builder.ins().isub(a, b);
                            vstack.push(r);
                            if idx + 1 < n {
                                let td = incoming[idx + 1].unwrap_or(0);
                                let start = vstack.len().saturating_sub(td);
                                let mut args: Vec<Value> = vstack[start..].to_vec();
                                for slot in &slot_params_by_block[idx + 1] {
                                    let var = *var_map
                                        .get(slot)
                                        .ok_or_else(|| "invalid slot".to_string())?;
                                    args.push(builder.use_var(var));
                                }
                                builder.ins().jump(blocks[idx + 1], &args);
                            }
                        }
                        LirInstr::Mul => {
                            let b = vstack.pop().ok_or("stack underflow during lowering")?;
                            let a = vstack.pop().ok_or("stack underflow during lowering")?;
                            let r = builder.ins().imul(a, b);
                            vstack.push(r);
                            if idx + 1 < n {
                                let td = incoming[idx + 1].unwrap_or(0);
                                let start = vstack.len().saturating_sub(td);
                                let mut args: Vec<Value> = vstack[start..].to_vec();
                                for slot in &slot_params_by_block[idx + 1] {
                                    let var = *var_map
                                        .get(slot)
                                        .ok_or_else(|| "invalid slot".to_string())?;
                                    args.push(builder.use_var(var));
                                }
                                builder.ins().jump(blocks[idx + 1], &args);
                            }
                        }
                        LirInstr::Div => {
                            let b = vstack.pop().ok_or("stack underflow during lowering")?;
                            let a = vstack.pop().ok_or("stack underflow during lowering")?;
                            let r = builder.ins().sdiv(a, b);
                            vstack.push(r);
                            if idx + 1 < n {
                                let td = incoming[idx + 1].unwrap_or(0);
                                let start = vstack.len().saturating_sub(td);
                                let mut args: Vec<Value> = vstack[start..].to_vec();
                                for slot in &slot_params_by_block[idx + 1] {
                                    let var = *var_map
                                        .get(slot)
                                        .ok_or_else(|| "invalid slot".to_string())?;
                                    args.push(builder.use_var(var));
                                }
                                builder.ins().jump(blocks[idx + 1], &args);
                            }
                        }
                        LirInstr::Load(slot) => {
                            let v = *var_map
                                .get(slot)
                                .ok_or_else(|| "invalid slot".to_string())?;
                            let val = builder.use_var(v);
                            vstack.push(val);
                            if idx + 1 < n {
                                let td = incoming[idx + 1].unwrap_or(0);
                                let start = vstack.len().saturating_sub(td);
                                let mut args: Vec<Value> = vstack[start..].to_vec();
                                for slot in &slot_params_by_block[idx + 1] {
                                    let var = *var_map
                                        .get(slot)
                                        .ok_or_else(|| "invalid slot".to_string())?;
                                    args.push(builder.use_var(var));
                                }
                                builder.ins().jump(blocks[idx + 1], &args);
                            }
                        }
                        LirInstr::Store(slot) => {
                            let val = vstack.pop().ok_or("stack underflow during lowering")?;
                            let var = *var_map
                                .get(slot)
                                .ok_or_else(|| "invalid slot".to_string())?;
                            builder.def_var(var, val);
                            if idx + 1 < n {
                                let td = incoming[idx + 1].unwrap_or(0);
                                let start = vstack.len().saturating_sub(td);
                                let mut args: Vec<Value> = vstack[start..].to_vec();
                                for slot in &slot_params_by_block[idx + 1] {
                                    let var = *var_map
                                        .get(slot)
                                        .ok_or_else(|| "invalid slot".to_string())?;
                                    args.push(builder.use_var(var));
                                }
                                builder.ins().jump(blocks[idx + 1], &args);
                            }
                        }
                        LirInstr::Call(name) => {
                            if name == "print" {
                                let arg = vstack.pop().ok_or("stack underflow during lowering")?;
                                let print_id = *func_id_map
                                    .get("print")
                                    .ok_or_else(|| "print not declared".to_string())?;
                                let callee = jit.declare_func_in_func(print_id, builder.func);
                                builder.ins().call(callee, &[arg]);
                            } else if let Some(callee_id) = func_id_map.get(name) {
                                let target = module
                                    .functions
                                    .iter()
                                    .find(|ff| &ff.name == name)
                                    .ok_or_else(|| "callee signature missing".to_string())?;
                                let mut args: Vec<Value> = Vec::new();
                                for _ in 0..target.params.len() {
                                    args.push(
                                        vstack
                                            .pop()
                                            .ok_or("stack underflow during call lowering")?,
                                    );
                                }
                                args.reverse();
                                let callee_ref =
                                    jit.declare_func_in_func(*callee_id, builder.func);
                                let call_inst = builder.ins().call(callee_ref, &args);
                                // push all returned values
                                let call_results = builder.inst_results(call_inst);
                                for rv in call_results {
                                    vstack.push(*rv);
                                }
                            } else {
                                return Err(format!(
                                    "unsupported call '{}' in Cranelift backend",
                                    name
                                ));
                            }

                            if idx + 1 < n {
                                let td = incoming[idx + 1].unwrap_or(0);
                                let start = vstack.len().saturating_sub(td);
                                let mut args: Vec<Value> = vstack[start..].to_vec();
                                for slot in &slot_params_by_block[idx + 1] {
                                    let var = *var_map
                                        .get(slot)
                                        .ok_or_else(|| "invalid slot".to_string())?;
                                    args.push(builder.use_var(var));
                                }
                                builder.ins().jump(blocks[idx + 1], &args);
                            }
                        }
                        LirInstr::Ret => {
                            let ret_count = f.rets.len();
                            let mut ret_vals: Vec<Value> = Vec::new();
                            for _ in 0..ret_count {
                                let val =
                                    vstack.pop().unwrap_or(builder.ins().iconst(types::I64, 0));
                                ret_vals.push(val);
                            }
                            ret_vals.reverse();
                            builder.ins().return_(&ret_vals);
                        }
                        LirInstr::Jump(target) => {
                            if *target >= n {
                                return Err("invalid jump target".into());
                            }
                            let td = incoming[*target].unwrap_or(0);
                            let start = vstack.len().saturating_sub(td);
                            let mut args: Vec<Value> = vstack[start..].to_vec();
                            for slot in &slot_params_by_block[*target] {
                                let var = *var_map
                                    .get(slot)
                                    .ok_or_else(|| "invalid slot".to_string())?;
                                args.push(builder.use_var(var));
                            }
                            builder.ins().jump(blocks[*target], &args);
                        }
                        LirInstr::CondJump { if_true, if_false } => {
                            let cond = vstack.pop().ok_or("stack underflow during lowering")?;
                            let td_t = incoming[*if_true].unwrap_or(0);
                            let td_f = incoming[*if_false].unwrap_or(0);
                            if td_t != td_f {
                                return Err(
                                    "mismatched incoming stack heights for branch targets".into()
                                );
                            }
                            let start = vstack.len().saturating_sub(td_t);
                            let stack_args = vstack[start..].to_vec();
                            // build true args
                            let mut true_args = stack_args.clone();
                            for slot in &slot_params_by_block[*if_true] {
                                let var = *var_map
                                    .get(slot)
                                    .ok_or_else(|| "invalid slot".to_string())?;
                                true_args.push(builder.use_var(var));
                            }
                            let mut false_args = stack_args;
                            for slot in &slot_params_by_block[*if_false] {
                                let var = *var_map
                                    .get(slot)
                                    .ok_or_else(|| "invalid slot".to_string())?;
                                false_args.push(builder.use_var(var));
                            }
                            builder
                                .ins()
                                .brnz(cond, blocks[*if_true], &true_args);
                            builder.ins().jump(blocks[*if_false], &false_args);
                        }
                        LirInstr::Drop(_) | LirInstr::Nop => {
                            if idx + 1 < n {
                                let td = incoming[idx + 1].unwrap_or(0);
                                let start = vstack.len().saturating_sub(td);
                                let mut args: Vec<Value> = vstack[start..].to_vec();
                                for slot in &slot_params_by_block[idx + 1] {
                                    let var = *var_map
                                        .get(slot)
                                        .ok_or_else(|| "invalid slot".to_string())?;
                                    args.push(builder.use_var(var));
                                }
                                builder.ins().jump(blocks[idx + 1], &args);
                            }
                        }
                    }

                    builder.seal_block(block);
                }
            }

            builder.seal_all_blocks();
            builder.finalize();

            jit.define_function(func_id, &mut ctx)
                .map_err(|e| e.to_string())?;
            jit.clear_context(&mut ctx);
        }

        // If the entry function returns multiple values, synthesize a
        // thin wrapper that accepts a pointer to an i64 buffer and
        // writes the callee returns into that buffer. This allows the
        // host to call the wrapper and read back multiple return values.
        let entry_name = module
            .functions
            .iter()
            .find(|f| f.name == "main")
            .map(|f| f.name.clone())
            .unwrap_or_else(|| module.functions[0].name.clone());
        let entry_id = *func_id_map
            .get(&entry_name)
            .ok_or_else(|| "entry function not found".to_string())?;
        let entry_func = module
            .functions
            .iter()
            .find(|ff| ff.name == entry_name)
            .ok_or_else(|| "entry function missing".to_string())?;
        let entry_retcnt = entry_func.rets.len();

        // Synthesize wrapper if multiple returns
        if entry_retcnt > 1 {
            let wrapper_name = format!("__entry_wrapper_{}", entry_name);
            let mut sig_w = jit.make_signature();
            // one pointer param (i64) for the result buffer
            sig_w.params.push(AbiParam::new(types::I64));
            let wrapper_id = jit
                .declare_function(&wrapper_name, Linkage::Local, &sig_w)
                .map_err(|e| e.to_string())?;
            func_id_map.insert(wrapper_name.clone(), wrapper_id);
            sig_map.insert(wrapper_name.clone(), sig_w.clone());

            // Lower the wrapper: call the entry function and store results into the provided buffer.
            let mut ctx_w = jit.make_context();
            ctx_w.func.signature = sig_w;
            let mut func_ctx_w = FunctionBuilderContext::new();
            let mut builder_w = FunctionBuilder::new(&mut ctx_w.func, &mut func_ctx_w);

            let block = builder_w.create_block();
            builder_w.append_block_param(block, types::I64); // result buffer pointer
            builder_w.switch_to_block(block);
            builder_w.seal_block(block);

            let params = builder_w.block_params(block).to_vec();
            let buf_ptr = params[0];

            // Call the original entry function
            let callee = jit.declare_func_in_func(entry_id, builder_w.func);
            let call_inst = builder_w.ins().call(callee, &[]);
            let results_vals: Vec<Value> = builder_w.inst_results(call_inst).to_vec();

            // Store each returned value at buf_ptr + i*8
            for (i, &val) in results_vals.iter().enumerate() {
                let off = (i as i64) * 8;
                let addr = builder_w.ins().iadd_imm(buf_ptr, off);
                builder_w
                    .ins()
                    .store(cranelift_codegen::ir::MemFlags::new(), val, addr, 0);
            }

            builder_w.ins().return_(&[]);
            builder_w.seal_all_blocks();
            builder_w.finalize();

            jit.define_function(wrapper_id, &mut ctx_w)
                .map_err(|e| e.to_string())?;
            jit.clear_context(&mut ctx_w);
        }

        // Finalize code and run `main` (first or named main)
        jit.finalize_definitions();

        let code = jit.get_finalized_function(entry_id);

        if entry_retcnt == 1 {
            let f: extern "C" fn() -> i64 = unsafe { std::mem::transmute(code) };
            let res = f();
            Ok(vec![res])
        } else if entry_retcnt == 0 {
            let f: extern "C" fn() = unsafe { std::mem::transmute(code) };
            f();
            Ok(Vec::new())
        } else {
            // Call the generated wrapper with a host-allocated buffer.
            let wrapper_name = format!("__entry_wrapper_{}", entry_name);
            let wrapper_id = *func_id_map
                .get(&wrapper_name)
                .ok_or_else(|| "wrapper missing".to_string())?;
            let wrapper_code = jit.get_finalized_function(wrapper_id);
            let mut outbuf: Vec<i64> = vec![0; entry_retcnt];
            let ptr = outbuf.as_mut_ptr() as i64;
            let wrapper_fn: extern "C" fn(i64) = unsafe { std::mem::transmute(wrapper_code) };
            wrapper_fn(ptr);
            Ok(outbuf)
        }
    }
}

pub use cranelift_backend::compile_and_run_with_jit;
