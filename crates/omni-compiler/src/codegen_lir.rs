use crate::mir;
use lir::{Function as LirFunction, Instr as LirInstr, Module as LirModule, Type as LirType};
use std::collections::HashMap;

/// Lower a MIR module to a minimal stack-based LIR Module.
pub fn lower_mir_to_lir(m: &mir::MirModule) -> LirModule {
    let mut out = LirModule::new();

    for func in &m.functions {
        let mut var_slots: HashMap<String, u32> = HashMap::new();
        let mut next_slot: u32 = 0;
        let mut saw_return = false;

        // First pass: collect variable names
        for block in &func.blocks {
            for instr in &block.instrs {
                match instr {
                    mir::Instruction::ConstInt { dest, .. }
                    | mir::Instruction::ConstStr { dest, .. }
                    | mir::Instruction::ConstBool { dest, .. }
                    | mir::Instruction::LinearMove { dest, .. }
                    | mir::Instruction::BinaryOp { dest, .. }
                    | mir::Instruction::UnaryOp { dest, .. }
                    | mir::Instruction::Assign { dest, .. }
                    | mir::Instruction::Call { dest, .. }
                    | mir::Instruction::FieldAccess { dest, .. }
                    | mir::Instruction::StructAccess { dest, .. } => {
                        if !var_slots.contains_key(dest) {
                            var_slots.insert(dest.clone(), next_slot);
                            next_slot += 1;
                        }
                    }
                    mir::Instruction::IndexAccess { dest, .. } if !var_slots.contains_key(dest) => {
                        var_slots.insert(dest.clone(), next_slot);
                        next_slot += 1;
                    }
                    mir::Instruction::IndexAccess { .. } => {}
                    mir::Instruction::Drop { var }
                    | mir::Instruction::DropLinear { var }
                    | mir::Instruction::Print { src: var }
                    | mir::Instruction::Return { value: var }
                    | mir::Instruction::Move { dest: _, src: var } => {
                        if !is_numeric_literal(var) && !var_slots.contains_key(var) {
                            var_slots.insert(var.clone(), next_slot);
                            next_slot += 1;
                        }
                    }
                    mir::Instruction::JumpIf { cond: var, .. }
                        if !is_numeric_literal(var) && !var_slots.contains_key(var) =>
                    {
                        var_slots.insert(var.clone(), next_slot);
                        next_slot += 1;
                    }
                    mir::Instruction::JumpIf { .. } => {}
                    _ => {}
                }
            }
        }

        // Build labels mapping
        let mut lir_instrs: Vec<LirInstr> = Vec::new();
        let mut label_map: HashMap<usize, usize> = HashMap::new();
        let mut jump_patches: Vec<(usize, usize)> = Vec::new(); // (lir_idx, mir_target)
        let mut cond_patches: Vec<(usize, usize)> = Vec::new(); // (lir_idx, mir_target)

        for block in &func.blocks {
            for instr in &block.instrs {
                match instr {
                    mir::Instruction::ConstInt { dest, value } => {
                        lir_instrs.push(LirInstr::Const(*value));
                        let slot = var_slots.get(dest).copied().unwrap_or_else(|| {
                            let id = next_slot;
                            next_slot += 1;
                            var_slots.insert(dest.clone(), id);
                            id
                        });
                        lir_instrs.push(LirInstr::Store(slot));
                    }
                    mir::Instruction::ConstBool { dest, value } => {
                        lir_instrs.push(LirInstr::Const(if *value { 1 } else { 0 }));
                        let slot = var_slots.get(dest).copied().unwrap_or_else(|| {
                            let id = next_slot;
                            next_slot += 1;
                            var_slots.insert(dest.clone(), id);
                            id
                        });
                        lir_instrs.push(LirInstr::Store(slot));
                    }
                    mir::Instruction::ConstStr { dest, .. } => {
                        // Strings unsupported in this minimal LIR; store a zero placeholder.
                        lir_instrs.push(LirInstr::Const(0));
                        let slot = var_slots.get(dest).copied().unwrap_or_else(|| {
                            let id = next_slot;
                            next_slot += 1;
                            var_slots.insert(dest.clone(), id);
                            id
                        });
                        lir_instrs.push(LirInstr::Store(slot));
                    }
                    mir::Instruction::Move { dest, src }
                    | mir::Instruction::Assign { dest, src } => {
                        if is_numeric_literal(src) {
                            let v = src.parse::<i64>().unwrap_or(0);
                            lir_instrs.push(LirInstr::Const(v));
                        } else {
                            let s = *var_slots.get(src).unwrap_or(&0);
                            lir_instrs.push(LirInstr::Load(s));
                        }
                        let slot = *var_slots.get(dest).unwrap_or(&0);
                        lir_instrs.push(LirInstr::Store(slot));
                    }
                    mir::Instruction::BinaryOp {
                        dest,
                        op,
                        left,
                        right,
                    } => {
                        if is_numeric_literal(left) {
                            lir_instrs.push(LirInstr::Const(left.parse().unwrap_or(0)));
                        } else {
                            let s = *var_slots.get(left).unwrap_or(&0);
                            lir_instrs.push(LirInstr::Load(s));
                        }
                        if is_numeric_literal(right) {
                            lir_instrs.push(LirInstr::Const(right.parse().unwrap_or(0)));
                        } else {
                            let s = *var_slots.get(right).unwrap_or(&0);
                            lir_instrs.push(LirInstr::Load(s));
                        }
                        // map op
                        use crate::lexer::TokenKind;
                        match op {
                            TokenKind::Plus => lir_instrs.push(LirInstr::Add),
                            TokenKind::Minus => lir_instrs.push(LirInstr::Sub),
                            TokenKind::Star => lir_instrs.push(LirInstr::Mul),
                            TokenKind::Slash => lir_instrs.push(LirInstr::Div),
                            _ => lir_instrs.push(LirInstr::Nop),
                        }
                        let slot = *var_slots.get(dest).unwrap_or(&0);
                        lir_instrs.push(LirInstr::Store(slot));
                    }
                    mir::Instruction::UnaryOp { dest, op, operand } => {
                        if is_numeric_literal(operand) {
                            lir_instrs.push(LirInstr::Const(operand.parse().unwrap_or(0)));
                        } else {
                            let s = *var_slots.get(operand).unwrap_or(&0);
                            lir_instrs.push(LirInstr::Load(s));
                        }
                        use crate::lexer::TokenKind;
                        match op {
                            TokenKind::Minus => {
                                // unary - : 0 - x
                                lir_instrs.push(LirInstr::Const(0));
                                lir_instrs.push(LirInstr::Sub);
                            }
                            _ => lir_instrs.push(LirInstr::Nop),
                        }
                        let slot = *var_slots.get(dest).unwrap_or(&0);
                        lir_instrs.push(LirInstr::Store(slot));
                    }
                    mir::Instruction::Print { src } => {
                        if is_numeric_literal(src) {
                            lir_instrs.push(LirInstr::Const(src.parse().unwrap_or(0)));
                        } else {
                            let s = *var_slots.get(src).unwrap_or(&0);
                            lir_instrs.push(LirInstr::Load(s));
                        }
                        lir_instrs.push(LirInstr::Call("print".to_string()));
                    }
                    mir::Instruction::Return { value } => {
                        if is_numeric_literal(value) {
                            lir_instrs.push(LirInstr::Const(value.parse().unwrap_or(0)));
                        } else {
                            let s = *var_slots.get(value).unwrap_or(&0);
                            lir_instrs.push(LirInstr::Load(s));
                        }
                        lir_instrs.push(LirInstr::Ret);
                        saw_return = true;
                    }
                    mir::Instruction::Call { dest, func, args } => {
                        for a in args {
                            if is_numeric_literal(a) {
                                lir_instrs.push(LirInstr::Const(a.parse().unwrap_or(0)));
                            } else {
                                let s = *var_slots.get(a).unwrap_or(&0);
                                lir_instrs.push(LirInstr::Load(s));
                            }
                        }
                        lir_instrs.push(LirInstr::Call(func.clone()));
                        let slot = *var_slots.get(dest).unwrap_or(&0);
                        lir_instrs.push(LirInstr::Store(slot));
                    }
                    mir::Instruction::Jump { target } => {
                        let idx = lir_instrs.len();
                        lir_instrs.push(LirInstr::Jump(0));
                        jump_patches.push((idx, *target));
                    }
                    mir::Instruction::JumpIf { cond, target } => {
                        if is_numeric_literal(cond) {
                            lir_instrs.push(LirInstr::Const(cond.parse().unwrap_or(0)));
                        } else {
                            let s = *var_slots.get(cond).unwrap_or(&0);
                            lir_instrs.push(LirInstr::Load(s));
                        }
                        let idx = lir_instrs.len();
                        lir_instrs.push(LirInstr::CondJump {
                            if_true: 0,
                            if_false: 0,
                        });
                        cond_patches.push((idx, *target));
                    }
                    mir::Instruction::Label { id } => {
                        let idx = lir_instrs.len();
                        label_map.insert(*id, idx);
                        lir_instrs.push(LirInstr::Nop);
                    }
                    mir::Instruction::Drop { var } | mir::Instruction::DropLinear { var } => {
                        let s = *var_slots.get(var).unwrap_or(&0);
                        lir_instrs.push(LirInstr::Drop(s));
                    }
                    mir::Instruction::StructDef { .. } => {
                        lir_instrs.push(LirInstr::Nop);
                    }
                    _ => {
                        // Fallback for unhandled instructions
                        lir_instrs.push(LirInstr::Nop);
                    }
                }
            }
        }

        // Patch jumps and conds using label_map
        for (lir_idx, mir_target) in jump_patches {
            let target = *label_map.get(&mir_target).unwrap_or(&0);
            if lir_idx < lir_instrs.len() {
                lir_instrs[lir_idx] = LirInstr::Jump(target);
            }
        }
        for (lir_idx, mir_target) in cond_patches {
            let target = *label_map.get(&mir_target).unwrap_or(&0);
            let fallthrough = lir_idx + 1;
            if lir_idx < lir_instrs.len() {
                lir_instrs[lir_idx] = LirInstr::CondJump {
                    if_true: target,
                    if_false: fallthrough,
                };
            }
        }

        if !saw_return {
            lir_instrs.push(LirInstr::Const(0));
            lir_instrs.push(LirInstr::Ret);
        }

        let lir_func = LirFunction::new(func.name.clone(), vec![], LirType::I64, lir_instrs);
        out.add_function(lir_func);
    }

    out
}

fn is_numeric_literal(s: &str) -> bool {
    s.parse::<i64>().is_ok()
}

/// Helper to produce a textual compiled representation via the codegen stub.
pub fn compile_lir_module_text(module: &LirModule) -> String {
    codegen_cranelift::compile_lir_stub(module)
}
