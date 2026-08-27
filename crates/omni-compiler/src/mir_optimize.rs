use crate::complete_lexer::TokenKind;
use crate::mir::{BasicBlock, Instruction, MirFunction, MirModule};
use std::collections::HashMap;

#[derive(Debug, Clone)]
enum ConstVal {
    Int(i64),
    Str(String),
    Bytes(Vec<u8>),
    Bool(bool),
}

pub fn run_mir_optimizations(module: &mut MirModule) {
    constant_fold_module(module);
    dce_module(module);
    inline_simple_functions(module);
}

pub fn constant_fold_module(module: &mut MirModule) {
    for func in &mut module.functions {
        constant_fold_function(func);
    }
}

pub fn constant_fold_function(func: &mut MirFunction) {
    for block in &mut func.blocks {
        constant_fold_block(block);
    }
}

pub fn constant_fold_block(block: &mut BasicBlock) {
    let mut consts: HashMap<String, ConstVal> = HashMap::new();
    let mut new_instrs: Vec<Instruction> = Vec::with_capacity(block.instrs.len());

    for instr in std::mem::take(&mut block.instrs) {
        match instr {
            Instruction::ConstInt { dest, value } => {
                consts.insert(dest.clone(), ConstVal::Int(value));
                new_instrs.push(Instruction::ConstInt { dest, value });
            }
            Instruction::ConstStr { dest, value } => {
                consts.insert(dest.clone(), ConstVal::Str(value.clone()));
                new_instrs.push(Instruction::ConstStr { dest, value });
            }
            Instruction::ConstBytes { dest, value } => {
                consts.insert(dest.clone(), ConstVal::Bytes(value.clone()));
                new_instrs.push(Instruction::ConstBytes { dest, value });
            }
            Instruction::ConstBool { dest, value } => {
                consts.insert(dest.clone(), ConstVal::Bool(value));
                new_instrs.push(Instruction::ConstBool { dest, value });
            }
            Instruction::BinaryOp {
                dest,
                op,
                left,
                right,
            } => {
                let lconst = consts.get(&left).cloned();
                let rconst = consts.get(&right).cloned();
                let folded = match (lconst, rconst) {
                    (Some(ConstVal::Int(a)), Some(ConstVal::Int(b))) => match &op {
                        TokenKind::Plus => a.checked_add(b).map(ConstVal::Int),
                        TokenKind::Minus => a.checked_sub(b).map(ConstVal::Int),
                        TokenKind::Star => a.checked_mul(b).map(ConstVal::Int),
                        TokenKind::Slash => a.checked_div(b).map(ConstVal::Int),
                        TokenKind::Percent => a.checked_rem(b).map(ConstVal::Int),
                        TokenKind::EqEq => Some(ConstVal::Bool(a == b)),
                        TokenKind::NotEq => Some(ConstVal::Bool(a != b)),
                        TokenKind::Lt => Some(ConstVal::Bool(a < b)),
                        TokenKind::LtEq => Some(ConstVal::Bool(a <= b)),
                        TokenKind::Gt => Some(ConstVal::Bool(a > b)),
                        TokenKind::GtEq => Some(ConstVal::Bool(a >= b)),
                        _ => None,
                    },
                    (Some(ConstVal::Str(a)), Some(ConstVal::Str(b))) => match &op {
                        TokenKind::Plus => Some(ConstVal::Str(format!("{}{}", a, b))),
                        TokenKind::EqEq => Some(ConstVal::Bool(a == b)),
                        TokenKind::NotEq => Some(ConstVal::Bool(a != b)),
                        _ => None,
                    },
                    (Some(ConstVal::Bool(a)), Some(ConstVal::Bool(b))) => match &op {
                        TokenKind::EqEq => Some(ConstVal::Bool(a == b)),
                        TokenKind::NotEq => Some(ConstVal::Bool(a != b)),
                        _ => None,
                    },
                    _ => None,
                };

                if let Some(value) = folded {
                    consts.insert(dest.clone(), value.clone());
                    match value {
                        ConstVal::Int(value) => {
                            new_instrs.push(Instruction::ConstInt { dest, value });
                        }
                        ConstVal::Str(value) => {
                            new_instrs.push(Instruction::ConstStr { dest, value });
                        }
                        ConstVal::Bytes(value) => {
                            new_instrs.push(Instruction::ConstBytes { dest, value });
                        }
                        ConstVal::Bool(value) => {
                            new_instrs.push(Instruction::ConstBool { dest, value });
                        }
                    }
                } else {
                    // A non-folded definition invalidates any stale fact for
                    // the destination. In particular, checked arithmetic that
                    // may fault must remain observable at runtime.
                    consts.remove(&dest);
                    new_instrs.push(Instruction::BinaryOp {
                        dest,
                        op,
                        left,
                        right,
                    });
                }
            }
            Instruction::UnaryOp { dest, op, operand } => {
                let folded = match consts.get(&operand).cloned() {
                    Some(ConstVal::Int(value)) if op == TokenKind::Minus => {
                        value.checked_neg().map(ConstVal::Int)
                    }
                    _ => None,
                };
                if let Some(value) = folded {
                    consts.insert(dest.clone(), value.clone());
                    match value {
                        ConstVal::Int(value) => {
                            new_instrs.push(Instruction::ConstInt { dest, value });
                        }
                        ConstVal::Str(value) => {
                            new_instrs.push(Instruction::ConstStr { dest, value });
                        }
                        ConstVal::Bytes(value) => {
                            new_instrs.push(Instruction::ConstBytes { dest, value });
                        }
                        ConstVal::Bool(value) => {
                            new_instrs.push(Instruction::ConstBool { dest, value });
                        }
                    }
                } else {
                    consts.remove(&dest);
                    new_instrs.push(Instruction::UnaryOp { dest, op, operand });
                }
            }
            Instruction::Move { dest, src } => {
                // MIR move/assignment destinations are mutable program
                // locations, not SSA names. Do not let an older constant fact
                // survive a redefinition. Deliberately avoid propagating the
                // source fact here until ownership semantics are qualified.
                consts.remove(&dest);
                new_instrs.push(Instruction::Move { dest, src });
            }
            Instruction::LinearMove { dest, src } => {
                consts.remove(&dest);
                new_instrs.push(Instruction::LinearMove { dest, src });
            }
            Instruction::Assign { dest, src } => {
                consts.remove(&dest);
                new_instrs.push(Instruction::Assign { dest, src });
            }
            Instruction::Call { dest, func, args } => {
                // Calls are observation and memory barriers. Arguments can
                // carry references to mutable locals, and the current MIR
                // does not yet carry an effect/provenance summary that would
                // prove which locations a callee may modify. Retaining any
                // pre-call fact could therefore fold a post-call expression
                // from a stale value and change source-order meaning.
                consts.clear();
                new_instrs.push(Instruction::Call { dest, func, args });
            }
            Instruction::Spawn { func, args } => {
                // A spawned function may observe or mutate reference-reachable
                // state concurrently. Do not propagate facts across the spawn
                // until alias/effect summaries prove that doing so is safe.
                consts.clear();
                new_instrs.push(Instruction::Spawn { func, args });
            }
            Instruction::DerefAssign { reference, src } => {
                // The pointee is not encoded in the instruction destination,
                // so conservatively invalidate every fact rather than guess
                // which local the reference aliases.
                consts.clear();
                new_instrs.push(Instruction::DerefAssign { reference, src });
            }
            Instruction::FieldAssign { base, field, src } => {
                // Aggregate subobject writes are provenance-sensitive. The
                // current non-SSA fact map cannot prove that another name does
                // not observe the same allocation/subobject.
                consts.clear();
                new_instrs.push(Instruction::FieldAssign { base, field, src });
            }
            Instruction::AggregateInit {
                dest,
                type_name,
                fields,
            } => {
                consts.remove(&dest);
                new_instrs.push(Instruction::AggregateInit {
                    dest,
                    type_name,
                    fields,
                });
            }
            Instruction::EnumInit {
                dest,
                type_name,
                variant,
                tag,
                fields,
            } => {
                consts.remove(&dest);
                new_instrs.push(Instruction::EnumInit {
                    dest,
                    type_name,
                    variant,
                    tag,
                    fields,
                });
            }
            Instruction::EnumTag { dest, base } => {
                consts.remove(&dest);
                new_instrs.push(Instruction::EnumTag { dest, base });
            }
            Instruction::EnumPayloadAccess { dest, base, index } => {
                consts.remove(&dest);
                new_instrs.push(Instruction::EnumPayloadAccess { dest, base, index });
            }
            Instruction::Channel {
                dest,
                elem_type,
                capacity,
            } => {
                consts.remove(&dest);
                new_instrs.push(Instruction::Channel {
                    dest,
                    elem_type,
                    capacity,
                });
            }
            Instruction::FieldAccess {
                dest,
                base,
                field,
                linear,
            } => {
                consts.remove(&dest);
                new_instrs.push(Instruction::FieldAccess {
                    dest,
                    base,
                    field,
                    linear,
                });
            }
            Instruction::StructAccess { dest, base, field } => {
                consts.remove(&dest);
                new_instrs.push(Instruction::StructAccess { dest, base, field });
            }
            Instruction::IndexAccess { dest, base, index } => {
                consts.remove(&dest);
                new_instrs.push(Instruction::IndexAccess { dest, base, index });
            }
            Instruction::SliceAccess {
                dest,
                base,
                start,
                end,
                inclusive,
            } => {
                consts.remove(&dest);
                new_instrs.push(Instruction::SliceAccess {
                    dest,
                    base,
                    start,
                    end,
                    inclusive,
                });
            }
            Instruction::Drop { var } => {
                consts.remove(&var);
                new_instrs.push(Instruction::Drop { var });
            }
            Instruction::DropLinear { var } => {
                consts.remove(&var);
                new_instrs.push(Instruction::DropLinear { var });
            }
            Instruction::Label { id } => {
                // The current MIR stores labels/jumps inside a BasicBlock.
                // Linear propagation across a control-flow join is not valid
                // without dominance/path analysis, so conservatively forget
                // all path-local facts at every boundary.
                consts.clear();
                new_instrs.push(Instruction::Label { id });
            }
            Instruction::Jump { target } => {
                consts.clear();
                new_instrs.push(Instruction::Jump { target });
            }
            Instruction::JumpIf { cond, target } => {
                consts.clear();
                new_instrs.push(Instruction::JumpIf { cond, target });
            }
            Instruction::MatchBranch {
                cond,
                then_block,
                else_block,
            } => {
                consts.clear();
                new_instrs.push(Instruction::MatchBranch {
                    cond,
                    then_block,
                    else_block,
                });
            }
            other => new_instrs.push(other),
        }
    }

    block.instrs = new_instrs;
}

pub fn dce_module(module: &mut MirModule) {
    for func in &mut module.functions {
        dce_function(func);
    }
}

pub fn dce_function(func: &mut MirFunction) {
    use std::collections::HashSet;

    // Iteratively remove definitions whose destination is never used.
    let mut changed = true;
    while changed {
        changed = false;
        let mut used: HashSet<String> = HashSet::new();

        // Collect all used variable names across the function
        for block in &func.blocks {
            for instr in &block.instrs {
                match instr {
                    Instruction::Move { src, .. }
                    | Instruction::LinearMove { src, .. }
                    | Instruction::Print { src }
                    | Instruction::Return { value: src }
                    | Instruction::Assign { src, .. } => {
                        used.insert(src.clone());
                    }
                    Instruction::BinaryOp { left, right, .. } => {
                        used.insert(left.clone());
                        used.insert(right.clone());
                    }
                    Instruction::UnaryOp { operand, .. } => {
                        used.insert(operand.clone());
                    }
                    Instruction::JumpIf { cond, .. } => {
                        used.insert(cond.clone());
                    }
                    Instruction::Call { args, .. } => {
                        for a in args {
                            used.insert(a.clone());
                        }
                    }
                    Instruction::AggregateInit { fields, .. }
                    | Instruction::EnumInit { fields, .. } => {
                        for (_, value) in fields {
                            used.insert(value.clone());
                        }
                    }
                    Instruction::FieldAssign { base, src, .. } => {
                        used.insert(base.clone());
                        used.insert(src.clone());
                    }
                    Instruction::Borrow { place, .. } => {
                        used.insert(place.clone());
                    }
                    Instruction::Deref { reference, .. } => {
                        used.insert(reference.clone());
                    }
                    Instruction::DerefAssign { reference, src } => {
                        used.insert(reference.clone());
                        used.insert(src.clone());
                    }
                    Instruction::EnumTag { base, .. }
                    | Instruction::EnumPayloadAccess { base, .. }
                    | Instruction::FieldAccess { base, .. }
                    | Instruction::StructAccess { base, .. } => {
                        used.insert(base.clone());
                    }
                    Instruction::IndexAccess { base, index, .. } => {
                        used.insert(base.clone());
                        used.insert(index.clone());
                    }
                    Instruction::SliceAccess {
                        base, start, end, ..
                    } => {
                        used.insert(base.clone());
                        used.insert(start.clone());
                        used.insert(end.clone());
                    }
                    Instruction::Drop { var } | Instruction::DropLinear { var } => {
                        used.insert(var.clone());
                    }
                    _ => {}
                }
            }
        }

        // Remove only provably inert literal definitions whose destination is unused.
        // Checked arithmetic is intentionally retained because an unused operation can still fault.
        for block in &mut func.blocks {
            let before = block.instrs.len();
            block.instrs.retain(|instr| {
                match instr {
                    Instruction::ConstInt { dest, .. }
                    | Instruction::ConstStr { dest, .. }
                    | Instruction::ConstBytes { dest, .. }
                    | Instruction::ConstBool { dest, .. } => {
                        if used.contains(dest) {
                            true
                        } else {
                            // If this dest is not used, drop it.
                            false
                        }
                    }
                    _ => true,
                }
            });
            if block.instrs.len() != before {
                changed = true;
            }
        }
    }
}

pub fn inline_simple_functions(module: &mut MirModule) {
    use std::collections::HashMap;

    // Only inline functions whose complete optimized body is exactly
    // `const; return`. Calls, drops, assignments, and control-flow are all
    // potentially observable and must never disappear merely because a
    // function happens to return a constant.
    let mut inlinable: HashMap<String, ConstVal> = HashMap::new();

    for f in &module.functions {
        if f.blocks.len() != 1 {
            continue;
        }
        let block = &f.blocks[0];
        if block.instrs.len() != 2 {
            continue;
        }
        let ret_var = match &block.instrs[1] {
            Instruction::Return { value } => value,
            _ => continue,
        };
        let value = match &block.instrs[0] {
            Instruction::ConstInt { dest, value } if dest == ret_var => ConstVal::Int(*value),
            Instruction::ConstStr { dest, value } if dest == ret_var => {
                ConstVal::Str(value.clone())
            }
            Instruction::ConstBytes { dest, value } if dest == ret_var => {
                ConstVal::Bytes(value.clone())
            }
            Instruction::ConstBool { dest, value } if dest == ret_var => ConstVal::Bool(*value),
            _ => continue,
        };
        inlinable.insert(f.name.clone(), value);
    }

    if inlinable.is_empty() {
        return;
    }

    // Replace Call instructions with the corresponding const when safe.
    for func in &mut module.functions {
        for block in &mut func.blocks {
            let mut temp: Vec<Instruction> = Vec::new();
            while let Some(i) = block.instrs.pop() {
                temp.push(i);
            }
            temp.reverse();

            let mut new_instrs: Vec<Instruction> = Vec::new();
            for instr in temp.into_iter() {
                match instr {
                    Instruction::Call {
                        dest,
                        func: callee,
                        args,
                    } => {
                        if args.is_empty() {
                            if let Some(cv) = inlinable.get(&callee) {
                                match cv {
                                    ConstVal::Int(v) => {
                                        new_instrs.push(Instruction::ConstInt { dest, value: *v });
                                        continue;
                                    }
                                    ConstVal::Str(s) => {
                                        new_instrs.push(Instruction::ConstStr {
                                            dest,
                                            value: s.clone(),
                                        });
                                        continue;
                                    }
                                    ConstVal::Bytes(bytes) => {
                                        new_instrs.push(Instruction::ConstBytes {
                                            dest,
                                            value: bytes.clone(),
                                        });
                                        continue;
                                    }
                                    ConstVal::Bool(b) => {
                                        new_instrs.push(Instruction::ConstBool { dest, value: *b });
                                        continue;
                                    }
                                }
                            }
                        }
                        new_instrs.push(Instruction::Call {
                            dest,
                            func: callee,
                            args,
                        });
                    }
                    other => new_instrs.push(other),
                }
            }
            block.instrs = new_instrs;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_facts_do_not_survive_mutable_assignment() {
        let mut block = BasicBlock::new(0);
        block.instrs = vec![
            Instruction::ConstInt {
                dest: "x".into(),
                value: 1,
            },
            Instruction::ConstInt {
                dest: "y".into(),
                value: 40,
            },
            Instruction::Assign {
                dest: "x".into(),
                src: "y".into(),
            },
            Instruction::ConstInt {
                dest: "two".into(),
                value: 2,
            },
            Instruction::BinaryOp {
                dest: "answer".into(),
                op: TokenKind::Plus,
                left: "x".into(),
                right: "two".into(),
            },
        ];

        constant_fold_block(&mut block);
        assert!(matches!(
            block.instrs.last(),
            Some(Instruction::BinaryOp { dest, .. }) if dest == "answer"
        ));
    }

    #[test]
    fn constant_facts_do_not_cross_control_flow_boundaries() {
        let mut block = BasicBlock::new(0);
        block.instrs = vec![
            Instruction::ConstInt {
                dest: "x".into(),
                value: 21,
            },
            Instruction::Label { id: 7 },
            Instruction::BinaryOp {
                dest: "answer".into(),
                op: TokenKind::Plus,
                left: "x".into(),
                right: "x".into(),
            },
        ];

        constant_fold_block(&mut block);
        assert!(matches!(
            block.instrs.last(),
            Some(Instruction::BinaryOp { dest, .. }) if dest == "answer"
        ));
    }

    #[test]
    fn faulting_integer_arithmetic_is_not_folded_away() {
        let mut block = BasicBlock::new(0);
        block.instrs = vec![
            Instruction::ConstInt {
                dest: "max".into(),
                value: i64::MAX,
            },
            Instruction::ConstInt {
                dest: "one".into(),
                value: 1,
            },
            Instruction::BinaryOp {
                dest: "overflow".into(),
                op: TokenKind::Plus,
                left: "max".into(),
                right: "one".into(),
            },
        ];

        constant_fold_block(&mut block);
        assert!(matches!(
            block.instrs.last(),
            Some(Instruction::BinaryOp { dest, .. }) if dest == "overflow"
        ));
    }

    #[test]
    fn dce_keeps_calls_even_when_result_is_unused() {
        let mut function = MirFunction::new("caller", false);
        function.blocks.push(BasicBlock::new(0));
        function.blocks[0].instrs = vec![Instruction::Call {
            dest: "unused".into(),
            func: "effectful".into(),
            args: Vec::new(),
        }];
        dce_function(&mut function);
        assert!(matches!(
            function.blocks[0].instrs.as_slice(),
            [Instruction::Call { func, .. }] if func == "effectful"
        ));
    }
}
