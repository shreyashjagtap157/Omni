//! Reference MIR executor for the currently-qualified scalar subset.
//!
//! This module is a development/conformance oracle. It is not Omni's canonical
//! deployment runtime: released programs use the owned native AOT backend.
//! Unsupported MIR is rejected rather than assigned invented behavior.

use crate::complete_lexer::TokenKind;
use crate::mir::{Instruction, MirFunction, MirModule};
use std::collections::HashMap;
use std::io::Write;

const MAX_CALL_DEPTH: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Int(i64),
    Str(String),
    Bytes(Vec<u8>),
    Bool(bool),
    Aggregate {
        type_name: String,
        fields: Vec<(String, Value)>,
    },
    Ref {
        place: String,
        mutable: bool,
    },
    Unit,
}

pub fn run_mir_module(module: &MirModule) -> Result<(), String> {
    let entry = module
        .functions
        .iter()
        .find(|f| f.name == "main")
        .ok_or_else(|| "MIR module has no 'main' entry function".to_string())?;
    if !entry.params.is_empty() {
        return Err("MIR entry function must not require parameters".to_string());
    }
    let _ = execute_function(module, entry, Vec::new(), 0)?;
    Ok(())
}

pub fn run_mir_function(func: &MirFunction) -> Result<(), String> {
    if !func.params.is_empty() {
        return Err(format!(
            "cannot execute MIR function '{}' without {} required argument(s)",
            func.name,
            func.params.len()
        ));
    }
    let module = MirModule {
        functions: vec![clone_function(func)],
        gc_mode: None,
        unsafe_blocks: Vec::new(),
    };
    let _ = execute_function(&module, &module.functions[0], Vec::new(), 0)?;
    Ok(())
}

fn clone_function(func: &MirFunction) -> MirFunction {
    MirFunction {
        name: func.name.clone(),
        params: func.params.clone(),
        param_types: func.param_types.clone(),
        return_type: func.return_type.clone(),
        returns_value: func.returns_value,
        synthetic: func.synthetic,
        blocks: func
            .blocks
            .iter()
            .map(|block| crate::mir::BasicBlock {
                id: block.id,
                instrs: block.instrs.clone(),
            })
            .collect(),
        is_safe_wrapper: func.is_safe_wrapper,
        effects: func.effects.clone(),
    }
}

fn execute_function(
    module: &MirModule,
    func: &MirFunction,
    args: Vec<Value>,
    depth: usize,
) -> Result<Value, String> {
    if depth >= MAX_CALL_DEPTH {
        return Err(format!("MIR call-depth limit exceeded in '{}'", func.name));
    }
    if args.len() != func.params.len() {
        return Err(format!(
            "MIR call to '{}' expected {} argument(s), got {}",
            func.name,
            func.params.len(),
            args.len()
        ));
    }

    let mut env: HashMap<String, Value> = func.params.iter().cloned().zip(args).collect();
    let (instructions, labels) = flatten_function(func)?;
    let mut ip = 0usize;

    while ip < instructions.len() {
        match &instructions[ip] {
            Instruction::ConstInt { dest, value } => {
                env.insert(dest.clone(), Value::Int(*value));
                ip += 1;
            }
            Instruction::ConstStr { dest, value } => {
                env.insert(dest.clone(), Value::Str(value.clone()));
                ip += 1;
            }
            Instruction::ConstBytes { dest, value } => {
                env.insert(dest.clone(), Value::Bytes(value.clone()));
                ip += 1;
            }
            Instruction::ConstBool { dest, value } => {
                env.insert(dest.clone(), Value::Bool(*value));
                ip += 1;
            }
            Instruction::Move { dest, src } | Instruction::Assign { dest, src } => {
                let value = get_value(&env, src)?.clone();
                env.insert(dest.clone(), value);
                ip += 1;
            }
            Instruction::LinearMove { dest, src } => {
                let value = env
                    .remove(src)
                    .ok_or_else(|| format!("use of unavailable linear value '{src}'"))?;
                env.insert(dest.clone(), value);
                ip += 1;
            }
            Instruction::Borrow {
                dest,
                place,
                mutable,
            } => {
                let _ = get_value(&env, place)?;
                env.insert(
                    dest.clone(),
                    Value::Ref {
                        place: place.clone(),
                        mutable: *mutable,
                    },
                );
                ip += 1;
            }
            Instruction::Reborrow {
                dest,
                parent,
                mutable,
            } => {
                let (place, parent_mutable) = match get_value(&env, parent)? {
                    Value::Ref { place, mutable } => (place.clone(), *mutable),
                    other => {
                        return Err(format!("cannot reborrow non-reference MIR value {other:?}"))
                    }
                };
                if *mutable && !parent_mutable {
                    return Err(
                        "cannot create mutable MIR reborrow from shared reference".to_string()
                    );
                }
                env.insert(
                    dest.clone(),
                    Value::Ref {
                        place,
                        mutable: *mutable,
                    },
                );
                ip += 1;
            }
            Instruction::Deref { dest, reference } => {
                let place = match get_value(&env, reference)? {
                    Value::Ref { place, .. } => place.clone(),
                    other => {
                        return Err(format!(
                            "cannot dereference non-reference MIR value {other:?}"
                        ))
                    }
                };
                let value = get_value(&env, &place)?.clone();
                env.insert(dest.clone(), value);
                ip += 1;
            }
            Instruction::DerefAssign { reference, src } => {
                let (place, mutable) = match get_value(&env, reference)? {
                    Value::Ref { place, mutable } => (place.clone(), *mutable),
                    other => {
                        return Err(format!(
                            "cannot assign through non-reference MIR value {other:?}"
                        ))
                    }
                };
                if !mutable {
                    return Err("cannot assign through shared MIR reference".to_string());
                }
                let value = get_value(&env, src)?.clone();
                env.insert(place, value);
                ip += 1;
            }
            Instruction::Print { src } => {
                match get_value(&env, src)? {
                    Value::Int(value) => println!("{value}"),
                    Value::Str(value) => println!("{value}"),
                    Value::Bytes(value) => {
                        let mut stdout = std::io::stdout().lock();
                        stdout
                            .write_all(value)
                            .map_err(|e| format!("stdout write failed: {e}"))?;
                        stdout
                            .write_all(b"\n")
                            .map_err(|e| format!("stdout write failed: {e}"))?;
                    }
                    Value::Bool(value) => println!("{value}"),
                    Value::Aggregate { .. } => {
                        return Err(
                            "printing aggregate values is not yet part of the v0.1.4 core ABI"
                                .to_string(),
                        )
                    }
                    Value::Ref { .. } => {
                        return Err("printing reference values is not qualified".to_string())
                    }
                    Value::Unit => println!("()"),
                }
                ip += 1;
            }
            Instruction::Drop { var } | Instruction::DropLinear { var } => {
                env.remove(var);
                ip += 1;
            }
            Instruction::Label { .. } => ip += 1,
            Instruction::Jump { target } => {
                ip = *labels
                    .get(target)
                    .ok_or_else(|| format!("MIR jump references undefined label {target}"))?;
            }
            Instruction::JumpIf { cond, target } => {
                if truthy(get_value(&env, cond)?)? {
                    ip = *labels.get(target).ok_or_else(|| {
                        format!("MIR conditional jump references undefined label {target}")
                    })?;
                } else {
                    ip += 1;
                }
            }
            Instruction::BinaryOp {
                dest,
                op,
                left,
                right,
            } => {
                let lhs = get_value(&env, left)?.clone();
                let rhs = get_value(&env, right)?.clone();
                let value = eval_binary(op, lhs, rhs)?;
                env.insert(dest.clone(), value);
                ip += 1;
            }
            Instruction::UnaryOp { dest, op, operand } => {
                let value = eval_unary(op, get_value(&env, operand)?.clone())?;
                env.insert(dest.clone(), value);
                ip += 1;
            }
            Instruction::Call { dest, func, args } => {
                if func == "print" {
                    if args.len() != 1 {
                        return Err(format!(
                            "builtin print expects 1 argument, got {}",
                            args.len()
                        ));
                    }
                    match get_value(&env, &args[0])? {
                        Value::Int(value) => println!("{value}"),
                        Value::Str(value) => println!("{value}"),
                        Value::Bytes(value) => {
                            let mut stdout = std::io::stdout().lock();
                            stdout
                                .write_all(value)
                                .map_err(|e| format!("stdout write failed: {e}"))?;
                            stdout
                                .write_all(b"\n")
                                .map_err(|e| format!("stdout write failed: {e}"))?;
                        }
                        Value::Bool(value) => println!("{value}"),
                        Value::Aggregate { .. } => {
                            return Err(
                                "printing aggregate values is not yet part of the v0.1.4 core ABI"
                                    .to_string(),
                            );
                        }
                        Value::Ref { .. } => {
                            return Err("printing reference values is not qualified".to_string())
                        }
                        Value::Unit => println!("()"),
                    }
                    env.insert(dest.clone(), Value::Unit);
                    ip += 1;
                    continue;
                }
                if func.starts_with("__omni_unsupported_") {
                    return Err(format!(
                        "MIR contains unsupported feature sentinel '{func}'"
                    ));
                }
                let callee = module
                    .functions
                    .iter()
                    .find(|candidate| candidate.name == *func)
                    .ok_or_else(|| format!("MIR call references undefined function '{func}'"))?;
                let call_args = args
                    .iter()
                    .map(|name| get_value(&env, name).cloned())
                    .collect::<Result<Vec<_>, _>>()?;
                let result = execute_function(module, callee, call_args, depth + 1)?;
                if callee.returns_value {
                    env.insert(dest.clone(), result);
                } else {
                    env.insert(dest.clone(), Value::Unit);
                }
                ip += 1;
            }
            Instruction::AggregateInit {
                dest,
                type_name,
                fields,
            } => {
                let mut values = Vec::with_capacity(fields.len());
                for (name, value) in fields {
                    values.push((name.clone(), get_value(&env, value)?.clone()));
                }
                env.insert(
                    dest.clone(),
                    Value::Aggregate {
                        type_name: type_name.clone(),
                        fields: values,
                    },
                );
                ip += 1;
            }
            Instruction::EnumInit {
                dest,
                type_name,
                variant: _,
                tag,
                fields,
            } => {
                let mut values = Vec::with_capacity(fields.len() + 1);
                values.push(("@tag".to_string(), Value::Int(i64::from(*tag))));
                for (name, value) in fields {
                    values.push((name.clone(), get_value(&env, value)?.clone()));
                }
                env.insert(
                    dest.clone(),
                    Value::Aggregate {
                        type_name: type_name.clone(),
                        fields: values,
                    },
                );
                ip += 1;
            }
            Instruction::EnumTag { dest, base } => {
                let Value::Aggregate { fields, .. } = get_value(&env, base)? else {
                    return Err(format!("enum tag access requires aggregate value '{base}'"));
                };
                let tag = fields
                    .first()
                    .filter(|(name, _)| name == "@tag")
                    .map(|(_, value)| value.clone())
                    .ok_or_else(|| format!("aggregate '{base}' is not an enum value"))?;
                env.insert(dest.clone(), tag);
                ip += 1;
            }
            Instruction::EnumPayloadAccess { dest, base, index } => {
                let Value::Aggregate { fields, .. } = get_value(&env, base)? else {
                    return Err(format!(
                        "enum payload access requires aggregate value '{base}'"
                    ));
                };
                let payload_index = usize::try_from(*index)
                    .map_err(|_| "enum payload index does not fit usize".to_string())?;
                let value = fields
                    .get(payload_index + 1)
                    .map(|(_, value)| value.clone())
                    .ok_or_else(|| {
                        format!(
                            "enum payload index {} out of bounds for '{}'",
                            payload_index, base
                        )
                    })?;
                env.insert(dest.clone(), value);
                ip += 1;
            }
            Instruction::Return { value } => {
                if !func.returns_value {
                    return Ok(Value::Unit);
                }
                return Ok(get_value(&env, value)?.clone());
            }
            Instruction::StructDef { .. } | Instruction::EnumDef { .. } => {
                // Type metadata has no runtime action in the scalar interpreter.
                ip += 1;
            }
            Instruction::FieldAccess {
                dest, base, field, ..
            } => {
                let Value::Aggregate { fields, .. } = get_value(&env, base)? else {
                    return Err(format!("field access requires aggregate value '{base}'"));
                };
                let value = fields
                    .iter()
                    .find(|(name, _)| name == field)
                    .map(|(_, value)| value.clone())
                    .ok_or_else(|| format!("aggregate '{base}' has no field '{field}'"))?;
                env.insert(dest.clone(), value);
                ip += 1;
            }
            Instruction::FieldAssign { base, field, src } => {
                let value = get_value(&env, src)?.clone();
                let Some(Value::Aggregate { fields, .. }) = env.get_mut(base) else {
                    return Err(format!(
                        "field assignment requires aggregate value '{base}'"
                    ));
                };
                let Some((_, slot)) = fields.iter_mut().find(|(name, _)| name == field) else {
                    return Err(format!("aggregate '{base}' has no field '{field}'"));
                };
                *slot = value;
                ip += 1;
            }
            Instruction::IndexAccess { dest, base, index } => {
                let idx = match get_value(&env, index)? {
                    Value::Int(value) if *value >= 0 => *value as usize,
                    Value::Int(_) => return Err("aggregate index must be non-negative".to_string()),
                    _ => return Err("aggregate index must be an integer".to_string()),
                };
                let Value::Aggregate { fields, .. } = get_value(&env, base)? else {
                    return Err(format!("index access requires aggregate value '{base}'"));
                };
                let value = fields
                    .get(idx)
                    .map(|(_, value)| value.clone())
                    .ok_or_else(|| {
                        format!(
                            "aggregate index {idx} out of bounds for length {}",
                            fields.len()
                        )
                    })?;
                env.insert(dest.clone(), value);
                ip += 1;
            }
            Instruction::SliceAccess {
                dest,
                base,
                start,
                end,
                inclusive,
            } => {
                let start_index = match get_value(&env, start)? {
                    Value::Int(value) if *value >= 0 => usize::try_from(*value)
                        .map_err(|_| "slice start does not fit usize".to_string())?,
                    Value::Int(_) => return Err("slice start must be non-negative".to_string()),
                    _ => return Err("slice start must be an integer".to_string()),
                };
                let raw_end = match get_value(&env, end)? {
                    Value::Int(value) if *value >= 0 => usize::try_from(*value)
                        .map_err(|_| "slice end does not fit usize".to_string())?,
                    Value::Int(_) => return Err("slice end must be non-negative".to_string()),
                    _ => return Err("slice end must be an integer".to_string()),
                };
                let end_exclusive = if *inclusive {
                    raw_end
                        .checked_add(1)
                        .ok_or_else(|| "inclusive slice end overflow".to_string())?
                } else {
                    raw_end
                };
                let Value::Aggregate { fields, .. } = get_value(&env, base)? else {
                    return Err(format!("slice access requires aggregate value '{base}'"));
                };
                if start_index > end_exclusive || end_exclusive > fields.len() {
                    return Err(format!(
                        "slice range {}..{} out of bounds for length {}",
                        start_index,
                        end_exclusive,
                        fields.len()
                    ));
                }
                let values = fields[start_index..end_exclusive]
                    .iter()
                    .enumerate()
                    .map(|(index, (_, value))| (index.to_string(), value.clone()))
                    .collect();
                env.insert(
                    dest.clone(),
                    Value::Aggregate {
                        type_name: "Slice".to_string(),
                        fields: values,
                    },
                );
                ip += 1;
            }
            Instruction::StructAccess { .. } => {
                return Err(format!("aggregate field mutation is not yet qualified in v0.1.4 (instruction at {}:{ip})", func.name));
            }
            Instruction::MatchBranch { .. } => {
                return Err(format!(
                    "match-branch MIR is not qualified in v0.1.4 (instruction at {}:{ip})",
                    func.name
                ));
            }
            Instruction::Spawn { .. } | Instruction::Channel { .. } => {
                return Err(format!(
                    "concurrency MIR is not qualified in v0.1.4 (instruction at {}:{ip})",
                    func.name
                ));
            }
        }
    }

    if func.returns_value {
        Err(format!(
            "MIR function '{}' reached the end without returning a value",
            func.name
        ))
    } else {
        Ok(Value::Unit)
    }
}

fn flatten_function(
    func: &MirFunction,
) -> Result<(Vec<Instruction>, HashMap<usize, usize>), String> {
    let mut instructions = Vec::new();
    let mut labels = HashMap::new();
    for block in &func.blocks {
        for instr in &block.instrs {
            let index = instructions.len();
            if let Instruction::Label { id } = instr {
                if labels.insert(*id, index).is_some() {
                    return Err(format!(
                        "MIR function '{}' defines label {} more than once",
                        func.name, id
                    ));
                }
            }
            instructions.push(instr.clone());
        }
    }
    Ok((instructions, labels))
}

fn get_value<'a>(env: &'a HashMap<String, Value>, name: &str) -> Result<&'a Value, String> {
    env.get(name)
        .ok_or_else(|| format!("use of undefined MIR value '{name}'"))
}

fn truthy(value: &Value) -> Result<bool, String> {
    match value {
        Value::Bool(value) => Ok(*value),
        Value::Int(value) => Ok(*value != 0),
        other => Err(format!("condition requires bool/i64 scalar, got {other:?}")),
    }
}

fn eval_binary(op: &TokenKind, lhs: Value, rhs: Value) -> Result<Value, String> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => match op {
            TokenKind::Plus => a
                .checked_add(b)
                .map(Value::Int)
                .ok_or_else(|| "checked i64 addition overflow".to_string()),
            TokenKind::Minus => a
                .checked_sub(b)
                .map(Value::Int)
                .ok_or_else(|| "checked i64 subtraction overflow".to_string()),
            TokenKind::Star => a
                .checked_mul(b)
                .map(Value::Int)
                .ok_or_else(|| "checked i64 multiplication overflow".to_string()),
            TokenKind::Slash => {
                if b == 0 {
                    Err("checked i64 division by zero".to_string())
                } else {
                    a.checked_div(b)
                        .map(Value::Int)
                        .ok_or_else(|| "checked i64 division overflow".to_string())
                }
            }
            TokenKind::Percent => {
                if b == 0 {
                    Err("checked i64 remainder by zero".to_string())
                } else {
                    a.checked_rem(b)
                        .map(Value::Int)
                        .ok_or_else(|| "checked i64 remainder overflow".to_string())
                }
            }
            TokenKind::EqEq => Ok(Value::Bool(a == b)),
            TokenKind::NotEq => Ok(Value::Bool(a != b)),
            TokenKind::Lt => Ok(Value::Bool(a < b)),
            TokenKind::LtEq => Ok(Value::Bool(a <= b)),
            TokenKind::Gt => Ok(Value::Bool(a > b)),
            TokenKind::GtEq => Ok(Value::Bool(a >= b)),
            TokenKind::AndAnd => Ok(Value::Bool(a != 0 && b != 0)),
            TokenKind::OrOr => Ok(Value::Bool(a != 0 || b != 0)),
            other => Err(format!("unsupported scalar binary operator {other:?}")),
        },
        (Value::Bool(a), Value::Bool(b)) => match op {
            TokenKind::EqEq => Ok(Value::Bool(a == b)),
            TokenKind::NotEq => Ok(Value::Bool(a != b)),
            TokenKind::AndAnd => Ok(Value::Bool(a && b)),
            TokenKind::OrOr => Ok(Value::Bool(a || b)),
            other => Err(format!("unsupported boolean binary operator {other:?}")),
        },
        (Value::Str(a), Value::Str(b)) => match op {
            TokenKind::Plus => Ok(Value::Str(a + &b)),
            TokenKind::EqEq => Ok(Value::Bool(a == b)),
            TokenKind::NotEq => Ok(Value::Bool(a != b)),
            other => Err(format!("unsupported string binary operator {other:?}")),
        },
        (a, b) => Err(format!(
            "type mismatch in binary operation: {a:?} and {b:?}"
        )),
    }
}

fn eval_unary(op: &TokenKind, value: Value) -> Result<Value, String> {
    match (op, value) {
        (TokenKind::Minus, Value::Int(value)) => value
            .checked_neg()
            .map(Value::Int)
            .ok_or_else(|| "checked i64 negation overflow".to_string()),
        (TokenKind::Bang, Value::Int(value)) => Ok(Value::Bool(value == 0)),
        (TokenKind::Bang, Value::Bool(value)) => Ok(Value::Bool(!value)),
        (other, value) => Err(format!(
            "unsupported unary operation {other:?} on {value:?}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::{BasicBlock, MirFunction, MirModule};

    #[test]
    fn scalar_call_returns_value() {
        let mut id = MirFunction::new("id", false);
        id.params = vec!["x".into()];
        id.returns_value = true;
        id.blocks = vec![BasicBlock {
            id: 0,
            instrs: vec![Instruction::Return { value: "x".into() }],
        }];

        let mut main = MirFunction::new("main", false);
        main.params = vec![];
        main.returns_value = true;
        main.blocks = vec![BasicBlock {
            id: 0,
            instrs: vec![
                Instruction::ConstInt {
                    dest: "a".into(),
                    value: 42,
                },
                Instruction::Call {
                    dest: "r".into(),
                    func: "id".into(),
                    args: vec!["a".into()],
                },
                Instruction::Return { value: "r".into() },
            ],
        }];

        let module = MirModule {
            functions: vec![id, main],
            gc_mode: None,
            unsafe_blocks: vec![],
        };
        let result = execute_function(&module, &module.functions[1], vec![], 0).expect("execute");
        assert_eq!(result, Value::Int(42));
    }

    #[test]
    fn unsupported_concurrency_fails_closed() {
        let mut main = MirFunction::new("main", false);
        main.returns_value = false;
        main.blocks = vec![BasicBlock {
            id: 0,
            instrs: vec![Instruction::Channel {
                dest: "c".into(),
                elem_type: "i64".into(),
                capacity: Some(1),
            }],
        }];
        let module = MirModule {
            functions: vec![main],
            gc_mode: None,
            unsafe_blocks: vec![],
        };
        let err = run_mir_module(&module).expect_err("must reject");
        assert!(err.contains("not qualified"));
    }
}
