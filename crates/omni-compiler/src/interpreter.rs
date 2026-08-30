use crate::ast::{Expr, InterpolatedFragment, Program, Stmt};
use crate::complete_lexer::TokenKind;
use crate::effect_system::{CancellationToken, Channel};
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    /// Stack of active effect handler mappings.
    /// Each layer is effect_name → handler_closure_value.
    static HANDLER_STACK: RefCell<Vec<HashMap<String, Value>>> = const { RefCell::new(Vec::new()) };
    /// Control flow signal for break/continue within loops.
    static CONTROL_FLOW: RefCell<Option<ControlFlow>> = const { RefCell::new(None) };
    /// Deferred cleanups registered by nested blocks.
    static DEFER_STACK: RefCell<Vec<Vec<Stmt>>> = const { RefCell::new(Vec::new()) };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlFlow {
    Break,
    Continue,
}

fn resolve_handler(name: &str) -> Option<Value> {
    HANDLER_STACK.with(|stack| {
        let stack = stack.borrow();
        // Search from top of stack (most recent handler) downward
        for layer in stack.iter().rev() {
            if let Some(val) = layer.get(name) {
                return Some(val.clone());
            }
        }
        None
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Unit,
    Int(i64),
    Float(f64),
    Char(char),
    Byte(u8),
    Str(String),
    Bytes(Vec<u8>),
    Bool(bool),
    Vector(Vec<Value>),
    Map(HashMap<String, Value>),
    Channel(Channel<Value>),
    CancellationToken(CancellationToken),
    Closure {
        params: Vec<(String, Option<String>)>,
        body: Box<Expr>,
        env: Box<HashMap<String, Value>>,
    },
    Result(Result<Box<Value>, Box<Value>>),
    Record(String, HashMap<String, Value>),
}

fn checked_index(index: i64, operation: &str) -> Result<usize, String> {
    usize::try_from(index).map_err(|_| {
        format!(
            "{}: index/length must be non-negative and representable",
            operation
        )
    })
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Unit => false,
        Value::Int(n) => *n != 0,
        Value::Float(n) => *n != 0.0,
        Value::Char(c) => *c != '\0',
        Value::Byte(b) => *b != 0,
        Value::Bytes(bytes) => !bytes.is_empty(),
        Value::Bool(b) => *b,
        Value::Str(s) => !s.is_empty(),
        Value::Vector(vv) => !vv.is_empty(),
        Value::Map(m) => !m.is_empty(),
        Value::Channel(_) => true,
        Value::CancellationToken(_) => true,
        Value::Closure { .. } => true,
        Value::Result(Ok(v)) => truthy(v),
        Value::Result(Err(_)) => false,
        Value::Record(_, _) => true,
    }
}

fn match_pattern(pattern: &crate::ast::Pattern, value: &Value) -> Option<HashMap<String, Value>> {
    match pattern {
        crate::ast::Pattern::Wildcard => Some(HashMap::new()),
        crate::ast::Pattern::Literal(expected) => match value {
            Value::Int(actual) if actual == expected => Some(HashMap::new()),
            _ => None,
        },
        crate::ast::Pattern::Var(name) => {
            let mut bindings = HashMap::new();
            bindings.insert(name.clone(), value.clone());
            Some(bindings)
        }
        crate::ast::Pattern::Struct(_name, fields) => {
            if let Value::Map(map) = value {
                let mut bindings = HashMap::new();
                for (field_name, field_pattern) in fields {
                    let field_value = map.get(field_name)?;
                    let nested_bindings = match_pattern(field_pattern, field_value)?;
                    for (bind_name, bind_value) in nested_bindings {
                        bindings.insert(bind_name, bind_value);
                    }
                }
                Some(bindings)
            } else {
                None
            }
        }
        crate::ast::Pattern::Or(patterns) => {
            for alternative in patterns {
                if let Some(bindings) = match_pattern(alternative, value) {
                    return Some(bindings);
                }
            }
            None
        }
    }
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::Unit => "()".to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(n) => n.to_string(),
        Value::Char(c) => c.to_string(),
        Value::Byte(b) => b.to_string(),
        Value::Str(s) => s.clone(),
        Value::Bytes(bytes) => format!("{:?}", bytes),
        Value::Bool(b) => b.to_string(),
        Value::Vector(vec) => format!("<vector len={}>", vec.len()),
        Value::Map(map) => format!("<map len={}>", map.len()),
        Value::Channel(_) => "<channel>".to_string(),
        Value::CancellationToken(_) => "<cancellation_token>".to_string(),
        Value::Closure { .. } => "<fn>".to_string(),
        Value::Result(Ok(v)) => format!("Ok({})", value_to_string(v.as_ref())),
        Value::Result(Err(v)) => format!("Err({})", value_to_string(v.as_ref())),
        Value::Record(name, _) => format!("{} {{ ... }}", name),
    }
}

fn eval_expr(
    expr: &Expr,
    env: &mut HashMap<String, Value>,
    functions: &HashMap<String, &Stmt>,
) -> Result<Value, String> {
    match expr {
        Expr::StringLit(s, _) => Ok(Value::Str(s.clone())),
        Expr::ByteString(bytes, _) => Ok(Value::Bytes(bytes.clone())),
        Expr::Byte(value, _) => Ok(Value::Byte(*value)),
        Expr::Number(n, _) => Ok(Value::Int(*n)),
        Expr::Float(n, _) => Ok(Value::Float(*n)),
        Expr::Char(c, _) => Ok(Value::Char(*c)),
        Expr::Var(name, _) => env
            .get(name)
            .cloned()
            .ok_or(format!("Undefined var {}", name)),
        Expr::Bool(b, _) => Ok(Value::Bool(*b)),
        Expr::Call(name, args, _) => {
            let mut evaled_args: Vec<Value> = Vec::new();
            let mut arg_var_names: Vec<Option<String>> = Vec::new();
            for a in args {
                match a {
                    Expr::Var(vname, _) => {
                        // retrieve the variable value directly to allow mutation of the
                        // original variable when builtins perform in-place updates.
                        let val = env
                            .get(vname)
                            .cloned()
                            .ok_or(format!("Undefined var {}", vname))?;
                        evaled_args.push(val);
                        arg_var_names.push(Some(vname.clone()));
                    }
                    _ => {
                        evaled_args.push(eval_expr(a, env, functions)?);
                        arg_var_names.push(None);
                    }
                }
            }

            match name.as_str() {
                "panic" => {
                    if evaled_args.len() == 1 {
                        match &evaled_args[0] {
                            Value::Str(s) => Err(format!("panic: {}", s)),
                            _ => Err("panic requires string".to_string()),
                        }
                    } else {
                        Err("panic requires one argument".to_string())
                    }
                }
                "vector_new" => Ok(Value::Vector(Vec::new())),
                "vector_len" => {
                    if evaled_args.len() == 1 {
                        match &evaled_args[0] {
                            Value::Vector(v) => Ok(Value::Int(v.len() as i64)),
                            _ => Err("vector_len requires vector".to_string()),
                        }
                    } else {
                        Err("vector_len requires one argument".to_string())
                    }
                }
                // String helpers
                "str_len" => {
                    if evaled_args.len() == 1 {
                        match &evaled_args[0] {
                            Value::Str(s) => Ok(Value::Int(s.len() as i64)),
                            _ => Err("str_len requires string".to_string()),
                        }
                    } else {
                        Err("str_len requires one argument".to_string())
                    }
                }
                "string_concat" => {
                    if evaled_args.len() == 2 {
                        match (&evaled_args[0], &evaled_args[1]) {
                            (Value::Str(a), Value::Str(b)) => Ok(Value::Str(format!("{}{}", a, b))),
                            _ => Err("string_concat requires two strings".to_string()),
                        }
                    } else {
                        Err("string_concat requires two arguments".to_string())
                    }
                }
                "string_eq" => {
                    if evaled_args.len() == 2 {
                        match (&evaled_args[0], &evaled_args[1]) {
                            (Value::Str(a), Value::Str(b)) => Ok(Value::Bool(a == b)),
                            _ => Err("string_eq requires two strings".to_string()),
                        }
                    } else {
                        Err("string_eq requires two arguments".to_string())
                    }
                }
                "string_push_char" => {
                    if evaled_args.len() == 2 {
                        match (&evaled_args[0], &evaled_args[1]) {
                            (Value::Str(a), Value::Str(b)) => Ok(Value::Str(format!("{}{}", a, b))),
                            _ => Err("string_push_char requires (string,char)".to_string()),
                        }
                    } else {
                        Err("string_push_char requires two arguments".to_string())
                    }
                }
                "string_substr" => {
                    if evaled_args.len() == 3 {
                        match (&evaled_args[0], &evaled_args[1], &evaled_args[2]) {
                            (Value::Str(s), Value::Int(start), Value::Int(len)) => {
                                let start = checked_index(*start, "string_substr")?;
                                let len = checked_index(*len, "string_substr")?;
                                let out: String = s.chars().skip(start).take(len).collect();
                                Ok(Value::Str(out))
                            }
                            _ => Err("string_substr requires (string,int,int)".to_string()),
                        }
                    } else {
                        Err("string_substr requires three arguments".to_string())
                    }
                }
                "string_starts_with" => {
                    if evaled_args.len() == 2 {
                        match (&evaled_args[0], &evaled_args[1]) {
                            (Value::Str(s), Value::Str(pref)) => {
                                Ok(Value::Bool(s.starts_with(pref)))
                            }
                            _ => Err("string_starts_with requires two strings".to_string()),
                        }
                    } else {
                        Err("string_starts_with requires two arguments".to_string())
                    }
                }
                "string_ends_with" => {
                    if evaled_args.len() == 2 {
                        match (&evaled_args[0], &evaled_args[1]) {
                            (Value::Str(s), Value::Str(suf)) => Ok(Value::Bool(s.ends_with(suf))),
                            _ => Err("string_ends_with requires two strings".to_string()),
                        }
                    } else {
                        Err("string_ends_with requires two arguments".to_string())
                    }
                }
                "string_find" => {
                    if evaled_args.len() == 2 {
                        match (&evaled_args[0], &evaled_args[1]) {
                            (Value::Str(s), Value::Str(sub)) => {
                                if let Some(pos) = s.find(sub) {
                                    Ok(Value::Int(pos as i64))
                                } else {
                                    Ok(Value::Int(-1))
                                }
                            }
                            _ => Err("string_find requires two strings".to_string()),
                        }
                    } else {
                        Err("string_find requires two arguments".to_string())
                    }
                }
                "string_trim" => {
                    if evaled_args.len() == 1 {
                        match &evaled_args[0] {
                            Value::Str(s) => Ok(Value::Str(s.trim().to_string())),
                            _ => Err("string_trim requires a string".to_string()),
                        }
                    } else {
                        Err("string_trim requires one argument".to_string())
                    }
                }
                "int_to_string" => {
                    if evaled_args.len() == 1 {
                        match &evaled_args[0] {
                            Value::Int(n) => Ok(Value::Str(n.to_string())),
                            _ => Err("int_to_string requires int".to_string()),
                        }
                    } else {
                        Err("int_to_string requires one argument".to_string())
                    }
                }
                "string_to_int" => {
                    if evaled_args.len() == 1 {
                        match &evaled_args[0] {
                            Value::Str(s) => s
                                .parse::<i64>()
                                .map(Value::Int)
                                .map_err(|_| "string_to_int: invalid i64 literal".to_string()),
                            _ => Err("string_to_int requires string".to_string()),
                        }
                    } else {
                        Err("string_to_int requires one argument".to_string())
                    }
                }
                "int_abs" => {
                    if evaled_args.len() == 1 {
                        match &evaled_args[0] {
                            Value::Int(n) => n
                                .checked_abs()
                                .map(Value::Int)
                                .ok_or_else(|| "int_abs: arithmetic overflow".to_string()),
                            _ => Err("int_abs requires int".to_string()),
                        }
                    } else {
                        Err("int_abs requires one argument".to_string())
                    }
                }
                "string_replace" => {
                    if evaled_args.len() == 3 {
                        match (&evaled_args[0], &evaled_args[1], &evaled_args[2]) {
                            (Value::Str(s), Value::Str(old), Value::Str(new)) => {
                                Ok(Value::Str(s.replace(old, new)))
                            }
                            _ => Err("string_replace requires three strings".to_string()),
                        }
                    } else {
                        Err("string_replace requires three arguments".to_string())
                    }
                }
                "int_pow" => {
                    if evaled_args.len() == 2 {
                        match (&evaled_args[0], &evaled_args[1]) {
                            (Value::Int(a), Value::Int(b)) => {
                                let exponent = u32::try_from(*b).map_err(|_| {
                                    "int_pow: exponent must be a non-negative u32".to_string()
                                })?;
                                a.checked_pow(exponent)
                                    .map(Value::Int)
                                    .ok_or_else(|| "int_pow: arithmetic overflow".to_string())
                            }
                            _ => Err("int_pow requires two ints".to_string()),
                        }
                    } else {
                        Err("int_pow requires two arguments".to_string())
                    }
                }
                "int_div" => {
                    if evaled_args.len() == 2 {
                        match (&evaled_args[0], &evaled_args[1]) {
                            (Value::Int(a), Value::Int(b)) => {
                                a.checked_div(*b).map(Value::Int).ok_or_else(|| {
                                    "int_div: divide by zero or arithmetic overflow".to_string()
                                })
                            }
                            _ => Err("int_div requires two ints".to_string()),
                        }
                    } else {
                        Err("int_div requires two arguments".to_string())
                    }
                }
                // HashSet implemented as Map-backed set (string/int keys only)
                "hashset_new" => Ok(Value::Map(HashMap::new())),
                // HashMap: distinct constructor for clarity
                "hashmap_new" => Ok(Value::Map(HashMap::new())),
                "hashset_insert" => {
                    if evaled_args.len() == 2 {
                        match (&evaled_args[0], &evaled_args[1]) {
                            (Value::Map(map), Value::Str(k)) => {
                                let mut new_map = map.clone();
                                new_map.insert(k.clone(), Value::Bool(true));
                                Ok(Value::Int(new_map.len() as i64))
                            }
                            (Value::Map(map), Value::Int(i)) => {
                                let mut new_map = map.clone();
                                new_map.insert(i.to_string(), Value::Bool(true));
                                Ok(Value::Int(new_map.len() as i64))
                            }
                            _ => Err("hashset_insert requires (set,key)".to_string()),
                        }
                    } else {
                        Err("hashset_insert requires two arguments".to_string())
                    }
                }
                "hashset_contains" => {
                    if evaled_args.len() == 2 {
                        match (&evaled_args[0], &evaled_args[1]) {
                            (Value::Map(map), Value::Str(k)) => {
                                Ok(Value::Bool(map.contains_key(k)))
                            }
                            (Value::Map(map), Value::Int(i)) => {
                                Ok(Value::Bool(map.contains_key(&i.to_string())))
                            }
                            _ => Err("hashset_contains requires (set,key)".to_string()),
                        }
                    } else {
                        Err("hashset_contains requires two arguments".to_string())
                    }
                }
                "hashset_remove" => {
                    if evaled_args.len() == 2 {
                        match (&evaled_args[0], &evaled_args[1]) {
                            (Value::Map(map), Value::Str(k)) => {
                                let mut new_map = map.clone();
                                new_map.remove(k);
                                Ok(Value::Int(new_map.len() as i64))
                            }
                            (Value::Map(map), Value::Int(i)) => {
                                let mut new_map = map.clone();
                                new_map.remove(&i.to_string());
                                Ok(Value::Int(new_map.len() as i64))
                            }
                            _ => Err("hashset_remove requires (set,key)".to_string()),
                        }
                    } else {
                        Err("hashset_remove requires two arguments".to_string())
                    }
                }
                "hashset_union" => {
                    if evaled_args.len() == 2 {
                        match (&evaled_args[0], &evaled_args[1]) {
                            (Value::Map(a), Value::Map(b)) => {
                                let mut out = a.clone();
                                for (k, v) in b.iter() {
                                    out.insert(k.clone(), v.clone());
                                }
                                Ok(Value::Map(out))
                            }
                            _ => Err("hashset_union requires two sets".to_string()),
                        }
                    } else {
                        Err("hashset_union requires two arguments".to_string())
                    }
                }
                "hashset_intersect" => {
                    if evaled_args.len() == 2 {
                        match (&evaled_args[0], &evaled_args[1]) {
                            (Value::Map(a), Value::Map(b)) => {
                                let mut out = HashMap::new();
                                for (k, v) in a.iter() {
                                    if b.contains_key(k) {
                                        out.insert(k.clone(), v.clone());
                                    }
                                }
                                Ok(Value::Map(out))
                            }
                            _ => Err("hashset_intersect requires two sets".to_string()),
                        }
                    } else {
                        Err("hashset_intersect requires two arguments".to_string())
                    }
                }
                "hashset_len" => {
                    if evaled_args.len() == 1 {
                        match &evaled_args[0] {
                            Value::Map(map) => Ok(Value::Int(map.len() as i64)),
                            _ => Err("hashset_len requires a set".to_string()),
                        }
                    } else {
                        Err("hashset_len requires one argument".to_string())
                    }
                }
                "hashset_clear" => {
                    if evaled_args.len() == 1 {
                        match &evaled_args[0] {
                            Value::Map(_map) => {
                                if let Some(Some(varname)) = arg_var_names.first() {
                                    env.insert(varname.clone(), Value::Map(HashMap::new()));
                                }
                                Ok(Value::Unit)
                            }
                            _ => Err("hashset_clear requires a set".to_string()),
                        }
                    } else {
                        Err("hashset_clear requires one argument".to_string())
                    }
                }
                "option_is_some" => {
                    if evaled_args.len() == 1 {
                        match &evaled_args[0] {
                            Value::Map(map) => Ok(Value::Bool(map.contains_key("value"))),
                            _ => Err("option_is_some requires an Option value (map)".to_string()),
                        }
                    } else {
                        Err("option_is_some requires one argument".to_string())
                    }
                }
                "result_is_ok" => {
                    if evaled_args.len() == 1 {
                        match &evaled_args[0] {
                            Value::Map(map) => Ok(Value::Bool(map.contains_key("value"))),
                            _ => Err("result_is_ok requires a Result value (map)".to_string()),
                        }
                    } else {
                        Err("result_is_ok requires one argument".to_string())
                    }
                }
                "option_unwrap_or" => {
                    if evaled_args.len() == 2 {
                        match &evaled_args[0] {
                            Value::Map(map) => {
                                if let Some(v) = map.get("value") {
                                    Ok(v.clone())
                                } else {
                                    Ok(evaled_args[1].clone())
                                }
                            }
                            _ => Err("option_unwrap_or requires (option,default)".to_string()),
                        }
                    } else {
                        Err("option_unwrap_or requires two arguments".to_string())
                    }
                }
                "result_unwrap_or" => {
                    if evaled_args.len() == 2 {
                        match &evaled_args[0] {
                            Value::Map(map) => {
                                if let Some(v) = map.get("value") {
                                    Ok(v.clone())
                                } else {
                                    Ok(evaled_args[1].clone())
                                }
                            }
                            _ => Err("result_unwrap_or requires (result,default)".to_string()),
                        }
                    } else {
                        Err("result_unwrap_or requires two arguments".to_string())
                    }
                }
                "result_map" => {
                    if evaled_args.len() == 2 {
                        match &evaled_args[0] {
                            Value::Map(map) => {
                                if let Some(v) = map.get("value") {
                                    match &evaled_args[1] {
                                        Value::Str(fname) => {
                                            if let Some(Stmt::Fn { params, body, .. }) =
                                                functions.get(fname)
                                            {
                                                if params.len() != 1 {
                                                    Err(format!(
                                                        "result_map: function {} must take 1 arg",
                                                        fname
                                                    ))
                                                } else {
                                                    let mut local_env = env.clone();
                                                    local_env
                                                        .insert(params[0].0.clone(), v.clone());
                                                    if let Some(res) =
                                                        eval_block(body, &mut local_env, functions)?
                                                    {
                                                        let mut out = HashMap::new();
                                                        out.insert("value".to_string(), res);
                                                        Ok(Value::Map(out))
                                                    } else {
                                                        Ok(Value::Map(HashMap::new()))
                                                    }
                                                }
                                            } else {
                                                Err("result_map: function not found".to_string())
                                            }
                                        }
                                        _ => {
                                            Err("result_map expects a function name string"
                                                .to_string())
                                        }
                                    }
                                } else {
                                    Ok(Value::Map(map.clone()))
                                }
                            }
                            _ => Err("result_map requires a Result value (map)".to_string()),
                        }
                    } else {
                        Err("result_map requires two arguments".to_string())
                    }
                }
                "result_map_err" => {
                    if evaled_args.len() == 2 {
                        match &evaled_args[0] {
                            Value::Map(map) => {
                                if map.contains_key("value") {
                                    Ok(Value::Map(map.clone()))
                                } else {
                                    match &evaled_args[1] {
                                        Value::Str(fname) => {
                                            if let Some(Stmt::Fn { params, body, .. }) =
                                                functions.get(fname)
                                            {
                                                if params.len() != 1 {
                                                    Err(format!("result_map_err: function {} must take 1 arg", fname))
                                                } else {
                                                    let err_val = map
                                                        .get("err")
                                                        .cloned()
                                                        .ok_or_else(|| "result_map_err: malformed Result is missing err payload".to_string())?;
                                                    let mut local_env = env.clone();
                                                    local_env.insert(params[0].0.clone(), err_val);
                                                    if let Some(res) =
                                                        eval_block(body, &mut local_env, functions)?
                                                    {
                                                        let mut out = map.clone();
                                                        out.insert("err".to_string(), res);
                                                        Ok(Value::Map(out))
                                                    } else {
                                                        Ok(Value::Map(map.clone()))
                                                    }
                                                }
                                            } else {
                                                Err("result_map_err: function not found"
                                                    .to_string())
                                            }
                                        }
                                        _ => Err("result_map_err expects a function name string"
                                            .to_string()),
                                    }
                                }
                            }
                            _ => Err("result_map_err requires a Result value (map)".to_string()),
                        }
                    } else {
                        Err("result_map_err requires two arguments".to_string())
                    }
                }
                "option_map" => {
                    if evaled_args.len() == 2 {
                        match &evaled_args[0] {
                            Value::Map(map) => {
                                if let Some(v) = map.get("value") {
                                    match &evaled_args[1] {
                                        Value::Str(fname) => {
                                            if let Some(Stmt::Fn { params, body, .. }) =
                                                functions.get(fname)
                                            {
                                                if params.len() != 1 {
                                                    Err(format!(
                                                        "option_map: function {} must take 1 arg",
                                                        fname
                                                    ))
                                                } else {
                                                    let mut local_env = env.clone();
                                                    local_env
                                                        .insert(params[0].0.clone(), v.clone());
                                                    if let Some(res) =
                                                        eval_block(body, &mut local_env, functions)?
                                                    {
                                                        let mut out = HashMap::new();
                                                        out.insert("value".to_string(), res);
                                                        Ok(Value::Map(out))
                                                    } else {
                                                        Ok(Value::Map(HashMap::new()))
                                                    }
                                                }
                                            } else {
                                                Err("option_map: function not found".to_string())
                                            }
                                        }
                                        _ => {
                                            Err("option_map expects a function name string"
                                                .to_string())
                                        }
                                    }
                                } else {
                                    Ok(Value::Map(map.clone()))
                                }
                            }
                            _ => Err("option_map requires an Option value (map)".to_string()),
                        }
                    } else {
                        Err("option_map requires two arguments".to_string())
                    }
                }
                "option_and" => {
                    if evaled_args.len() == 2 {
                        match &evaled_args[0] {
                            Value::Map(map) => {
                                if map.contains_key("value") {
                                    Ok(evaled_args[1].clone())
                                } else {
                                    Ok(Value::Map(map.clone()))
                                }
                            }
                            _ => Err("option_and requires an Option value (map)".to_string()),
                        }
                    } else {
                        Err("option_and requires two arguments".to_string())
                    }
                }
                "option_flat_map" => {
                    if evaled_args.len() == 2 {
                        match &evaled_args[0] {
                            Value::Map(map) => {
                                if let Some(v) = map.get("value") {
                                    match &evaled_args[1] {
                                        Value::Str(fname) => {
                                            if let Some(Stmt::Fn { params, body, .. }) =
                                                functions.get(fname)
                                            {
                                                if params.len() != 1 {
                                                    Err(format!("option_flat_map: function {} must take 1 arg", fname))
                                                } else {
                                                    let mut local_env = env.clone();
                                                    local_env
                                                        .insert(params[0].0.clone(), v.clone());
                                                    if let Some(res) =
                                                        eval_block(body, &mut local_env, functions)?
                                                    {
                                                        Ok(res)
                                                    } else {
                                                        Ok(Value::Map(HashMap::new()))
                                                    }
                                                }
                                            } else {
                                                Err("option_flat_map: function not found"
                                                    .to_string())
                                            }
                                        }
                                        _ => Err("option_flat_map expects a function name string"
                                            .to_string()),
                                    }
                                } else {
                                    Ok(Value::Map(map.clone()))
                                }
                            }
                            _ => Err("option_flat_map requires an Option value (map)".to_string()),
                        }
                    } else {
                        Err("option_flat_map requires two arguments".to_string())
                    }
                }
                "option_or_else" => {
                    if evaled_args.len() == 2 {
                        match &evaled_args[0] {
                            Value::Map(map) => {
                                if map.contains_key("value") {
                                    Ok(Value::Map(map.clone()))
                                } else {
                                    match &evaled_args[1] {
                                        Value::Str(fname) => {
                                            if let Some(Stmt::Fn { params, body, .. }) =
                                                functions.get(fname)
                                            {
                                                if !params.is_empty() {
                                                    Err(format!("option_or_else: function {} must take 0 args", fname))
                                                } else {
                                                    let mut local_env = env.clone();
                                                    if let Some(res) =
                                                        eval_block(body, &mut local_env, functions)?
                                                    {
                                                        Ok(res)
                                                    } else {
                                                        Ok(Value::Map(HashMap::new()))
                                                    }
                                                }
                                            } else {
                                                Err("option_or_else: function not found"
                                                    .to_string())
                                            }
                                        }
                                        _ => Err("option_or_else expects a function name string"
                                            .to_string()),
                                    }
                                }
                            }
                            _ => Err("option_or_else requires an Option value (map)".to_string()),
                        }
                    } else {
                        Err("option_or_else requires two arguments".to_string())
                    }
                }
                "option_zip" => {
                    if evaled_args.len() == 2 {
                        match (&evaled_args[0], &evaled_args[1]) {
                            (Value::Map(a), Value::Map(b)) => {
                                match (a.get("value"), b.get("value")) {
                                    (Some(va), Some(vb)) => {
                                        let mut out = HashMap::new();
                                        out.insert(
                                            "value".to_string(),
                                            Value::Vector(vec![va.clone(), vb.clone()]),
                                        );
                                        Ok(Value::Map(out))
                                    }
                                    _ => Ok(Value::Map(HashMap::new())),
                                }
                            }
                            _ => Err("option_zip requires two Option values".to_string()),
                        }
                    } else {
                        Err("option_zip requires two arguments".to_string())
                    }
                }
                "option_transpose" => {
                    if evaled_args.len() == 1 {
                        match &evaled_args[0] {
                            Value::Map(map) => {
                                if let Some(v) = map.get("value") {
                                    match v {
                                        Value::Map(inner) => {
                                            if let Some(ok_val) = inner.get("value") {
                                                // Some(Ok(v)) → Ok(Some(v)) → Map{"value": Map{"value": v}}
                                                let mut inner_out = HashMap::new();
                                                inner_out
                                                    .insert("value".to_string(), ok_val.clone());
                                                let mut out = HashMap::new();
                                                out.insert(
                                                    "value".to_string(),
                                                    Value::Map(inner_out),
                                                );
                                                Ok(Value::Map(out))
                                            } else if let Some(err_val) = inner.get("err") {
                                                // Some(Err(e)) → Err(e) → Map{"err": e}
                                                let mut out = HashMap::new();
                                                out.insert("err".to_string(), err_val.clone());
                                                Ok(Value::Map(out))
                                            } else {
                                                Ok(Value::Map(HashMap::new()))
                                            }
                                        }
                                        _ => Err("option_transpose requires Option<Result> value"
                                            .to_string()),
                                    }
                                } else {
                                    // None → Ok(None) → Map{"value": {}}
                                    let mut out = HashMap::new();
                                    out.insert("value".to_string(), Value::Map(HashMap::new()));
                                    Ok(Value::Map(out))
                                }
                            }
                            _ => Err("option_transpose requires an Option value".to_string()),
                        }
                    } else {
                        Err("option_transpose requires one argument".to_string())
                    }
                }
                "result_flat_map" => {
                    if evaled_args.len() == 2 {
                        match &evaled_args[0] {
                            Value::Map(map) => {
                                if let Some(v) = map.get("value") {
                                    match &evaled_args[1] {
                                        Value::Str(fname) => {
                                            if let Some(Stmt::Fn { params, body, .. }) =
                                                functions.get(fname)
                                            {
                                                if params.len() != 1 {
                                                    Err(format!("result_flat_map: function {} must take 1 arg", fname))
                                                } else {
                                                    let mut local_env = env.clone();
                                                    local_env
                                                        .insert(params[0].0.clone(), v.clone());
                                                    if let Some(res) =
                                                        eval_block(body, &mut local_env, functions)?
                                                    {
                                                        Ok(res)
                                                    } else {
                                                        Ok(Value::Map(HashMap::new()))
                                                    }
                                                }
                                            } else {
                                                Err("result_flat_map: function not found"
                                                    .to_string())
                                            }
                                        }
                                        _ => Err("result_flat_map expects a function name string"
                                            .to_string()),
                                    }
                                } else {
                                    Ok(Value::Map(map.clone()))
                                }
                            }
                            _ => Err("result_flat_map requires a Result value (map)".to_string()),
                        }
                    } else {
                        Err("result_flat_map requires two arguments".to_string())
                    }
                }
                "result_or_else" => {
                    if evaled_args.len() == 2 {
                        match &evaled_args[0] {
                            Value::Map(map) => {
                                if map.contains_key("value") {
                                    Ok(Value::Map(map.clone()))
                                } else {
                                    match &evaled_args[1] {
                                        Value::Str(fname) => {
                                            if let Some(Stmt::Fn { params, body, .. }) =
                                                functions.get(fname)
                                            {
                                                if params.len() != 1 {
                                                    Err(format!("result_or_else: function {} must take 1 arg (the error value)", fname))
                                                } else {
                                                    let err_val = map
                                                        .get("err")
                                                        .cloned()
                                                        .ok_or_else(|| "result_or_else: malformed Result is missing err payload".to_string())?;
                                                    let mut local_env = env.clone();
                                                    local_env.insert(params[0].0.clone(), err_val);
                                                    if let Some(res) =
                                                        eval_block(body, &mut local_env, functions)?
                                                    {
                                                        Ok(res)
                                                    } else {
                                                        Ok(Value::Map(HashMap::new()))
                                                    }
                                                }
                                            } else {
                                                Err("result_or_else: function not found"
                                                    .to_string())
                                            }
                                        }
                                        _ => Err("result_or_else expects a function name string"
                                            .to_string()),
                                    }
                                }
                            }
                            _ => Err("result_or_else requires a Result value (map)".to_string()),
                        }
                    } else {
                        Err("result_or_else requires two arguments".to_string())
                    }
                }
                "result_transpose" => {
                    if evaled_args.len() == 1 {
                        match &evaled_args[0] {
                            Value::Map(map) => {
                                if let Some(v) = map.get("value") {
                                    match v {
                                        Value::Map(inner) => {
                                            if let Some(val) = inner.get("value") {
                                                // Ok(Some(v)) → Some(Ok(v)) → Map{"value": Map{"value": v}}
                                                let mut inner_out = HashMap::new();
                                                inner_out.insert("value".to_string(), val.clone());
                                                let mut out = HashMap::new();
                                                out.insert(
                                                    "value".to_string(),
                                                    Value::Map(inner_out),
                                                );
                                                Ok(Value::Map(out))
                                            } else {
                                                // Ok(None) → None → Map{}
                                                Ok(Value::Map(HashMap::new()))
                                            }
                                        }
                                        _ => Err("result_transpose requires Result<Option> value"
                                            .to_string()),
                                    }
                                } else if let Some(err_val) = map.get("err") {
                                    // Err(e) → Some(Err(e)) → Map{"value": Map{"err": e}}
                                    let mut inner_out = HashMap::new();
                                    inner_out.insert("err".to_string(), err_val.clone());
                                    let mut out = HashMap::new();
                                    out.insert("value".to_string(), Value::Map(inner_out));
                                    Ok(Value::Map(out))
                                } else {
                                    Err("result_transpose requires a Result value".to_string())
                                }
                            }
                            _ => Err("result_transpose requires a Result value (map)".to_string()),
                        }
                    } else {
                        Err("result_transpose requires one argument".to_string())
                    }
                }
                "option_filter" => {
                    if evaled_args.len() == 2 {
                        match &evaled_args[0] {
                            Value::Map(map) => {
                                if let Some(v) = map.get("value") {
                                    match &evaled_args[1] {
                                        Value::Str(fname) => {
                                            if let Some(Stmt::Fn { params, body, .. }) =
                                                functions.get(fname)
                                            {
                                                if params.len() != 1 {
                                                    Err(format!("option_filter: function {} must take 1 arg", fname))
                                                } else {
                                                    let mut local_env = env.clone();
                                                    local_env
                                                        .insert(params[0].0.clone(), v.clone());
                                                    match eval_block(body, &mut local_env, functions)? {
                                                        Some(Value::Bool(true)) | Some(Value::Int(1)) => {
                                                            Ok(Value::Map(map.clone()))
                                                        }
                                                        Some(Value::Bool(false)) | Some(Value::Int(0)) => {
                                                            Ok(Value::Map(HashMap::new()))
                                                        }
                                                        _ => Err("option_filter function must return bool".to_string()),
                                                    }
                                                }
                                            } else {
                                                Err("option_filter: function not found".to_string())
                                            }
                                        }
                                        _ => Err("option_filter expects a function name string"
                                            .to_string()),
                                    }
                                } else {
                                    Ok(Value::Map(map.clone()))
                                }
                            }
                            _ => Err("option_filter requires an Option value (map)".to_string()),
                        }
                    } else {
                        Err("option_filter requires two arguments".to_string())
                    }
                }
                // Vector helpers
                "vector_push" => {
                    if evaled_args.len() == 2 {
                        match &evaled_args[0] {
                            Value::Vector(vec) => {
                                let mut new_vec = vec.clone();
                                new_vec.push(evaled_args[1].clone());
                                // if the first arg was a variable, update it in the environment
                                if let Some(Some(varname)) = arg_var_names.first() {
                                    env.insert(varname.clone(), Value::Vector(new_vec.clone()));
                                }
                                Ok(Value::Int(new_vec.len() as i64))
                            }
                            _ => Err("vector_push requires a vector as first argument".to_string()),
                        }
                    } else {
                        Err("vector_push requires two arguments".to_string())
                    }
                }
                "vector_get" => {
                    if evaled_args.len() == 2 {
                        match (&evaled_args[0], &evaled_args[1]) {
                            (Value::Vector(vec), Value::Int(i)) => {
                                let idx = checked_index(*i, "vector_get")?;
                                vec.get(idx)
                                    .cloned()
                                    .ok_or_else(|| "vector_get: index out of bounds".to_string())
                            }
                            _ => Err("vector_get requires (vector,int)".to_string()),
                        }
                    } else {
                        Err("vector_get requires two arguments".to_string())
                    }
                }
                "vector_set" => {
                    if evaled_args.len() == 3 {
                        match (&evaled_args[0], &evaled_args[1], &evaled_args[2]) {
                            (Value::Vector(vec), Value::Int(i), val) => {
                                let mut new_vec = vec.clone();
                                let idx = checked_index(*i, "vector_set")?;
                                if idx < new_vec.len() {
                                    new_vec[idx] = val.clone();
                                    if let Some(Some(varname)) = arg_var_names.first() {
                                        env.insert(varname.clone(), Value::Vector(new_vec.clone()));
                                    }
                                    Ok(Value::Int(new_vec.len() as i64))
                                } else {
                                    Err("vector_set: index out of bounds".to_string())
                                }
                            }
                            _ => Err("vector_set requires (vector,int,val)".to_string()),
                        }
                    } else {
                        Err("vector_set requires three arguments".to_string())
                    }
                }
                "vector_pop" => {
                    if evaled_args.len() == 1 {
                        match &evaled_args[0] {
                            Value::Vector(vec) => {
                                let mut new_vec = vec.clone();
                                let Some(val) = new_vec.pop() else {
                                    return Err("vector_pop: empty vector".to_string());
                                };
                                if let Some(Some(varname)) = arg_var_names.first() {
                                    env.insert(varname.clone(), Value::Vector(new_vec));
                                }
                                Ok(val)
                            }
                            _ => Err("vector_pop requires a vector".to_string()),
                        }
                    } else {
                        Err("vector_pop requires one argument".to_string())
                    }
                }
                "vector_insert" => {
                    if evaled_args.len() == 3 {
                        match (&evaled_args[0], &evaled_args[1], &evaled_args[2]) {
                            (Value::Vector(vec), Value::Int(i), val) => {
                                let mut new_vec = vec.clone();
                                let idx = checked_index(*i, "vector_insert")?;
                                if idx <= new_vec.len() {
                                    new_vec.insert(idx, val.clone());
                                    if let Some(Some(varname)) = arg_var_names.first() {
                                        env.insert(varname.clone(), Value::Vector(new_vec.clone()));
                                    }
                                    Ok(Value::Int(new_vec.len() as i64))
                                } else {
                                    Err("vector_insert: index out of bounds".to_string())
                                }
                            }
                            _ => Err("vector_insert requires (vector,int,val)".to_string()),
                        }
                    } else {
                        Err("vector_insert requires three arguments".to_string())
                    }
                }
                "vector_remove" => {
                    if evaled_args.len() == 2 {
                        match (&evaled_args[0], &evaled_args[1]) {
                            (Value::Vector(vec), Value::Int(i)) => {
                                let mut new_vec = vec.clone();
                                let idx = checked_index(*i, "vector_remove")?;
                                if idx < new_vec.len() {
                                    let removed = new_vec.remove(idx);
                                    if let Some(Some(varname)) = arg_var_names.first() {
                                        env.insert(varname.clone(), Value::Vector(new_vec));
                                    }
                                    Ok(removed)
                                } else {
                                    Err("vector_remove: index out of bounds".to_string())
                                }
                            }
                            _ => Err("vector_remove requires (vector,int)".to_string()),
                        }
                    } else {
                        Err("vector_remove requires two arguments".to_string())
                    }
                }
                "vector_clear" => {
                    if evaled_args.len() == 1 {
                        match &evaled_args[0] {
                            Value::Vector(_vec) => {
                                if let Some(Some(varname)) = arg_var_names.first() {
                                    env.insert(varname.clone(), Value::Vector(Vec::new()));
                                }
                                Ok(Value::Unit)
                            }
                            _ => Err("vector_clear requires a vector".to_string()),
                        }
                    } else {
                        Err("vector_clear requires one argument".to_string())
                    }
                }
                "vector_contains" => {
                    if evaled_args.len() == 2 {
                        match (&evaled_args[0], &evaled_args[1]) {
                            (Value::Vector(vec), val) => Ok(Value::Bool(vec.contains(val))),
                            _ => Err("vector_contains requires a vector and a value".to_string()),
                        }
                    } else {
                        Err("vector_contains requires two arguments".to_string())
                    }
                }
                "vector_capacity" => {
                    if evaled_args.len() == 1 {
                        match &evaled_args[0] {
                            Value::Vector(vec) => Ok(Value::Int(vec.capacity() as i64)),
                            _ => Err("vector_capacity requires a vector".to_string()),
                        }
                    } else {
                        Err("vector_capacity requires one argument".to_string())
                    }
                }
                "vector_reserve" => {
                    if evaled_args.len() == 2 {
                        match (&evaled_args[0], &evaled_args[1]) {
                            (Value::Vector(vec), Value::Int(extra)) => {
                                let additional = checked_index(*extra, "vector_reserve")?;
                                let mut new_vec = vec.clone();
                                new_vec.reserve(additional);
                                if let Some(Some(varname)) = arg_var_names.first() {
                                    env.insert(varname.clone(), Value::Vector(new_vec));
                                    Ok(Value::Unit)
                                } else {
                                    Err("vector_reserve requires a mutable named vector in the legacy interpreter".to_string())
                                }
                            }
                            _ => Err("vector_reserve requires (vector,int)".to_string()),
                        }
                    } else {
                        Err("vector_reserve requires two arguments".to_string())
                    }
                }
                // HashMap helpers
                "hashmap_insert" => {
                    if evaled_args.len() == 3 {
                        match (&evaled_args[0], &evaled_args[1], &evaled_args[2]) {
                            (Value::Map(map), Value::Str(k), v) => {
                                let mut new_map = map.clone();
                                new_map.insert(k.clone(), v.clone());
                                if let Some(Some(varname)) = arg_var_names.first() {
                                    env.insert(varname.clone(), Value::Map(new_map.clone()));
                                }
                                Ok(Value::Int(new_map.len() as i64))
                            }
                            _ => Err("hashmap_insert requires (map,string,value)".to_string()),
                        }
                    } else {
                        Err("hashmap_insert requires three arguments".to_string())
                    }
                }
                "hashmap_get" => {
                    if evaled_args.len() == 2 {
                        match (&evaled_args[0], &evaled_args[1]) {
                            (Value::Map(map), Value::Str(k)) => map
                                .get(k)
                                .cloned()
                                .ok_or_else(|| format!("hashmap_get: key '{}' not found", k)),
                            _ => Err("hashmap_get requires (map,string)".to_string()),
                        }
                    } else {
                        Err("hashmap_get requires two arguments".to_string())
                    }
                }
                "hashmap_remove" => {
                    if evaled_args.len() == 2 {
                        match (&evaled_args[0], &evaled_args[1]) {
                            (Value::Map(map), Value::Str(k)) => {
                                let mut new_map = map.clone();
                                new_map.remove(k);
                                if let Some(Some(varname)) = arg_var_names.first() {
                                    env.insert(varname.clone(), Value::Map(new_map.clone()));
                                }
                                Ok(Value::Int(new_map.len() as i64))
                            }
                            _ => Err("hashmap_remove requires (map,string)".to_string()),
                        }
                    } else {
                        Err("hashmap_remove requires two arguments".to_string())
                    }
                }
                "hashmap_len" => {
                    if evaled_args.len() == 1 {
                        match &evaled_args[0] {
                            Value::Map(map) => Ok(Value::Int(map.len() as i64)),
                            _ => Err("hashmap_len requires a map".to_string()),
                        }
                    } else {
                        Err("hashmap_len requires one argument".to_string())
                    }
                }
                "hashmap_contains" => {
                    if evaled_args.len() == 2 {
                        match (&evaled_args[0], &evaled_args[1]) {
                            (Value::Map(map), Value::Str(k)) => {
                                Ok(Value::Bool(map.contains_key(k)))
                            }
                            _ => Err("hashmap_contains requires (map,string)".to_string()),
                        }
                    } else {
                        Err("hashmap_contains requires two arguments".to_string())
                    }
                }
                "hashmap_clear" => {
                    if evaled_args.len() == 1 {
                        match &evaled_args[0] {
                            Value::Map(_) => {
                                if let Some(Some(varname)) = arg_var_names.first() {
                                    env.insert(varname.clone(), Value::Map(HashMap::new()));
                                    Ok(Value::Unit)
                                } else {
                                    Err("hashmap_clear requires a mutable named map in the legacy interpreter".to_string())
                                }
                            }
                            _ => Err("hashmap_clear requires a map".to_string()),
                        }
                    } else {
                        Err("hashmap_clear requires one argument".to_string())
                    }
                }
                _ => {
                    // Check if this name is handled by an active effect handler
                    if let Some(handler_val) = resolve_handler(name) {
                        // Route through handler: call the handler's function
                        if let Value::Closure {
                            params,
                            body,
                            env: captured_env,
                        } = handler_val
                        {
                            if params.len() != evaled_args.len() {
                                Err(format!(
                                    "Effect handler '{}' expected {} args, got {}",
                                    name,
                                    params.len(),
                                    evaled_args.len()
                                ))
                            } else {
                                let mut local_env = (*captured_env).clone();
                                for (p, a) in params.iter().zip(evaled_args.iter()) {
                                    local_env.insert(p.0.clone(), a.clone());
                                }
                                let val = eval_expr(&body, &mut local_env, functions)?;
                                Ok(val)
                            }
                        } else {
                            // Handler value is not a closure — fall through to normal function call
                            if let Some(Stmt::Fn { params, body, .. }) = functions.get(name) {
                                if params.len() != evaled_args.len() {
                                    Err(format!(
                                        "Expected {} args for function {}",
                                        params.len(),
                                        name
                                    ))
                                } else {
                                    let mut local_env = env.clone();
                                    for (p, a) in params.iter().zip(evaled_args.iter()) {
                                        local_env.insert(p.0.clone(), a.clone());
                                    }
                                    if let Some(val) = eval_block(body, &mut local_env, functions)?
                                    {
                                        Ok(val)
                                    } else {
                                        Ok(Value::Unit)
                                    }
                                }
                            } else {
                                Err(format!("Undefined function: {}", name))
                            }
                        }
                    } else {
                        if let Some(Stmt::Fn { params, body, .. }) = functions.get(name) {
                            if params.len() != evaled_args.len() {
                                Err(format!(
                                    "Expected {} args for function {}",
                                    params.len(),
                                    name
                                ))
                            } else {
                                let mut local_env = env.clone();
                                for (p, a) in params.iter().zip(evaled_args.iter()) {
                                    local_env.insert(p.0.clone(), a.clone());
                                }
                                if let Some(val) = eval_block(body, &mut local_env, functions)? {
                                    Ok(val)
                                } else {
                                    Ok(Value::Unit)
                                }
                            }
                        } else {
                            Err(format!("Undefined function: {}", name))
                        }
                    }
                }
            }
        }
        Expr::Interpolated(frags, _) => {
            let mut out = String::new();
            for frag in frags.iter() {
                match frag {
                    InterpolatedFragment::Literal(s, _) => out.push_str(s),
                    InterpolatedFragment::Expr(e) => {
                        let v = eval_expr(e, env, functions)?;
                        out.push_str(&value_to_string(&v));
                    }
                }
            }
            Ok(Value::Str(out))
        }
        Expr::BinaryOp {
            op, left, right, ..
        } => {
            let l = eval_expr(left, env, functions)?;
            let r = eval_expr(right, env, functions)?;
            match (l, r) {
                (Value::Int(a), Value::Int(b)) => {
                    let res_int = match op {
                        TokenKind::Plus => a
                            .checked_add(b)
                            .ok_or_else(|| "integer addition overflow".to_string())?,
                        TokenKind::Minus => a
                            .checked_sub(b)
                            .ok_or_else(|| "integer subtraction overflow".to_string())?,
                        TokenKind::Star => a
                            .checked_mul(b)
                            .ok_or_else(|| "integer multiplication overflow".to_string())?,
                        TokenKind::Slash => a
                            .checked_div(b)
                            .ok_or_else(|| "integer division by zero or overflow".to_string())?,
                        TokenKind::Percent => a
                            .checked_rem(b)
                            .ok_or_else(|| "integer remainder by zero or overflow".to_string())?,
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
                        _ => return Err("Unsupported binary op for ints".to_string()),
                    };
                    Ok(Value::Int(res_int))
                }
                (Value::Str(a), Value::Str(b)) => match op {
                    TokenKind::Plus => Ok(Value::Str(format!("{}{}", a, b))),
                    TokenKind::EqEq => Ok(Value::Int(if a == b { 1 } else { 0 })),
                    TokenKind::NotEq => Ok(Value::Int(if a != b { 1 } else { 0 })),
                    _ => Err("Unsupported binary op for strings".to_string()),
                },
                _ => Err("Unsupported binary op types".to_string()),
            }
        }
        Expr::Borrow { .. } | Expr::Deref { .. } => Err(
            "safe reference execution is not qualified in the bootstrap reference interpreter"
                .to_string(),
        ),
        Expr::UnaryOp { op, inner, .. } => {
            let v = eval_expr(inner, env, functions)?;
            match op {
                TokenKind::Minus => match v {
                    Value::Int(n) => n
                        .checked_neg()
                        .map(Value::Int)
                        .ok_or_else(|| "integer negation overflow".to_string()),
                    _ => Err("Unary - requires int".to_string()),
                },
                TokenKind::Bang => match v {
                    Value::Bool(b) => Ok(Value::Bool(!b)),
                    _ => Err("Unary ! requires bool".to_string()),
                },
                _ => Err("Unsupported unary op".to_string()),
            }
        }
        Expr::FieldAccess { base, field, .. } => {
            let base_value = eval_expr(base, env, functions)?;
            match base_value {
                Value::Str(s) if field == "len" => Ok(Value::Int(s.chars().count() as i64)),
                Value::Vector(vec) if field == "len" => Ok(Value::Int(vec.len() as i64)),
                Value::Map(map) => map
                    .get(field)
                    .cloned()
                    .ok_or_else(|| format!("Unknown field '{}'", field)),
                Value::Str(s) if field == "is_empty" => Ok(Value::Bool(s.is_empty())),
                Value::Vector(vec) if field == "is_empty" => Ok(Value::Bool(vec.is_empty())),
                Value::Vector(vec) if field == "first" => vec
                    .first()
                    .cloned()
                    .ok_or_else(|| "Vector is empty".to_string()),
                Value::Vector(vec) if field == "last" => vec
                    .last()
                    .cloned()
                    .ok_or_else(|| "Vector is empty".to_string()),
                Value::Vector(vec) if field == "clone" => Ok(Value::Vector(vec.clone())),
                Value::Str(s) if field == "clone" => Ok(Value::Str(s.clone())),
                // Removed unreachable branches: handled by the generic Map lookup above
                other => Err(format!(
                    "FieldAccess not implemented for {:?} on field '{}'",
                    other, field
                )),
            }
        }
        Expr::IfExpr {
            cond, then, else_, ..
        } => {
            let c = eval_expr(cond, env, functions)?;
            if truthy(&c) {
                eval_expr(then, env, functions)
            } else {
                eval_expr(else_, env, functions)
            }
        }
        Expr::Block(stmts, _) => {
            let mut local = env.clone();
            eval_block(stmts, &mut local, functions)?;
            Ok(Value::Unit)
        }
        Expr::Tuple(exprs, _) | Expr::Array(exprs, _) => {
            let mut values = Vec::new();
            for expr in exprs {
                values.push(eval_expr(expr, env, functions)?);
            }
            Ok(Value::Vector(values))
        }
        Expr::Index(base, index, _) => {
            let base_value = eval_expr(base, env, functions)?;
            let index_value = eval_expr(index, env, functions)?;
            match (base_value, index_value) {
                (Value::Vector(values), Value::Int(idx)) => {
                    let position = checked_index(idx, "vector index")?;
                    values
                        .get(position)
                        .cloned()
                        .ok_or_else(|| "Index out of bounds".to_string())
                }
                (Value::Str(text), Value::Int(idx)) => {
                    let position = checked_index(idx, "string index")?;
                    text.chars()
                        .nth(position)
                        .map(|ch| Value::Str(ch.to_string()))
                        .ok_or_else(|| "Index out of bounds".to_string())
                }
                _ => Err("Index evaluation not implemented for these operands".to_string()),
            }
        }
        Expr::Match { expr, arms, .. } => {
            let scrutinee = eval_expr(expr, env, functions)?;

            for arm in arms {
                let Some(bindings) = match_pattern(&arm.pattern, &scrutinee) else {
                    continue;
                };

                let mut local_env = env.clone();
                for (name, value) in bindings {
                    local_env.insert(name, value);
                }

                if let Some(guard) = &arm.guard {
                    let guard_value = eval_expr(guard, &mut local_env, functions)?;
                    if !truthy(&guard_value) {
                        continue;
                    }
                }

                return eval_expr(&arm.body, &mut local_env, functions);
            }

            Err("Non-exhaustive match expression".to_string())
        }
        Expr::Range {
            start,
            end,
            inclusive,
            ..
        } => {
            let start_val = eval_expr(start, env, functions)?;
            let end_val = eval_expr(end, env, functions)?;

            match (start_val, end_val) {
                (Value::Int(s), Value::Int(e)) => {
                    let end_idx = if *inclusive { e + 1 } else { e };
                    let vec: Vec<Value> = (s..end_idx).map(Value::Int).collect();
                    Ok(Value::Vector(vec))
                }
                _ => Err("Range only supports integer bounds".to_string()),
            }
        }
        Expr::Lambda { params, body, .. } => {
            let captured_env = env.clone();
            Ok(Value::Closure {
                params: params.clone(),
                body: body.clone(),
                env: Box::new(captured_env),
            })
        }
        Expr::Await(inner, _) => {
            let val = eval_expr(inner, env, functions)?;
            // await on a ready value is a no-op
            Ok(val)
        }
        Expr::Try(inner, _) => {
            let val = eval_expr(inner, env, functions)?;
            match val {
                Value::Result(Ok(v)) => Ok(*v),
                Value::Result(Err(e)) => Err(format!("Propagated error: {:?}", e)),
                other => Ok(other),
            }
        }
        Expr::StructLit { name, fields, .. } => {
            let mut map = HashMap::new();
            for (k, v) in fields {
                let val = eval_expr(v, env, functions)?;
                map.insert(k.clone(), val);
            }
            Ok(Value::Record(name.clone(), map))
        }
    }
}

fn run_deferred_cleanups(
    cleanups: Vec<Stmt>,
    env: &mut HashMap<String, Value>,
    functions: &HashMap<String, &Stmt>,
) -> Result<(), String> {
    for cleanup in cleanups.into_iter().rev() {
        match cleanup {
            Stmt::Block(body, _) => {
                let _ = eval_block(&body, env, functions)?;
            }
            other => {
                let single = vec![other];
                let _ = eval_block(&single, env, functions)?;
            }
        }
    }
    Ok(())
}

fn eval_block(
    stmts: &[Stmt],
    env: &mut HashMap<String, Value>,
    functions: &HashMap<String, &Stmt>,
) -> Result<Option<Value>, String> {
    // Track handler stack depth to scope effect handlers properly.
    // Handlers registered within this block are popped when the block exits.
    let handler_depth = HANDLER_STACK.with(|stack| stack.borrow().len());
    let defer_depth = DEFER_STACK.with(|stack| stack.borrow().len());
    DEFER_STACK.with(|stack| stack.borrow_mut().push(Vec::new()));
    let result = eval_block_inner(stmts, env, functions);
    let cleanups = DEFER_STACK.with(|stack| stack.borrow_mut().pop().unwrap_or_default());
    // Pop any handlers added during this block
    HANDLER_STACK.with(|stack| {
        let mut stack = stack.borrow_mut();
        while stack.len() > handler_depth {
            stack.pop();
        }
    });
    DEFER_STACK.with(|stack| {
        let mut stack = stack.borrow_mut();
        while stack.len() > defer_depth {
            stack.pop();
        }
    });
    if result.is_ok() {
        run_deferred_cleanups(cleanups, env, functions)?;
    }
    result
}

fn eval_block_inner(
    stmts: &[Stmt],
    env: &mut HashMap<String, Value>,
    functions: &HashMap<String, &Stmt>,
) -> Result<Option<Value>, String> {
    for stmt in stmts {
        match stmt {
            Stmt::Annotation(_, _) => {}
            Stmt::Let(name, _type_ann, expr, _) | Stmt::LetMut(name, _type_ann, expr, _) => {
                let val = eval_expr(expr, env, functions)?;
                env.insert(name.clone(), val);
            }
            Stmt::Print(expr, _) => {
                let val = eval_expr(expr, env, functions)?;
                match val {
                    Value::Unit => println!("()"),
                    Value::Int(n) => println!("{}", n),
                    Value::Float(n) => println!("{}", n),
                    Value::Char(c) => println!("{}", c),
                    Value::Byte(b) => println!("{}", b),
                    Value::Str(s) => println!("{}", s),
                    Value::Bytes(bytes) => println!("{:?}", bytes),
                    Value::Bool(b) => println!("{}", b),
                    Value::Vector(v) => println!("<vector len={}>", v.len()),
                    Value::Map(m) => println!("<map len={}>", m.len()),
                    Value::Channel(_) => println!("<channel>"),
                    Value::CancellationToken(_) => println!("<cancellation_token>"),
                    Value::Closure { .. } => println!("<fn>"),
                    Value::Result(Ok(v)) => println!("Ok({})", value_to_string(v.as_ref())),
                    Value::Result(Err(v)) => println!("Err({})", value_to_string(v.as_ref())),
                    Value::Record(name, _) => println!("{} {{ ... }}", name),
                }
            }
            Stmt::ExprStmt(expr, _) => {
                let _ = eval_expr(expr, env, functions)?;
            }
            Stmt::Block(inner, _) => {
                if let Some(val) = eval_block(inner, env, functions)? {
                    return Ok(Some(val));
                }
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
                ..
            } => {
                let c = eval_expr(cond, env, functions)?;
                if truthy(&c) {
                    if let Some(val) = eval_block(then_body, env, functions)? {
                        return Ok(Some(val));
                    }
                } else {
                    if let Some(val) = eval_block(else_body, env, functions)? {
                        return Ok(Some(val));
                    }
                }
            }
            Stmt::Loop { body, .. } => loop {
                if let Some(val) = eval_block(body, env, functions)? {
                    return Ok(Some(val));
                }
                // Check for break/continue signals from the loop body
                if let Some(cf) = CONTROL_FLOW.with(|stack| stack.borrow_mut().take()) {
                    match cf {
                        ControlFlow::Break => break,
                        ControlFlow::Continue => continue,
                    }
                }
            },
            Stmt::For {
                var_name,
                iterable,
                body,
                ..
            } => {
                let iter_val = eval_expr(iterable, env, functions)?;
                match iter_val {
                    Value::Int(n) => {
                        for i in 0..n {
                            env.insert(var_name.clone(), Value::Int(i));
                            if let Some(val) = eval_block(body, env, functions)? {
                                return Ok(Some(val));
                            }
                            if let Some(cf) = CONTROL_FLOW.with(|stack| stack.borrow_mut().take()) {
                                match cf {
                                    ControlFlow::Break => break,
                                    ControlFlow::Continue => continue,
                                }
                            }
                        }
                    }
                    Value::Str(s) => {
                        for c in s.chars() {
                            env.insert(var_name.clone(), Value::Str(c.to_string()));
                            if let Some(val) = eval_block(body, env, functions)? {
                                return Ok(Some(val));
                            }
                            if let Some(cf) = CONTROL_FLOW.with(|stack| stack.borrow_mut().take()) {
                                match cf {
                                    ControlFlow::Break => break,
                                    ControlFlow::Continue => continue,
                                }
                            }
                        }
                    }
                    Value::Vector(v) => {
                        for elem in v {
                            env.insert(var_name.clone(), elem);
                            if let Some(val) = eval_block(body, env, functions)? {
                                return Ok(Some(val));
                            }
                            if let Some(cf) = CONTROL_FLOW.with(|stack| stack.borrow_mut().take()) {
                                match cf {
                                    ControlFlow::Break => break,
                                    ControlFlow::Continue => continue,
                                }
                            }
                        }
                    }
                    Value::Map(m) => {
                        for key in m.keys() {
                            env.insert(var_name.clone(), Value::Str(key.clone()));
                            if let Some(val) = eval_block(body, env, functions)? {
                                return Ok(Some(val));
                            }
                            if let Some(cf) = CONTROL_FLOW.with(|stack| stack.borrow_mut().take()) {
                                match cf {
                                    ControlFlow::Break => break,
                                    ControlFlow::Continue => continue,
                                }
                            }
                        }
                    }
                    _ => {
                        return Err(
                            "For loop requires int, string, vector, or map iterable".to_string()
                        )
                    }
                }
            }
            Stmt::While { cond, body, .. } => {
                while truthy(&eval_expr(cond, env, functions)?) {
                    if let Some(val) = eval_block(body, env, functions)? {
                        return Ok(Some(val));
                    }
                    if let Some(cf) = CONTROL_FLOW.with(|stack| stack.borrow_mut().take()) {
                        match cf {
                            ControlFlow::Break => break,
                            ControlFlow::Continue => continue,
                        }
                    }
                }
            }
            Stmt::Return(expr, _) => {
                let val = eval_expr(expr, env, functions)?;
                return Ok(Some(val));
            }
            Stmt::Break(_) => {
                CONTROL_FLOW.with(|stack| stack.borrow_mut().replace(ControlFlow::Break));
                return Ok(None);
            }
            Stmt::Continue(_) => {
                CONTROL_FLOW.with(|stack| stack.borrow_mut().replace(ControlFlow::Continue));
                return Ok(None);
            }
            Stmt::Assign(name, expr, _) => {
                let val = eval_expr(expr, env, functions)?;
                env.insert(name.clone(), val);
            }
            Stmt::ExprFieldAssign(_, _, _, _) => {}
            Stmt::DerefAssign(_, _, _) => {
                return Err(
                    "reference assignment is available only in the qualified MIR/native path"
                        .to_string(),
                );
            }
            Stmt::WhileIn {
                var_name,
                iterable,
                body,
                ..
            } => {
                let iter_val = eval_expr(iterable, env, functions)?;
                match iter_val {
                    Value::Int(n) => {
                        for i in 0..n {
                            env.insert(var_name.clone(), Value::Int(i));
                            if let Some(val) = eval_block(body, env, functions)? {
                                return Ok(Some(val));
                            }
                            if let Some(cf) = CONTROL_FLOW.with(|stack| stack.borrow_mut().take()) {
                                match cf {
                                    ControlFlow::Break => break,
                                    ControlFlow::Continue => continue,
                                }
                            }
                        }
                    }
                    _ => return Err("WhileIn requires int iterable".to_string()),
                }
            }
            Stmt::Unsafe { body, .. } => {
                if let Some(val) = eval_block(body, env, functions)? {
                    return Ok(Some(val));
                }
            }
            Stmt::LetLinear(name, _type_ann, expr, _) => {
                let val = eval_expr(expr, env, functions)?;
                env.insert(name.clone(), val);
            }
            Stmt::Struct { .. } => {}
            Stmt::Enum { .. } => {}
            Stmt::ErrorSet { .. } => {}
            Stmt::Impl { .. } => {}
            Stmt::Trait { .. } => {}
            Stmt::TypeAlias { .. } => {}
            Stmt::Use { .. } => {}
            Stmt::GcMode { .. } => {}
            Stmt::CancelToken { .. } => {}
            Stmt::EffectHandler {
                effect, handler, ..
            } => {
                // Evaluate the handler expression and register it in the handler stack
                let handler_val = eval_expr(handler, env, functions)?;
                let mut handler_layer = HashMap::new();
                handler_layer.insert(effect.clone(), handler_val);
                HANDLER_STACK.with(|stack| {
                    stack.borrow_mut().push(handler_layer);
                });
            }
            Stmt::Spawn { task, .. } => {
                let _ = eval_expr(task, env, functions)?;
            }
            Stmt::Channel {
                elem_type,
                capacity,
                ..
            } => {
                let raw_capacity = capacity.unwrap_or(16);
                let capacity = usize::try_from(raw_capacity)
                    .map_err(|_| "channel capacity is out of range for this target".to_string())?;
                env.insert(
                    format!("__chan_{}", elem_type),
                    Value::Channel(crate::effect_system::Channel::new(capacity)),
                );
            }
            Stmt::Actor { .. } => {}
            Stmt::WorkStealingExecutor { .. } => {}
            Stmt::DeterministicRuntime { .. } => {}
            Stmt::Tensor { .. } => {}
            Stmt::Simd { .. } => {}
            Stmt::DocComment { .. } => {}
            Stmt::DebugSession { .. } => {}
            Stmt::Capability { .. } => {}
            Stmt::FfiSandbox { .. } => {}
            Stmt::Fn { .. } => {}
            Stmt::UseScoped { .. } => {}
            Stmt::ContractRequires { .. } => {}
            Stmt::ContractEnsures { .. } => {}
            Stmt::ContractInvariant { .. } => {}
            Stmt::ComptimeLimit { .. } => {}
            Stmt::Defer { cleanup, .. } | Stmt::AsyncDefer { cleanup, .. } => {
                DEFER_STACK.with(|stack| {
                    if let Some(current) = stack.borrow_mut().last_mut() {
                        current.push((**cleanup).clone());
                    }
                });
            }
            Stmt::Mod(_, _) | Stmt::ModBlock(_, _, _) => {}
        }
    }
    Ok(None)
}

pub fn run_program(program: &Program) -> Result<(), String> {
    let mut env: HashMap<String, Value> = HashMap::new();
    let mut functions: HashMap<String, &Stmt> = HashMap::new();
    for stmt in &program.stmts {
        if let Stmt::Fn { name, .. } = stmt {
            functions.insert(name.clone(), stmt);
        }
    }
    let _ = eval_block(&program.stmts, &mut env, &functions)?;
    Ok(())
}
