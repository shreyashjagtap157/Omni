// Optional Cranelift development/oracle backend. It is not canonical Omni execution.
use lir::{Instr as LirInstr, Module};
use std::collections::HashMap;

/// Textual LIR renderer for debugging.
pub fn render_lir_text(module: &Module) -> String {
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

/// Deterministic, dependency-free LIR reference executor for the scalar
/// qualified scalar subset. This is a development oracle only; canonical Omni programs
/// are emitted by the owned native AOT backend.
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
                    let b = stack.pop().ok_or("stack underflow in add")?;
                    let a = stack.pop().ok_or("stack underflow in add")?;
                    stack.push(a.checked_add(b).ok_or("checked integer overflow in add")?);
                }
                LirInstr::Sub => {
                    let b = stack.pop().ok_or("stack underflow in sub")?;
                    let a = stack.pop().ok_or("stack underflow in sub")?;
                    stack.push(a.checked_sub(b).ok_or("checked integer overflow in sub")?);
                }
                LirInstr::Mul => {
                    let b = stack.pop().ok_or("stack underflow in mul")?;
                    let a = stack.pop().ok_or("stack underflow in mul")?;
                    stack.push(a.checked_mul(b).ok_or("checked integer overflow in mul")?);
                }
                LirInstr::Div => {
                    let b = stack.pop().ok_or("stack underflow in div")?;
                    let a = stack.pop().ok_or("stack underflow in div")?;
                    if b == 0 {
                        return Err("division by zero".to_string());
                    }
                    stack.push(a.checked_div(b).ok_or("checked integer overflow in div")?);
                }
                LirInstr::Mod => {
                    let b = stack.pop().ok_or("stack underflow in mod")?;
                    let a = stack.pop().ok_or("stack underflow in mod")?;
                    if b == 0 {
                        return Err("remainder by zero".to_string());
                    }
                    stack.push(a.checked_rem(b).ok_or("checked integer overflow in mod")?);
                }
                LirInstr::Lt => {
                    let b = stack.pop().ok_or("stack underflow in lt")?;
                    let a = stack.pop().ok_or("stack underflow in lt")?;
                    stack.push(if a < b { 1 } else { 0 });
                }
                LirInstr::Gt => {
                    let b = stack.pop().ok_or("stack underflow in gt")?;
                    let a = stack.pop().ok_or("stack underflow in gt")?;
                    stack.push(if a > b { 1 } else { 0 });
                }
                LirInstr::Le => {
                    let b = stack.pop().ok_or("stack underflow in le")?;
                    let a = stack.pop().ok_or("stack underflow in le")?;
                    stack.push(if a <= b { 1 } else { 0 });
                }
                LirInstr::Ge => {
                    let b = stack.pop().ok_or("stack underflow in ge")?;
                    let a = stack.pop().ok_or("stack underflow in ge")?;
                    stack.push(if a >= b { 1 } else { 0 });
                }
                LirInstr::Eq => {
                    let b = stack.pop().ok_or("stack underflow in eq")?;
                    let a = stack.pop().ok_or("stack underflow in eq")?;
                    stack.push(if a == b { 1 } else { 0 });
                }
                LirInstr::Ne => {
                    let b = stack.pop().ok_or("stack underflow in ne")?;
                    let a = stack.pop().ok_or("stack underflow in ne")?;
                    stack.push(if a != b { 1 } else { 0 });
                }
                LirInstr::Not => {
                    let a = stack.pop().ok_or("stack underflow in not")?;
                    stack.push(if a == 0 { 1 } else { 0 });
                }
                LirInstr::Load(slot) => {
                    let v = *locals
                        .get(slot)
                        .ok_or_else(|| format!("read of uninitialized local slot {}", slot))?;
                    stack.push(v);
                }
                LirInstr::Store(slot) => {
                    let v = stack.pop().ok_or("stack underflow in store")?;
                    locals.insert(*slot, v);
                }
                LirInstr::Call(name) => {
                    if name == "print" {
                        let v = stack.pop().ok_or("stack underflow in print")?;
                        prints.push(format!("{}", v));
                    } else if name == "__register_effect_handler" {
                        return Err("effect-handler runtime is not implemented in the Cranelift development oracle".to_string());
                    } else {
                        let callee_idx = *name_map
                            .get(name)
                            .ok_or_else(|| format!("unknown function '{}'", name))?;
                        let target = &module.functions[callee_idx];
                        let mut cargs: Vec<i64> = Vec::new();
                        for _ in 0..target.params.len() {
                            cargs.push(stack.pop().ok_or_else(|| {
                                format!("stack underflow preparing call to '{}'", name)
                            })?);
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
                        rets.push(
                            stack
                                .pop()
                                .ok_or("stack underflow returning from function")?,
                        );
                    }
                    rets.reverse();
                    return Ok(rets);
                }
                LirInstr::Jump(target) => {
                    ip = *target;
                    continue;
                }
                LirInstr::CondJump { if_true, if_false } => {
                    let cond = stack.pop().ok_or("stack underflow in conditional jump")?;
                    ip = if cond != 0 { *if_true } else { *if_false };
                    continue;
                }
                LirInstr::PrintStr(s) => {
                    prints.push(s.clone());
                }
                LirInstr::Drop(slot) => {
                    locals.remove(slot);
                }
                LirInstr::Nop => {}
                LirInstr::StringRef(_)
                | LirInstr::BytesRef(_)
                | LirInstr::PrintBytes
                | LirInstr::LoadByteIndex
                | LirInstr::LoadOffset(_, _)
                | LirInstr::BoundsCheck(_)
                | LirInstr::LoadIndex { .. }
                | LirInstr::StoreOffset(_, _)
                | LirInstr::GetAddr(_)
                | LirInstr::LoadPtrOffset(_, _)
                | LirInstr::StorePtrOffset(_, _)
                | LirInstr::AddOffset
                | LirInstr::LoadInd
                | LirInstr::StoreInd => {
                    return Err("aggregate/pointer LIR is not implemented in the Cranelift scalar development oracle".to_string());
                }
            }
            ip += 1;
        }

        if f.rets.is_empty() {
            Ok(Vec::new())
        } else {
            Err(format!(
                "function '{}' reached end without required return value",
                f.name
            ))
        }
    }

    let entry_idx = module
        .functions
        .iter()
        .position(|f| f.name == "main")
        .ok_or_else(|| "LIR module has no 'main' entry function".to_string())?;
    let rets = interp_fn(module, &name_map, entry_idx, Vec::new(), &mut prints)?;
    Ok(RunResult {
        return_values: rets,
        prints,
    })
}

/// The historical Cranelift JIT is intentionally unavailable in the remediated
/// v0.1.4 baseline. Its archived implementation did not preserve all Omni
/// checked-arithmetic and pointer semantics. Re-enabling it requires a later
/// differential-conformance milestone.
pub fn compile_and_run_with_jit(_module: &Module) -> Result<Vec<i64>, String> {
    Err("Cranelift JIT execution is not qualified in Omni v0.1.4; use the owned native AOT backend or the scalar LIR reference executor".to_string())
}
