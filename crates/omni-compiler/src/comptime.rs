use crate::ast::{Expr, Program, Stmt};
use crate::lexer::TokenKind;
use crate::type_checker::Type;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ComptimeValue {
    Int(i64),
    String(String),
    Bool(bool),
    Unit,
    Tuple(Vec<ComptimeValue>),
    Struct(String, HashMap<String, ComptimeValue>),
}

impl ComptimeValue {
    pub fn type_of(&self) -> Type {
        match self {
            ComptimeValue::Int(_) => Type::Int,
            ComptimeValue::String(_) => Type::String,
            ComptimeValue::Bool(_) => Type::Bool,
            ComptimeValue::Unit => Type::Unit,
            ComptimeValue::Tuple(vals) => Type::Struct {
                name: "Tuple".to_string(),
                fields: vals.iter().map(|v| v.type_of()).collect(),
                is_linear: false,
            },
            ComptimeValue::Struct(name, fields) => Type::Struct {
                name: name.clone(),
                fields: fields.values().map(|v| v.type_of()).collect(),
                is_linear: false,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum ComptimeError {
    TypeError(String),
    UndefinedVariable(String),
    DivisionByZero,
    InvalidOperation(String),
    RecursionLimit,
}

pub struct ComptimeContext {
    pub variables: HashMap<String, ComptimeValue>,
    pub functions: HashMap<String, ComptimeFunction>,
    pub recursion_limit: usize,
    pub current_depth: usize,
}

#[derive(Debug, Clone)]
pub struct ComptimeFunction {
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
}

fn match_pattern(
    pattern: &crate::ast::Pattern,
    value: &ComptimeValue,
) -> Option<HashMap<String, ComptimeValue>> {
    match pattern {
        crate::ast::Pattern::Wildcard => Some(HashMap::new()),
        crate::ast::Pattern::Literal(expected) => match value {
            ComptimeValue::Int(actual) if actual == expected => Some(HashMap::new()),
            _ => None,
        },
        crate::ast::Pattern::Var(name) => {
            let mut bindings = HashMap::new();
            bindings.insert(name.clone(), value.clone());
            Some(bindings)
        }
        crate::ast::Pattern::Struct(_name, fields) => {
            if let ComptimeValue::Struct(_, values) = value {
                let mut bindings = HashMap::new();
                for (field_name, field_pattern) in fields {
                    let field_value = values.get(field_name)?;
                    let nested = match_pattern(field_pattern, field_value)?;
                    for (bind_name, bind_value) in nested {
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

impl ComptimeContext {
    pub fn new() -> Self {
        ComptimeContext {
            variables: HashMap::new(),
            functions: HashMap::new(),
            recursion_limit: 1000,
            current_depth: 0,
        }
    }

    pub fn eval_program(&mut self, prog: &Program) -> Result<ComptimeValue, ComptimeError> {
        let mut last_value = ComptimeValue::Unit;

        for stmt in &prog.stmts {
            last_value = self.eval_stmt(stmt)?;
        }

        Ok(last_value)
    }

    pub fn eval_stmt(&mut self, stmt: &Stmt) -> Result<ComptimeValue, ComptimeError> {
        match stmt {
            Stmt::Let(name, expr) => {
                let value = self.eval_expr(expr)?;
                self.variables.insert(name.clone(), value);
                Ok(ComptimeValue::Unit)
            }
            Stmt::ExprStmt(expr) => self.eval_expr(expr),
            Stmt::Print(expr) => {
                let value = self.eval_expr(expr)?;
                println!("{:?}", value);
                Ok(ComptimeValue::Unit)
            }
            Stmt::Return(expr) => self.eval_expr(expr),
            Stmt::Fn {
                name,
                is_public: _,
                params,
                body,
                ..
            } => {
                self.functions.insert(
                    name.clone(),
                    ComptimeFunction {
                        params: params.clone(),
                        body: body.clone(),
                    },
                );
                Ok(ComptimeValue::Unit)
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
                ..
            } => {
                let cond_value = self.eval_expr(cond)?;
                if self.is_truthy(&cond_value) {
                    for s in then_body {
                        self.eval_stmt(s)?;
                    }
                } else {
                    for s in else_body {
                        self.eval_stmt(s)?;
                    }
                }
                Ok(ComptimeValue::Unit)
            }
            Stmt::Loop { .. } => {
                // Note: Simplified - would need proper break handling in production
                Ok(ComptimeValue::Unit)
            }
            Stmt::While { cond, body } => {
                while {
                    let cond_val = self.eval_expr(cond)?;
                    self.is_truthy(&cond_val)
                } {
                    for s in body {
                        self.eval_stmt(s)?;
                    }
                }
                Ok(ComptimeValue::Unit)
            }
            Stmt::For {
                var_name,
                iterable,
                body,
            } => {
                let iter_value = self.eval_expr(iterable)?;
                if let ComptimeValue::Tuple(vals) = iter_value {
                    for val in vals {
                        self.variables.insert(var_name.clone(), val);
                        for s in body {
                            self.eval_stmt(s)?;
                        }
                    }
                }
                Ok(ComptimeValue::Unit)
            }
            Stmt::Break => Ok(ComptimeValue::Unit), // Simplified
            Stmt::Continue => Ok(ComptimeValue::Unit), // Simplified
            _ => Ok(ComptimeValue::Unit),
        }
    }

    pub fn eval_expr(&mut self, expr: &Expr) -> Result<ComptimeValue, ComptimeError> {
        match expr {
            Expr::Number(n) => Ok(ComptimeValue::Int(*n)),
            Expr::StringLit(s) => Ok(ComptimeValue::String(s.clone())),
            Expr::Bool(b) => Ok(ComptimeValue::Bool(*b)),
            Expr::Var(name) => self
                .variables
                .get(name)
                .cloned()
                .ok_or_else(|| ComptimeError::UndefinedVariable(name.clone())),
            Expr::Call(name, args) => self.eval_call(name, args),
            Expr::BinaryOp { op, left, right } => self.eval_binary_op(op, left, right),
            Expr::UnaryOp { op, inner } => {
                let value = self.eval_expr(inner)?;
                self.eval_unary_op(op, &value)
            }
            Expr::FieldAccess { base, field } => {
                let base_value = self.eval_expr(base)?;
                match base_value {
                    ComptimeValue::String(s) if field == "len" => {
                        Ok(ComptimeValue::Int(s.chars().count() as i64))
                    }
                    ComptimeValue::Tuple(values) if field == "len" => {
                        Ok(ComptimeValue::Int(values.len() as i64))
                    }
                    ComptimeValue::Struct(_, fields) => {
                        fields.get(field).cloned().ok_or_else(|| {
                            ComptimeError::InvalidOperation(format!("unknown field {}", field))
                        })
                    }
                    other => Err(ComptimeError::InvalidOperation(format!(
                        "field access {:?}.{}",
                        other, field
                    ))),
                }
            }
            Expr::Tuple(exprs) => {
                let mut values = Vec::new();
                for e in exprs {
                    values.push(self.eval_expr(e)?);
                }
                Ok(ComptimeValue::Tuple(values))
            }
            Expr::Match { expr, arms } => {
                let scrutinee = self.eval_expr(expr)?;

                for arm in arms {
                    let Some(bindings) = match_pattern(&arm.pattern, &scrutinee) else {
                        continue;
                    };

                    let mut local_ctx = ComptimeContext {
                        variables: self.variables.clone(),
                        functions: self.functions.clone(),
                        recursion_limit: self.recursion_limit,
                        current_depth: self.current_depth,
                    };
                    for (name, value) in bindings {
                        local_ctx.variables.insert(name, value);
                    }

                    if let Some(guard) = &arm.guard {
                        let guard_value = local_ctx.eval_expr(guard)?;
                        if !local_ctx.is_truthy(&guard_value) {
                            continue;
                        }
                    }

                    return local_ctx.eval_expr(&arm.body);
                }

                Err(ComptimeError::InvalidOperation(
                    "non-exhaustive match expression".to_string(),
                ))
            }
            Expr::Block(stmts) => {
                let mut last = ComptimeValue::Unit;
                for s in stmts {
                    last = self.eval_stmt(s)?;
                }
                Ok(last)
            }
            _ => Ok(ComptimeValue::Unit),
        }
    }

    fn eval_call(&mut self, name: &str, args: &[Expr]) -> Result<ComptimeValue, ComptimeError> {
        // Check if it's a builtin
        match name {
            "+" | "add" => {
                let a = self.eval_expr(&args[0])?;
                let b = self.eval_expr(&args[1])?;
                if let (ComptimeValue::Int(ia), ComptimeValue::Int(ib)) = (a, b) {
                    return Ok(ComptimeValue::Int(ia + ib));
                }
            }
            "-" | "sub" => {
                let a = self.eval_expr(&args[0])?;
                let b = self.eval_expr(&args[1])?;
                if let (ComptimeValue::Int(ia), ComptimeValue::Int(ib)) = (a, b) {
                    return Ok(ComptimeValue::Int(ia - ib));
                }
            }
            "*" | "mul" => {
                let a = self.eval_expr(&args[0])?;
                let b = self.eval_expr(&args[1])?;
                if let (ComptimeValue::Int(ia), ComptimeValue::Int(ib)) = (a, b) {
                    return Ok(ComptimeValue::Int(ia * ib));
                }
            }
            "/" | "div" => {
                let a = self.eval_expr(&args[0])?;
                let b = self.eval_expr(&args[1])?;
                if let (ComptimeValue::Int(ia), ComptimeValue::Int(ib)) = (a, b) {
                    if ib == 0 {
                        return Err(ComptimeError::DivisionByZero);
                    }
                    return Ok(ComptimeValue::Int(ia / ib));
                }
            }
            "%" | "mod" => {
                let a = self.eval_expr(&args[0])?;
                let b = self.eval_expr(&args[1])?;
                if let (ComptimeValue::Int(ia), ComptimeValue::Int(ib)) = (a, b) {
                    return Ok(ComptimeValue::Int(ia % ib));
                }
            }
            _ => {}
        }

        // User-defined function
        if let Some(func) = self.functions.get(name).cloned() {
            // Evaluate arguments
            let mut args_values = Vec::new();
            for arg in args {
                args_values.push(self.eval_expr(arg)?);
            }

            // Create local scope
            let mut local_ctx = ComptimeContext {
                variables: self.variables.clone(),
                functions: self.functions.clone(),
                recursion_limit: self.recursion_limit,
                current_depth: self.current_depth + 1,
            };

            if local_ctx.current_depth > local_ctx.recursion_limit {
                return Err(ComptimeError::RecursionLimit);
            }

            // Bind parameters
            for (i, param) in func.params.iter().enumerate() {
                if i < args_values.len() {
                    local_ctx
                        .variables
                        .insert(param.clone(), args_values[i].clone());
                }
            }

            // Evaluate body
            let mut last = ComptimeValue::Unit;
            for stmt in &func.body {
                last = local_ctx.eval_stmt(stmt)?;
            }

            return Ok(last);
        }

        Err(ComptimeError::UndefinedVariable(format!(
            "function '{}'",
            name
        )))
    }

    fn eval_binary_op(
        &mut self,
        op: &TokenKind,
        left: &Expr,
        right: &Expr,
    ) -> Result<ComptimeValue, ComptimeError> {
        let left_val = self.eval_expr(left)?;
        let right_val = self.eval_expr(right)?;

        match op {
            TokenKind::Plus => {
                if let ComptimeValue::Int(l) = &left_val {
                    if let ComptimeValue::Int(r) = &right_val {
                        return Ok(ComptimeValue::Int(*l + *r));
                    }
                }
                if let ComptimeValue::String(l) = &left_val {
                    if let ComptimeValue::String(r) = &right_val {
                        return Ok(ComptimeValue::String(l.clone() + r));
                    }
                }
            }
            TokenKind::Minus => {
                if let ComptimeValue::Int(l) = &left_val {
                    if let ComptimeValue::Int(r) = &right_val {
                        return Ok(ComptimeValue::Int(*l - *r));
                    }
                }
            }
            TokenKind::Star => {
                if let ComptimeValue::Int(l) = &left_val {
                    if let ComptimeValue::Int(r) = &right_val {
                        return Ok(ComptimeValue::Int(*l * *r));
                    }
                }
            }
            TokenKind::Slash => {
                if let ComptimeValue::Int(l) = &left_val {
                    if let ComptimeValue::Int(r) = &right_val {
                        if *r == 0 {
                            return Err(ComptimeError::DivisionByZero);
                        }
                        return Ok(ComptimeValue::Int(*l / *r));
                    }
                }
                if let (ComptimeValue::String(l), ComptimeValue::String(r)) = (left_val, right_val)
                {
                    return Ok(ComptimeValue::String(l + &r));
                }
            }
            TokenKind::Percent => {
                if let ComptimeValue::Int(l) = &left_val {
                    if let ComptimeValue::Int(r) = &right_val {
                        return Ok(ComptimeValue::Int(*l % *r));
                    }
                }
            }
            TokenKind::EqEq => {
                return Ok(ComptimeValue::Bool(left_val == right_val));
            }
            TokenKind::NotEq => {
                return Ok(ComptimeValue::Bool(left_val != right_val));
            }
            TokenKind::Lt => {
                if let ComptimeValue::Int(l) = &left_val {
                    if let ComptimeValue::Int(r) = &right_val {
                        return Ok(ComptimeValue::Bool(*l < *r));
                    }
                }
            }
            TokenKind::LtEq => {
                if let ComptimeValue::Int(l) = &left_val {
                    if let ComptimeValue::Int(r) = &right_val {
                        return Ok(ComptimeValue::Bool(*l <= *r));
                    }
                }
            }
            TokenKind::Gt => {
                if let ComptimeValue::Int(l) = &left_val {
                    if let ComptimeValue::Int(r) = &right_val {
                        return Ok(ComptimeValue::Bool(*l > *r));
                    }
                }
            }
            TokenKind::GtEq => {
                if let ComptimeValue::Int(l) = &left_val {
                    if let ComptimeValue::Int(r) = &right_val {
                        return Ok(ComptimeValue::Bool(*l >= *r));
                    }
                }
            }
            TokenKind::AndAnd => {
                if let ComptimeValue::Bool(l) = &left_val {
                    if let ComptimeValue::Bool(r) = &right_val {
                        return Ok(ComptimeValue::Bool(*l && *r));
                    }
                }
            }
            TokenKind::OrOr => {
                if let ComptimeValue::Bool(l) = &left_val {
                    if let ComptimeValue::Bool(r) = &right_val {
                        return Ok(ComptimeValue::Bool(*l || *r));
                    }
                }
            }
            _ => {}
        }

        Err(ComptimeError::InvalidOperation(format!("{:?}", op)))
    }

    fn eval_unary_op(
        &self,
        op: &TokenKind,
        value: &ComptimeValue,
    ) -> Result<ComptimeValue, ComptimeError> {
        match op {
            TokenKind::Minus => {
                if let ComptimeValue::Int(n) = value {
                    return Ok(ComptimeValue::Int(-*n));
                }
            }
            TokenKind::Bang => {
                if let ComptimeValue::Bool(b) = value {
                    return Ok(ComptimeValue::Bool(!*b));
                }
            }
            _ => {}
        }
        Err(ComptimeError::InvalidOperation(format!("unary {:?}", op)))
    }

    fn is_truthy(&self, value: &ComptimeValue) -> bool {
        match value {
            ComptimeValue::Bool(b) => *b,
            ComptimeValue::Int(n) => *n != 0,
            ComptimeValue::String(s) => !s.is_empty(),
            ComptimeValue::Unit => false,
            ComptimeValue::Tuple(vals) => !vals.is_empty(),
            ComptimeValue::Struct(_, fields) => !fields.is_empty(),
        }
    }
}

impl Default for ComptimeContext {
    fn default() -> Self {
        Self::new()
    }
}

pub fn eval_comptime(expr: &Expr) -> Result<ComptimeValue, ComptimeError> {
    let mut ctx = ComptimeContext::new();
    ctx.eval_expr(expr)
}

pub fn is_comptime_known(expr: &Expr) -> bool {
    eval_comptime(expr).is_ok()
}
