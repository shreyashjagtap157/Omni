use crate::lexer::TokenKind;
use crate::mir::{Instruction, MirModule};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Str(String),
    Bool(bool),
}

pub fn run_mir_module(module: &MirModule) -> Result<(), String> {
    let mut env: HashMap<String, Value> = HashMap::new();

    for func in &module.functions {
        for block in &func.blocks {
            let mut ip = 0;
            let blocks: Vec<usize> = block
                .instrs
                .iter()
                .enumerate()
                .filter(|(_, i)| matches!(i, Instruction::Label { .. }))
                .map(|(idx, _)| idx)
                .collect();
            let mut block_map: HashMap<usize, usize> = HashMap::new();
            for (i, &idx) in blocks.iter().enumerate() {
                block_map.insert(idx, i);
            }

            while ip < block.instrs.len() {
                let instr = &block.instrs[ip];
                match instr {
                    Instruction::ConstInt { dest, value } => {
                        env.insert(dest.clone(), Value::Int(*value));
                        ip += 1;
                    }
                    Instruction::ConstStr { dest, value } => {
                        env.insert(dest.clone(), Value::Str(value.clone()));
                        ip += 1;
                    }
                    Instruction::ConstBool { dest, value } => {
                        env.insert(dest.clone(), Value::Bool(*value));
                        ip += 1;
                    }
                    Instruction::Move { dest, src } => {
                        let v = env
                            .get(src)
                            .cloned()
                            .ok_or(format!("Undefined var: {}", src))?;
                        env.insert(dest.clone(), v);
                        ip += 1;
                    }
                    Instruction::Print { src } => {
                        let v = env
                            .get(src)
                            .cloned()
                            .ok_or(format!("Undefined var: {}", src))?;
                        match v {
                            Value::Int(n) => println!("{}", n),
                            Value::Str(s) => println!("{}", s),
                            Value::Bool(b) => println!("{}", b),
                        }
                        ip += 1;
                    }
                    Instruction::Drop { var } => {
                        env.remove(var);
                        ip += 1;
                    }
                    Instruction::Jump { target } => {
                        if let Some(&target_idx) = blocks.get(*target) {
                            ip = target_idx;
                        } else {
                            ip += 1;
                        }
                    }
                    Instruction::JumpIf { cond, target } => {
                        let v = env
                            .get(cond)
                            .cloned()
                            .ok_or(format!("Undefined var: {}", cond))?;
                        let cond_val = match v {
                            Value::Int(n) => n != 0,
                            Value::Bool(b) => b,
                            Value::Str(s) => !s.is_empty(),
                        };
                        if cond_val {
                            if let Some(&target_idx) = blocks.get(*target) {
                                ip = target_idx;
                            } else {
                                ip += 1;
                            }
                        } else {
                            ip += 1;
                        }
                    }
                    Instruction::Label { .. } => {
                        ip += 1;
                    }
                    Instruction::BinaryOp {
                        dest,
                        op,
                        left,
                        right,
                    } => {
                        let lv = env
                            .get(left)
                            .cloned()
                            .ok_or(format!("Undefined var: {}", left))?;
                        let rv = env
                            .get(right)
                            .cloned()
                            .ok_or(format!("Undefined var: {}", right))?;
                        let result = match (lv, rv) {
                            (Value::Int(a), Value::Int(b)) => {
                                let r = match op {
                                    TokenKind::Plus => a + b,
                                    TokenKind::Minus => a - b,
                                    TokenKind::Star => a * b,
                                    TokenKind::Slash => a / b,
                                    TokenKind::Percent => a % b,
                                    TokenKind::EqEq => {
                                        if a == b {
                                            1
                                        } else {
                                            0
                                        }
                                    }
                                    TokenKind::NotEq => {
                                        if a != b {
                                            1
                                        } else {
                                            0
                                        }
                                    }
                                    TokenKind::Lt => {
                                        if a < b {
                                            1
                                        } else {
                                            0
                                        }
                                    }
                                    TokenKind::LtEq => {
                                        if a <= b {
                                            1
                                        } else {
                                            0
                                        }
                                    }
                                    TokenKind::Gt => {
                                        if a > b {
                                            1
                                        } else {
                                            0
                                        }
                                    }
                                    TokenKind::GtEq => {
                                        if a >= b {
                                            1
                                        } else {
                                            0
                                        }
                                    }
                                    TokenKind::AndAnd => {
                                        if a != 0 && b != 0 {
                                            1
                                        } else {
                                            0
                                        }
                                    }
                                    TokenKind::OrOr => {
                                        if a != 0 || b != 0 {
                                            1
                                        } else {
                                            0
                                        }
                                    }
                                    _ => return Err(format!("Unsupported binary op: {:?}", op)),
                                };
                                Value::Int(r)
                            }
                            (Value::Str(a), Value::Str(b)) => match op {
                                TokenKind::Plus => Value::Str(a + &b),
                                TokenKind::EqEq => Value::Bool(a == b),
                                TokenKind::NotEq => Value::Bool(a != b),
                                _ => {
                                    return Err(format!(
                                        "Unsupported binary op for strings: {:?}",
                                        op
                                    ))
                                }
                            },
                            _ => return Err("Type mismatch in binary operation".to_string()),
                        };
                        env.insert(dest.clone(), result);
                        ip += 1;
                    }
                    Instruction::UnaryOp { dest, op, operand } => {
                        let v = env
                            .get(operand)
                            .cloned()
                            .ok_or(format!("Undefined var: {}", operand))?;
                        let result = match v {
                            Value::Int(n) => {
                                let r = match op {
                                    TokenKind::Minus => -n,
                                    TokenKind::Bang => {
                                        if n == 0 {
                                            1
                                        } else {
                                            0
                                        }
                                    }
                                    _ => return Err(format!("Unsupported unary op: {:?}", op)),
                                };
                                Value::Int(r)
                            }
                            Value::Bool(b) => {
                                let r = match op {
                                    TokenKind::Bang => !b,
                                    _ => {
                                        return Err(format!(
                                            "Unsupported unary op for bool: {:?}",
                                            op
                                        ))
                                    }
                                };
                                Value::Bool(r)
                            }
                            _ => return Err("Type mismatch in unary operation".to_string()),
                        };
                        env.insert(dest.clone(), result);
                        ip += 1;
                    }
                    Instruction::Return { value } => {
                        let _ = env
                            .get(value)
                            .cloned()
                            .ok_or(format!("Undefined var: {}", value))?;
                        return Ok(());
                    }
                    Instruction::Assign { dest, src } => {
                        let v = env.get(src).cloned().unwrap_or(Value::Int(0));
                        env.insert(dest.clone(), v);
                        ip += 1;
                    }
                    Instruction::Call { dest, func, args } => {
                        let _ = func;
                        let _ = args;
                        env.insert(dest.clone(), Value::Int(0));
                        ip += 1;
                    }
                    Instruction::FieldAccess { dest, base, field } => {
                        let b = env
                            .get(base)
                            .cloned()
                            .ok_or(format!("Undefined var: {}", base))?;
                        let result = match b {
                            Value::Str(s) if field == "len" => Value::Int(s.len() as i64),
                            _ => return Err(format!("Unknown field access: {}", field)),
                        };
                        env.insert(dest.clone(), result);
                        ip += 1;
                    }
                    Instruction::StructAccess { dest, base, field } => {
                        let b = env
                            .get(base)
                            .cloned()
                            .ok_or(format!("Undefined var: {}", base))?;
                        let result = match b {
                            Value::Str(s) if field == "len" => Value::Int(s.len() as i64),
                            _ => return Err(format!("Unknown struct field access: {}", field)),
                        };
                        env.insert(dest.clone(), result);
                        ip += 1;
                    }
                    Instruction::IndexAccess { dest, base, index } => {
                        let b = env
                            .get(base)
                            .cloned()
                            .ok_or(format!("Undefined var: {}", base))?;
                        let i = env
                            .get(index)
                            .cloned()
                            .ok_or(format!("Undefined var: {}", index))?;
                        let _ = (b, i);
                        env.insert(dest.clone(), Value::Int(0));
                        ip += 1;
                    }
                    Instruction::LinearMove { dest, src } => {
                        let v = env
                            .get(src)
                            .cloned()
                            .ok_or(format!("Undefined var: {}", src))?;
                        env.insert(dest.clone(), v);
                        env.remove(src);
                        ip += 1;
                    }
                    Instruction::DropLinear { var } => {
                        env.remove(var);
                        ip += 1;
                    }
                    Instruction::StructDef { .. } => {
                        ip += 1;
                    }
                    Instruction::EnumDef { .. } => {
                        ip += 1;
                    }
                }
            }
        }
    }
    Ok(())
}
