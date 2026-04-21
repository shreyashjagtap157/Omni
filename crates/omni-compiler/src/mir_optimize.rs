use crate::lexer::TokenKind;
use crate::mir::{BasicBlock, Instruction, MirFunction, MirModule};
use std::collections::HashMap;

#[derive(Debug, Clone)]
enum ConstVal {
    Int(i64),
    Str(String),
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
    let mut out_instrs: Vec<Instruction> = Vec::new();

    while let Some(instr) = block.instrs.pop() {
        // We drain from the end to preserve simple processing; collect into a vec
        out_instrs.push(instr);
    }
    out_instrs.reverse();

    let mut new_instrs: Vec<Instruction> = Vec::new();
    for instr in out_instrs.into_iter() {
        match instr {
            Instruction::ConstInt { dest, value } => {
                consts.insert(dest.clone(), ConstVal::Int(value));
                new_instrs.push(Instruction::ConstInt { dest, value });
            }
            Instruction::ConstStr { dest, value } => {
                consts.insert(dest.clone(), ConstVal::Str(value.clone()));
                new_instrs.push(Instruction::ConstStr { dest, value });
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
                match (lconst, rconst) {
                    (Some(ConstVal::Int(a)), Some(ConstVal::Int(b))) => match op {
                        TokenKind::Plus => {
                            let v = a + b;
                            consts.insert(dest.clone(), ConstVal::Int(v));
                            new_instrs.push(Instruction::ConstInt { dest, value: v });
                        }
                        TokenKind::Minus => {
                            let v = a - b;
                            consts.insert(dest.clone(), ConstVal::Int(v));
                            new_instrs.push(Instruction::ConstInt { dest, value: v });
                        }
                        TokenKind::Star => {
                            let v = a * b;
                            consts.insert(dest.clone(), ConstVal::Int(v));
                            new_instrs.push(Instruction::ConstInt { dest, value: v });
                        }
                        TokenKind::Slash => {
                            let v = if b == 0 { 0 } else { a / b };
                            consts.insert(dest.clone(), ConstVal::Int(v));
                            new_instrs.push(Instruction::ConstInt { dest, value: v });
                        }
                        TokenKind::Percent => {
                            let v = if b == 0 { 0 } else { a % b };
                            consts.insert(dest.clone(), ConstVal::Int(v));
                            new_instrs.push(Instruction::ConstInt { dest, value: v });
                        }
                        TokenKind::EqEq => {
                            consts.insert(dest.clone(), ConstVal::Bool(a == b));
                            new_instrs.push(Instruction::ConstBool {
                                dest,
                                value: a == b,
                            });
                        }
                        TokenKind::NotEq => {
                            consts.insert(dest.clone(), ConstVal::Bool(a != b));
                            new_instrs.push(Instruction::ConstBool {
                                dest,
                                value: a != b,
                            });
                        }
                        TokenKind::Lt => {
                            consts.insert(dest.clone(), ConstVal::Bool(a < b));
                            new_instrs.push(Instruction::ConstBool { dest, value: a < b });
                        }
                        TokenKind::LtEq => {
                            consts.insert(dest.clone(), ConstVal::Bool(a <= b));
                            new_instrs.push(Instruction::ConstBool {
                                dest,
                                value: a <= b,
                            });
                        }
                        TokenKind::Gt => {
                            consts.insert(dest.clone(), ConstVal::Bool(a > b));
                            new_instrs.push(Instruction::ConstBool { dest, value: a > b });
                        }
                        TokenKind::GtEq => {
                            consts.insert(dest.clone(), ConstVal::Bool(a >= b));
                            new_instrs.push(Instruction::ConstBool {
                                dest,
                                value: a >= b,
                            });
                        }
                        _ => new_instrs.push(Instruction::BinaryOp {
                            dest,
                            op,
                            left,
                            right,
                        }),
                    },
                    (Some(ConstVal::Str(a)), Some(ConstVal::Str(b))) => match op {
                        TokenKind::Plus => {
                            let s = format!("{}{}", a, b);
                            consts.insert(dest.clone(), ConstVal::Str(s.clone()));
                            new_instrs.push(Instruction::ConstStr { dest, value: s });
                        }
                        TokenKind::EqEq => {
                            consts.insert(dest.clone(), ConstVal::Bool(a == b));
                            new_instrs.push(Instruction::ConstBool {
                                dest,
                                value: a == b,
                            });
                        }
                        TokenKind::NotEq => {
                            consts.insert(dest.clone(), ConstVal::Bool(a != b));
                            new_instrs.push(Instruction::ConstBool {
                                dest,
                                value: a != b,
                            });
                        }
                        _ => new_instrs.push(Instruction::BinaryOp {
                            dest,
                            op,
                            left,
                            right,
                        }),
                    },
                    _ => new_instrs.push(Instruction::BinaryOp {
                        dest,
                        op,
                        left,
                        right,
                    }),
                }
            }
            Instruction::UnaryOp { dest, op, operand } => {
                if let Some(ConstVal::Int(a)) = consts.get(&operand).cloned() {
                    match op {
                        TokenKind::Minus => {
                            let v = -a;
                            consts.insert(dest.clone(), ConstVal::Int(v));
                            new_instrs.push(Instruction::ConstInt { dest, value: v });
                        }
                        _ => new_instrs.push(Instruction::UnaryOp { dest, op, operand }),
                    }
                } else {
                    new_instrs.push(Instruction::UnaryOp { dest, op, operand })
                }
            }
            other => new_instrs.push(other),
        }
    }

    block.instrs = new_instrs;
}

#[allow(dead_code)]
fn try_constant_fold(_instr: &Instruction) -> Option<Instruction> {
    None
}

#[allow(dead_code)]
fn eval_var(_var: &str) -> Option<i64> {
    None
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
                    Instruction::FieldAccess { base, .. }
                    | Instruction::StructAccess { base, .. }
                    | Instruction::IndexAccess { base, .. } => {
                        used.insert(base.clone());
                    }
                    _ => {}
                }
            }
        }

        // Remove instructions that define a destination never present in `used`.
        for block in &mut func.blocks {
            let before = block.instrs.len();
            block.instrs.retain(|instr| {
                match instr {
                    Instruction::ConstInt { dest, .. }
                    | Instruction::ConstStr { dest, .. }
                    | Instruction::ConstBool { dest, .. }
                    | Instruction::BinaryOp { dest, .. }
                    | Instruction::UnaryOp { dest, .. }
                    | Instruction::Move { dest, .. }
                    | Instruction::LinearMove { dest, .. }
                    | Instruction::Assign { dest, .. }
                    | Instruction::Call { dest, .. } => {
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

#[allow(dead_code)]
fn is_dead(_instr: &Instruction) -> bool {
    false
}

pub fn inline_simple_functions(module: &mut MirModule) {
    use std::collections::HashMap;

    // Identify functions that are trivially inlinable as a constant-return.
    let mut inlinable: HashMap<String, ConstVal> = HashMap::new();

    for f in &module.functions {
        if f.blocks.len() != 1 {
            continue;
        }
        let block = &f.blocks[0];

        // Must end with a Return and return a value that was produced by
        // a single const instruction in the same block. This keeps the
        // inliner simple and conservative.
        let ret_var = match block.instrs.last() {
            Some(Instruction::Return { value }) => Some(value.clone()),
            _ => None,
        };
        if ret_var.is_none() {
            continue;
        }
        let ret_var = ret_var.unwrap();

        // Search for a const that defines ret_var.
        let mut found: Option<ConstVal> = None;
        let mut cost = 0usize;
        for instr in &block.instrs {
            match instr {
                Instruction::ConstInt { dest, value } if dest == &ret_var => {
                    found = Some(ConstVal::Int(*value));
                }
                Instruction::ConstStr { dest, value } if dest == &ret_var => {
                    found = Some(ConstVal::Str(value.clone()));
                }
                Instruction::ConstBool { dest, value } if dest == &ret_var => {
                    found = Some(ConstVal::Bool(*value));
                }
                Instruction::Label { .. }
                | Instruction::Drop { .. }
                | Instruction::DropLinear { .. } => {}
                _ => cost += 1,
            }
        }

        // Heuristic: only inline tiny functions (cost <= 4) that return a
        // single constant.
        if cost <= 4 {
            if let Some(cv) = found {
                inlinable.insert(f.name.clone(), cv);
            }
        }
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
