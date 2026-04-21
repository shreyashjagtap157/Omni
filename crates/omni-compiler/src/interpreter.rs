use crate::ast::{Expr, InterpolatedFragment, Program, Stmt};
use crate::lexer::TokenKind;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Str(String),
    Bool(bool),
    Vector(Vec<Value>),
    Map(HashMap<String, Value>),
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Int(n) => *n != 0,
        Value::Bool(b) => *b,
        Value::Str(s) => !s.is_empty(),
        Value::Vector(vv) => !vv.is_empty(),
        Value::Map(m) => !m.is_empty(),
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
        Value::Int(n) => n.to_string(),
        Value::Str(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Vector(vec) => format!("<vector len={}>", vec.len()),
        Value::Map(map) => format!("<map len={}>", map.len()),
    }
}

fn eval_expr(
    expr: &Expr,
    env: &mut HashMap<String, Value>,
    functions: &HashMap<String, &Stmt>,
) -> Result<Value, String> {
    match expr {
        Expr::StringLit(s) => Ok(Value::Str(s.clone())),
        Expr::Number(n) => Ok(Value::Int(*n)),
        Expr::Var(name) => env
            .get(name)
            .cloned()
            .ok_or(format!("Undefined var {}", name)),
        Expr::Bool(b) => Ok(Value::Bool(*b)),
        Expr::Call(name, args) => {
            let mut evaled_args: Vec<Value> = Vec::new();
            let mut arg_var_names: Vec<Option<String>> = Vec::new();
            for a in args {
                match a {
                    Expr::Var(vname) => {
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
                                let out: String = s
                                    .chars()
                                    .skip(*start as usize)
                                    .take(*len as usize)
                                    .collect();
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
                            Value::Str(s) => match s.parse::<i64>() {
                                Ok(n) => Ok(Value::Int(n)),
                                Err(_) => Ok(Value::Int(0)),
                            },
                            _ => Err("string_to_int requires string".to_string()),
                        }
                    } else {
                        Err("string_to_int requires one argument".to_string())
                    }
                }
                "int_abs" => {
                    if evaled_args.len() == 1 {
                        match &evaled_args[0] {
                            Value::Int(n) => Ok(Value::Int(n.abs())),
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
                                if *b < 0 {
                                    Ok(Value::Int(0))
                                } else {
                                    Ok(Value::Int(a.pow(*b as u32)))
                                }
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
                                if *b == 0 {
                                    Err("divide by zero".to_string())
                                } else {
                                    Ok(Value::Int(a / b))
                                }
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
                            Value::Map(_map) => Ok(Value::Int(0)),
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
                                                    local_env.insert(params[0].clone(), v.clone());
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
                                                        .unwrap_or(Value::Int(0));
                                                    let mut local_env = env.clone();
                                                    local_env.insert(params[0].clone(), err_val);
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
                                                    local_env.insert(params[0].clone(), v.clone());
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
                                let idx = if *i < 0 { 0usize } else { *i as usize };
                                if idx < vec.len() {
                                    Ok(vec[idx].clone())
                                } else {
                                    Ok(Value::Int(0))
                                }
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
                                let idx = if *i < 0 { 0usize } else { *i as usize };
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
                                if new_vec.is_empty() {
                                    Err("vector_pop: empty vector".to_string())
                                } else {
                                    let val = new_vec.pop().unwrap();
                                    if let Some(Some(varname)) = arg_var_names.first() {
                                        env.insert(varname.clone(), Value::Vector(new_vec));
                                    }
                                    Ok(val)
                                }
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
                                let idx = if *i < 0 { 0usize } else { *i as usize };
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
                                let idx = if *i < 0 { 0usize } else { *i as usize };
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
                                Ok(Value::Int(0))
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
                            Value::Vector(vec) => Ok(Value::Int(vec.len() as i64)),
                            _ => Ok(Value::Int(0)),
                        }
                    } else {
                        Err("vector_capacity requires one argument".to_string())
                    }
                }
                "vector_reserve" => {
                    if evaled_args.len() == 2 {
                        Ok(Value::Int(0))
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
                            (Value::Map(map), Value::Str(k)) => {
                                Ok(map.get(k).cloned().unwrap_or(Value::Int(0)))
                            }
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
                            _ => Ok(Value::Int(0)),
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
                        Ok(Value::Int(0))
                    } else {
                        Err("hashmap_clear requires one argument".to_string())
                    }
                }
                _ => {
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
                                local_env.insert(p.clone(), a.clone());
                            }
                            if let Some(val) = eval_block(body, &mut local_env, functions)? {
                                Ok(val)
                            } else {
                                Ok(Value::Int(0))
                            }
                        }
                    } else {
                        Err(format!("Undefined function: {}", name))
                    }
                }
            }
        }
        Expr::Interpolated(frags) => {
            let mut out = String::new();
            for frag in frags.iter() {
                match frag {
                    InterpolatedFragment::Literal(s) => out.push_str(s),
                    InterpolatedFragment::Expr(e) => {
                        let v = eval_expr(e, env, functions)?;
                        out.push_str(&value_to_string(&v));
                    }
                }
            }
            Ok(Value::Str(out))
        }
        Expr::BinaryOp { op, left, right } => {
            let l = eval_expr(left, env, functions)?;
            let r = eval_expr(right, env, functions)?;
            match (l, r) {
                (Value::Int(a), Value::Int(b)) => {
                    let res_int = match op {
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
        Expr::UnaryOp { op, inner } => {
            let v = eval_expr(inner, env, functions)?;
            match op {
                TokenKind::Minus => match v {
                    Value::Int(n) => Ok(Value::Int(-n)),
                    _ => Err("Unary - requires int".to_string()),
                },
                TokenKind::Bang => match v {
                    Value::Bool(b) => Ok(Value::Bool(!b)),
                    _ => Err("Unary ! requires bool".to_string()),
                },
                _ => Err("Unsupported unary op".to_string()),
            }
        }
        Expr::FieldAccess { base, field } => {
            let base_value = eval_expr(base, env, functions)?;
            match base_value {
                Value::Str(s) if field == "len" => Ok(Value::Int(s.chars().count() as i64)),
                Value::Vector(vec) if field == "len" => Ok(Value::Int(vec.len() as i64)),
                Value::Map(map) => map
                    .get(field)
                    .cloned()
                    .ok_or_else(|| format!("Unknown field '{}'", field)),
                other => Err(format!("FieldAccess not implemented for {:?}", other)),
            }
        }
        Expr::IfExpr { cond, then, else_ } => {
            let c = eval_expr(cond, env, functions)?;
            if truthy(&c) {
                eval_expr(then, env, functions)
            } else {
                eval_expr(else_, env, functions)
            }
        }
        Expr::Block(stmts) => {
            let mut local = env.clone();
            eval_block(stmts, &mut local, functions)?;
            Ok(Value::Int(0))
        }
        Expr::Tuple(exprs) => {
            let mut values = Vec::new();
            for expr in exprs {
                values.push(eval_expr(expr, env, functions)?);
            }
            Ok(Value::Vector(values))
        }
        Expr::Index(base, index) => {
            let base_value = eval_expr(base, env, functions)?;
            let index_value = eval_expr(index, env, functions)?;
            match (base_value, index_value) {
                (Value::Vector(values), Value::Int(idx)) => {
                    let position = if idx < 0 { 0usize } else { idx as usize };
                    values
                        .get(position)
                        .cloned()
                        .ok_or_else(|| "Index out of bounds".to_string())
                }
                (Value::Str(text), Value::Int(idx)) => {
                    let position = if idx < 0 { 0usize } else { idx as usize };
                    text.chars()
                        .nth(position)
                        .map(|ch| Value::Str(ch.to_string()))
                        .ok_or_else(|| "Index out of bounds".to_string())
                }
                _ => Err("Index evaluation not implemented for these operands".to_string()),
            }
        }
        Expr::Match { expr, arms } => {
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
    }
}

fn eval_block(
    stmts: &[Stmt],
    env: &mut HashMap<String, Value>,
    functions: &HashMap<String, &Stmt>,
) -> Result<Option<Value>, String> {
    for stmt in stmts {
        match stmt {
            Stmt::Let(name, expr) => {
                let val = eval_expr(expr, env, functions)?;
                env.insert(name.clone(), val);
            }
            Stmt::Print(expr) => {
                let val = eval_expr(expr, env, functions)?;
                match val {
                    Value::Int(n) => println!("{}", n),
                    Value::Str(s) => println!("{}", s),
                    Value::Bool(b) => println!("{}", b),
                    Value::Vector(v) => println!("<vector len={}>", v.len()),
                    Value::Map(m) => println!("<map len={}>", m.len()),
                }
            }
            Stmt::ExprStmt(expr) => {
                let _ = eval_expr(expr, env, functions)?;
            }
            Stmt::Block(inner) => {
                if let Some(val) = eval_block(inner, env, functions)? {
                    return Ok(Some(val));
                }
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
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
            Stmt::Loop { body } => loop {
                if let Some(val) = eval_block(body, env, functions)? {
                    return Ok(Some(val));
                }
            },
            Stmt::For {
                var_name,
                iterable,
                body,
            } => {
                let iter_val = eval_expr(iterable, env, functions)?;
                match iter_val {
                    Value::Int(n) => {
                        for i in 0..n {
                            env.insert(var_name.clone(), Value::Int(i));
                            if let Some(val) = eval_block(body, env, functions)? {
                                return Ok(Some(val));
                            }
                        }
                    }
                    Value::Str(s) => {
                        for c in s.chars() {
                            env.insert(var_name.clone(), Value::Str(c.to_string()));
                            if let Some(val) = eval_block(body, env, functions)? {
                                return Ok(Some(val));
                            }
                        }
                    }
                    Value::Vector(v) => {
                        for elem in v {
                            env.insert(var_name.clone(), elem);
                            if let Some(val) = eval_block(body, env, functions)? {
                                return Ok(Some(val));
                            }
                        }
                    }
                    Value::Map(m) => {
                        for key in m.keys() {
                            env.insert(var_name.clone(), Value::Str(key.clone()));
                            if let Some(val) = eval_block(body, env, functions)? {
                                return Ok(Some(val));
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
            Stmt::While { cond, body } => {
                while truthy(&eval_expr(cond, env, functions)?) {
                    if let Some(val) = eval_block(body, env, functions)? {
                        return Ok(Some(val));
                    }
                }
            }
            Stmt::Return(expr) => {
                let val = eval_expr(expr, env, functions)?;
                return Ok(Some(val));
            }
            Stmt::Break => {}
            Stmt::Continue => {}
            Stmt::Assign(name, expr) => {
                let val = eval_expr(expr, env, functions)?;
                env.insert(name.clone(), val);
            }
            Stmt::ExprFieldAssign(_, _, _) => {}
            Stmt::WhileIn {
                var_name,
                iterable,
                body,
            } => {
                let iter_val = eval_expr(iterable, env, functions)?;
                match iter_val {
                    Value::Int(n) => {
                        for i in 0..n {
                            env.insert(var_name.clone(), Value::Int(i));
                            if let Some(val) = eval_block(body, env, functions)? {
                                return Ok(Some(val));
                            }
                        }
                    }
                    _ => return Err("WhileIn requires int iterable".to_string()),
                }
            }
            Stmt::Unsafe { body } => {
                if let Some(val) = eval_block(body, env, functions)? {
                    return Ok(Some(val));
                }
            }
            Stmt::LetLinear(name, expr) => {
                let val = eval_expr(expr, env, functions)?;
                env.insert(name.clone(), val);
            }
            Stmt::Struct { .. } => {}
            Stmt::Enum { .. } => {}
            Stmt::Fn { .. } => {}
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
