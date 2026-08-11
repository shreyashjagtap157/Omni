use crate::ast::{Expr, Program, Stmt};
use crate::complete_lexer::TokenKind;
use crate::types::Type;
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

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
    Unsupported(String),
    Arity {
        name: String,
        expected: usize,
        actual: usize,
    },
    ArithmeticOverflow(&'static str),
    OperationLimit,
    RecursionLimit,
}

pub struct ComptimeContext {
    pub variables: HashMap<String, ComptimeValue>,
    pub functions: HashMap<String, ComptimeFunction>,
    pub recursion_limit: usize,
    pub current_depth: usize,
    pub max_operations: usize,
    operations: Rc<Cell<usize>>,
}

#[derive(Debug, Clone)]
pub struct ComptimeFunction {
    pub params: Vec<(String, Option<String>)>,
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
            max_operations: 100_000,
            operations: Rc::new(Cell::new(0)),
        }
    }

    fn tick(&self) -> Result<(), ComptimeError> {
        let next = self.operations.get().saturating_add(1);
        self.operations.set(next);
        if next > self.max_operations {
            Err(ComptimeError::OperationLimit)
        } else {
            Ok(())
        }
    }

    pub fn operations_used(&self) -> usize {
        self.operations.get()
    }

    pub fn eval_program(&mut self, prog: &Program) -> Result<ComptimeValue, ComptimeError> {
        let mut last_value = ComptimeValue::Unit;

        for stmt in &prog.stmts {
            last_value = self.eval_stmt(stmt)?;
        }

        Ok(last_value)
    }

    pub fn eval_stmt(&mut self, stmt: &Stmt) -> Result<ComptimeValue, ComptimeError> {
        self.tick()?;
        match stmt {
            Stmt::Let(name, _type_ann, expr, _) => {
                let value = self.eval_expr(expr)?;
                self.variables.insert(name.clone(), value);
                Ok(ComptimeValue::Unit)
            }
            Stmt::ExprStmt(expr, _) => self.eval_expr(expr),
            Stmt::Print(expr, _) => {
                let value = self.eval_expr(expr)?;
                println!("{:?}", value);
                Ok(ComptimeValue::Unit)
            }
            Stmt::Return(expr, _) => self.eval_expr(expr),
            Stmt::Fn {
                name,
                visibility: _,
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
            Stmt::Loop { .. } => Err(ComptimeError::Unsupported(
                "unbounded comptime loop; use a bounded while/for form until loop control is qualified".to_string(),
            )),
            Stmt::While { cond, body, .. } => {
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
                ..
            } => {
                let iter_value = self.eval_expr(iterable)?;
                if let ComptimeValue::Tuple(vals) = iter_value {
                    for val in vals {
                        self.variables.insert(var_name.clone(), val);
                        for s in body {
                            self.eval_stmt(s)?;
                        }
                    }
                    Ok(ComptimeValue::Unit)
                } else {
                    Err(ComptimeError::TypeError(
                        "comptime for currently requires a tuple iterable".to_string(),
                    ))
                }
            }
            Stmt::Break(_) => Err(ComptimeError::Unsupported(
                "comptime break is not yet supported".to_string(),
            )),
            Stmt::Continue(_) => Err(ComptimeError::Unsupported(
                "comptime continue is not yet supported".to_string(),
            )),
            other => Err(ComptimeError::Unsupported(format!(
                "statement {:?} is not supported by the compile-time evaluator",
                other
            ))),
        }
    }

    pub fn eval_expr(&mut self, expr: &Expr) -> Result<ComptimeValue, ComptimeError> {
        self.tick()?;
        match expr {
            Expr::Number(n, _) => Ok(ComptimeValue::Int(*n)),
            Expr::StringLit(s, _) => Ok(ComptimeValue::String(s.clone())),
            Expr::Bool(b, _) => Ok(ComptimeValue::Bool(*b)),
            Expr::Var(name, _) => self
                .variables
                .get(name)
                .cloned()
                .ok_or_else(|| ComptimeError::UndefinedVariable(name.clone())),
            Expr::Call(name, args, _) => self.eval_call(name, args),
            Expr::BinaryOp {
                op, left, right, ..
            } => self.eval_binary_op(op, left, right),
            Expr::UnaryOp { op, inner, .. } => {
                let value = self.eval_expr(inner)?;
                self.eval_unary_op(op, &value)
            }
            Expr::FieldAccess { base, field, .. } => {
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
            Expr::Tuple(exprs, _) => {
                let mut values = Vec::new();
                for e in exprs {
                    values.push(self.eval_expr(e)?);
                }
                Ok(ComptimeValue::Tuple(values))
            }
            Expr::Match { expr, arms, .. } => {
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
                        max_operations: self.max_operations,
                        operations: Rc::clone(&self.operations),
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
            Expr::Block(stmts, _) => {
                let mut last = ComptimeValue::Unit;
                for s in stmts {
                    last = self.eval_stmt(s)?;
                }
                Ok(last)
            }
            other => Err(ComptimeError::Unsupported(format!(
                "expression {:?} is not supported by the compile-time evaluator",
                other
            ))),
        }
    }

    fn eval_call(&mut self, name: &str, args: &[Expr]) -> Result<ComptimeValue, ComptimeError> {
        let require_arity = |expected: usize| -> Result<(), ComptimeError> {
            if args.len() == expected {
                Ok(())
            } else {
                Err(ComptimeError::Arity {
                    name: name.to_string(),
                    expected,
                    actual: args.len(),
                })
            }
        };

        // Check if it's a builtin. Arithmetic uses the same checked i64
        // semantics as the Stage-0 runtime.
        match name {
            "+" | "add" => {
                require_arity(2)?;
                let a = self.eval_expr(&args[0])?;
                let b = self.eval_expr(&args[1])?;
                if let (ComptimeValue::Int(ia), ComptimeValue::Int(ib)) = (a, b) {
                    return ia
                        .checked_add(ib)
                        .map(ComptimeValue::Int)
                        .ok_or(ComptimeError::ArithmeticOverflow("add"));
                }
            }
            "-" | "sub" => {
                require_arity(2)?;
                let a = self.eval_expr(&args[0])?;
                let b = self.eval_expr(&args[1])?;
                if let (ComptimeValue::Int(ia), ComptimeValue::Int(ib)) = (a, b) {
                    return ia
                        .checked_sub(ib)
                        .map(ComptimeValue::Int)
                        .ok_or(ComptimeError::ArithmeticOverflow("sub"));
                }
            }
            "*" | "mul" => {
                require_arity(2)?;
                let a = self.eval_expr(&args[0])?;
                let b = self.eval_expr(&args[1])?;
                if let (ComptimeValue::Int(ia), ComptimeValue::Int(ib)) = (a, b) {
                    return ia
                        .checked_mul(ib)
                        .map(ComptimeValue::Int)
                        .ok_or(ComptimeError::ArithmeticOverflow("mul"));
                }
            }
            "/" | "div" => {
                require_arity(2)?;
                let a = self.eval_expr(&args[0])?;
                let b = self.eval_expr(&args[1])?;
                if let (ComptimeValue::Int(ia), ComptimeValue::Int(ib)) = (a, b) {
                    if ib == 0 {
                        return Err(ComptimeError::DivisionByZero);
                    }
                    return ia
                        .checked_div(ib)
                        .map(ComptimeValue::Int)
                        .ok_or(ComptimeError::ArithmeticOverflow("div"));
                }
            }
            "%" | "mod" => {
                require_arity(2)?;
                let a = self.eval_expr(&args[0])?;
                let b = self.eval_expr(&args[1])?;
                if let (ComptimeValue::Int(ia), ComptimeValue::Int(ib)) = (a, b) {
                    if ib == 0 {
                        return Err(ComptimeError::DivisionByZero);
                    }
                    return ia
                        .checked_rem(ib)
                        .map(ComptimeValue::Int)
                        .ok_or(ComptimeError::ArithmeticOverflow("mod"));
                }
            }
            _ => {}
        }

        // User-defined function
        if let Some(func) = self.functions.get(name).cloned() {
            if args.len() != func.params.len() {
                return Err(ComptimeError::Arity {
                    name: name.to_string(),
                    expected: func.params.len(),
                    actual: args.len(),
                });
            }
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
                max_operations: self.max_operations,
                operations: Rc::clone(&self.operations),
            };

            if local_ctx.current_depth > local_ctx.recursion_limit {
                return Err(ComptimeError::RecursionLimit);
            }

            // Bind parameters
            for (i, param) in func.params.iter().enumerate() {
                if i < args_values.len() {
                    local_ctx
                        .variables
                        .insert(param.0.clone(), args_values[i].clone());
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
                        return l
                            .checked_add(*r)
                            .map(ComptimeValue::Int)
                            .ok_or(ComptimeError::ArithmeticOverflow("add"));
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
                        return l
                            .checked_sub(*r)
                            .map(ComptimeValue::Int)
                            .ok_or(ComptimeError::ArithmeticOverflow("sub"));
                    }
                }
            }
            TokenKind::Star => {
                if let ComptimeValue::Int(l) = &left_val {
                    if let ComptimeValue::Int(r) = &right_val {
                        return l
                            .checked_mul(*r)
                            .map(ComptimeValue::Int)
                            .ok_or(ComptimeError::ArithmeticOverflow("mul"));
                    }
                }
            }
            TokenKind::Slash => {
                if let ComptimeValue::Int(l) = &left_val {
                    if let ComptimeValue::Int(r) = &right_val {
                        if *r == 0 {
                            return Err(ComptimeError::DivisionByZero);
                        }
                        return l
                            .checked_div(*r)
                            .map(ComptimeValue::Int)
                            .ok_or(ComptimeError::ArithmeticOverflow("div"));
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
                        if *r == 0 {
                            return Err(ComptimeError::DivisionByZero);
                        }
                        return l
                            .checked_rem(*r)
                            .map(ComptimeValue::Int)
                            .ok_or(ComptimeError::ArithmeticOverflow("mod"));
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
                    return n
                        .checked_neg()
                        .map(ComptimeValue::Int)
                        .ok_or(ComptimeError::ArithmeticOverflow("neg"));
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
