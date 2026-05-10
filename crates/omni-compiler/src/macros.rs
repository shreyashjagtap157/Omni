use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MacroArg {
    Literal(String),
    TokenTree(Vec<MacroArg>),
    Repetition(Box<MacroArg>, RepetitionKind),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RepetitionKind {
    ZeroOrMore,
    ZeroOrOne,
    OneOrMore,
}

#[derive(Debug, Clone)]
pub struct MacroRule {
    pattern: Vec<MacroArg>,
    template: Vec<MacroArg>,
}

#[derive(Debug, Clone)]
pub struct MacroDefinition {
    pub name: String,
    pub rules: Vec<MacroRule>,
    pub is_macro_rules: bool,
}

impl MacroDefinition {
    pub fn new(name: &str) -> Self {
        MacroDefinition {
            name: name.to_string(),
            rules: Vec::new(),
            is_macro_rules: false,
        }
    }

    pub fn add_rule(&mut self, pattern: Vec<MacroArg>, template: Vec<MacroArg>) {
        self.rules.push(MacroRule { pattern, template });
    }

    pub fn expand(&self, args: &[MacroArg]) -> Result<Vec<MacroArg>, String> {
        for rule in &self.rules {
            if let Some(bindings) = self.match_pattern(&rule.pattern, args) {
                return Ok(self.apply_template(&rule.template, &bindings));
            }
        }
        Err(format!("No matching rule for macro `{}`", self.name))
    }

    fn match_pattern(
        &self,
        pattern: &[MacroArg],
        args: &[MacroArg],
    ) -> Option<HashMap<String, Vec<MacroArg>>> {
        if pattern.len() != args.len() {
            return None;
        }

        let mut bindings = HashMap::new();

        for (p, a) in pattern.iter().zip(args.iter()) {
            match (p, a) {
                (MacroArg::TokenTree(tokens), arg) => {
                    if !self.match_token_tree(tokens, arg, &mut bindings) {
                        return None;
                    }
                }
                (MacroArg::Literal(lit), MacroArg::Literal(arg_lit)) if lit == arg_lit => {}
                (MacroArg::Literal(name), _) if name.starts_with('$') => {
                    let var_name = name.trim_start_matches('$');
                    bindings
                        .entry(var_name.to_string())
                        .or_insert_with(Vec::new)
                        .push(a.clone());
                }
                _ => return None,
            }
        }

        Some(bindings)
    }

    fn match_token_tree(
        &self,
        pattern: &[MacroArg],
        arg: &MacroArg,
        _bindings: &mut HashMap<String, Vec<MacroArg>>,
    ) -> bool {
        match (pattern, arg) {
            ([], MacroArg::TokenTree(tokens)) if tokens.is_empty() => true,
            ([MacroArg::Literal(lit)], MacroArg::Literal(a)) if lit == a => true,
            ([MacroArg::Repetition(_inner, kind)], MacroArg::TokenTree(tokens)) => match kind {
                RepetitionKind::ZeroOrMore => true,
                RepetitionKind::ZeroOrOne => tokens.len() <= 1,
                RepetitionKind::OneOrMore => !tokens.is_empty(),
            },
            _ => false,
        }
    }

    fn apply_template(
        &self,
        template: &[MacroArg],
        bindings: &HashMap<String, Vec<MacroArg>>,
    ) -> Vec<MacroArg> {
        let mut result = Vec::new();

        for item in template {
            match item {
                MacroArg::Literal(s) => {
                    if s.starts_with('$') {
                        let var_name = s.trim_start_matches('$');
                        if let Some(vals) = bindings.get(var_name) {
                            for v in vals {
                                result.push(v.clone());
                            }
                        }
                    } else {
                        result.push(item.clone());
                    }
                }
                MacroArg::TokenTree(tokens) => {
                    let expanded = self.apply_template(tokens, bindings);
                    if !expanded.is_empty() {
                        result.push(MacroArg::TokenTree(expanded));
                    }
                }
                MacroArg::Repetition(_, _) => {}
            }
        }

        result
    }
}

pub struct MacroExpander {
    macros: HashMap<String, MacroDefinition>,
}

impl MacroExpander {
    pub fn new() -> Self {
        MacroExpander {
            macros: HashMap::new(),
        }
    }

    pub fn define(&mut self, name: &str, def: MacroDefinition) {
        self.macros.insert(name.to_string(), def);
    }

    pub fn expand(&self, name: &str, args: &[MacroArg]) -> Result<Vec<MacroArg>, String> {
        match self.macros.get(name) {
            Some(macro_def) => macro_def.expand(args),
            None => Err(format!("Unknown macro: `{}`", name)),
        }
    }

    pub fn is_defined(&self, name: &str) -> bool {
        self.macros.contains_key(name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeInfo {
    pub name: String,
    pub params: Vec<TypeInfo>,
    pub fields: Vec<(String, TypeInfo)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComptimeValue {
    Int(i64),
    String(String),
    Bool(bool),
    List(Vec<ComptimeValue>),
    Tuple(Vec<ComptimeValue>),
    Type(TypeInfo),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComptimeContext {
    pub ops_budget: usize,
    pub ops_used: usize,
    pub env: HashMap<String, ComptimeValue>,
    pub depth: usize,
}

impl ComptimeContext {
    pub fn new(ops_budget: usize) -> Self {
        ComptimeContext {
            depth: 0,
            ops_budget,
            ops_used: 0,
            env: HashMap::new(),
        }
    }

    pub fn alloc_op(&mut self) -> Result<(), String> {
        self.ops_used += 1;
        if self.ops_used > self.ops_budget {
            Err("Comptime operation budget exceeded".to_string())
        } else {
            Ok(())
        }
    }

    pub fn eval(&mut self, expr: &ComptimeExpr) -> Result<ComptimeValue, String> {
        self.alloc_op()?;

        match expr {
            ComptimeExpr::Int(n) => Ok(ComptimeValue::Int(*n)),
            ComptimeExpr::String(s) => Ok(ComptimeValue::String(s.clone())),
            ComptimeExpr::Bool(b) => Ok(ComptimeValue::Bool(*b)),
            ComptimeExpr::BinOp(op, a, b) => {
                let av = self.eval(a)?;
                let bv = self.eval(b)?;
                self.bin_op(op, &av, &bv)
            }
            ComptimeExpr::UnaryOp(op, a) => {
                let av = self.eval(a)?;
                self.unary_op(op, &av)
            }
            ComptimeExpr::Call(name, args) => self.call(name, args),
            ComptimeExpr::Block(stmts) => self.eval_block(stmts),
            ComptimeExpr::TypeOf(t) => Ok(ComptimeValue::Type(t.clone())),
        }
    }

    fn bin_op(
        &mut self,
        op: &str,
        a: &ComptimeValue,
        b: &ComptimeValue,
    ) -> Result<ComptimeValue, String> {
        match (op, a, b) {
            ("+", ComptimeValue::Int(ai), ComptimeValue::Int(bi)) => {
                Ok(ComptimeValue::Int(ai + bi))
            }
            ("-", ComptimeValue::Int(ai), ComptimeValue::Int(bi)) => {
                Ok(ComptimeValue::Int(ai - bi))
            }
            ("*", ComptimeValue::Int(ai), ComptimeValue::Int(bi)) => {
                Ok(ComptimeValue::Int(ai * bi))
            }
            ("/", ComptimeValue::Int(ai), ComptimeValue::Int(bi)) if *bi != 0 => {
                Ok(ComptimeValue::Int(ai / bi))
            }
            ("%", ComptimeValue::Int(ai), ComptimeValue::Int(bi)) if *bi != 0 => {
                Ok(ComptimeValue::Int(ai % bi))
            }
            ("==", _, _) => Ok(ComptimeValue::Bool(a.eq(b))),
            ("!=", _, _) => Ok(ComptimeValue::Bool(!a.eq(b))),
            ("<", ComptimeValue::Int(ai), ComptimeValue::Int(bi)) => {
                Ok(ComptimeValue::Bool(ai < bi))
            }
            (">", ComptimeValue::Int(ai), ComptimeValue::Int(bi)) => {
                Ok(ComptimeValue::Bool(ai > bi))
            }
            ("<=", ComptimeValue::Int(ai), ComptimeValue::Int(bi)) => {
                Ok(ComptimeValue::Bool(ai <= bi))
            }
            (">=", ComptimeValue::Int(ai), ComptimeValue::Int(bi)) => {
                Ok(ComptimeValue::Bool(ai >= bi))
            }
            ("&&", ComptimeValue::Bool(ai), ComptimeValue::Bool(bi)) => {
                Ok(ComptimeValue::Bool(*ai && *bi))
            }
            ("||", ComptimeValue::Bool(ai), ComptimeValue::Bool(bi)) => {
                Ok(ComptimeValue::Bool(*ai || *bi))
            }
            _ => Err(format!("Invalid binary operation: {} {:?} {:?}", op, a, b)),
        }
    }

    fn unary_op(&mut self, op: &str, a: &ComptimeValue) -> Result<ComptimeValue, String> {
        match (op, a) {
            ("-", ComptimeValue::Int(n)) => Ok(ComptimeValue::Int(-n)),
            ("!", ComptimeValue::Bool(b)) => Ok(ComptimeValue::Bool(!b)),
            _ => Err(format!("Invalid unary operation: {} {:?}", op, a)),
        }
    }

    fn call(&mut self, name: &str, args: &[ComptimeExpr]) -> Result<ComptimeValue, String> {
        match name {
            "sizeof" => {
                if let Some(ComptimeExpr::TypeOf(t)) = args.first() {
                    Ok(ComptimeValue::Int(self.sizeof_type(t) as i64))
                } else {
                    Err("sizeof requires a type".to_string())
                }
            }
            "typeof" => {
                if let Some(arg) = args.first() {
                    let val = self.eval(arg)?;
                    Ok(ComptimeValue::Type(self.type_of_value(&val)))
                } else {
                    Err("typeof requires an expression".to_string())
                }
            }
            "stringify" => {
                if let Some(arg) = args.first() {
                    let val = self.eval(arg)?;
                    Ok(ComptimeValue::String(format!("{:?}", val)))
                } else {
                    Err("stringify requires an expression".to_string())
                }
            }
            _ => Err(format!("Unknown comptime function: {}", name)),
        }
    }

    fn sizeof_type(&self, t: &TypeInfo) -> usize {
        match t.name.as_str() {
            "i8" | "u8" => 1,
            "i16" | "u16" => 2,
            "i32" | "u32" | "f32" => 4,
            "i64" | "u64" | "f64" => 8,
            _ => 8,
        }
    }

    fn type_of_value(&self, v: &ComptimeValue) -> TypeInfo {
        match v {
            ComptimeValue::Int(_) => TypeInfo {
                name: "i64".to_string(),
                params: vec![],
                fields: vec![],
            },
            ComptimeValue::String(_) => TypeInfo {
                name: "String".to_string(),
                params: vec![],
                fields: vec![],
            },
            ComptimeValue::Bool(_) => TypeInfo {
                name: "bool".to_string(),
                params: vec![],
                fields: vec![],
            },
            ComptimeValue::List(_) => TypeInfo {
                name: "Vec".to_string(),
                params: vec![],
                fields: vec![],
            },
            ComptimeValue::Tuple(_) => TypeInfo {
                name: "Tuple".to_string(),
                params: vec![],
                fields: vec![],
            },
            ComptimeValue::Type(t) => t.clone(),
        }
    }

    fn eval_block(&mut self, stmts: &[ComptimeStmt]) -> Result<ComptimeValue, String> {
        let mut result = ComptimeValue::Int(0);
        for stmt in stmts {
            result = self.eval_stmt(stmt)?;
        }
        Ok(result)
    }

    fn eval_stmt(&mut self, stmt: &ComptimeStmt) -> Result<ComptimeValue, String> {
        match stmt {
            ComptimeStmt::Let(name, expr) => {
                let val = self.eval(expr)?;
                self.env.insert(name.clone(), val);
                Ok(ComptimeValue::Int(0))
            }
            ComptimeStmt::Expr(expr) => self.eval(expr),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ComptimeExpr {
    Int(i64),
    String(String),
    Bool(bool),
    BinOp(String, Box<ComptimeExpr>, Box<ComptimeExpr>),
    UnaryOp(String, Box<ComptimeExpr>),
    Call(String, Vec<ComptimeExpr>),
    Block(Vec<ComptimeStmt>),
    TypeOf(TypeInfo),
}

#[derive(Debug, Clone)]
pub enum ComptimeStmt {
    Let(String, ComptimeExpr),
    Expr(ComptimeExpr),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Pattern {
    Wildcard,
    Literal(String),
    Int(i64),
    Bool(bool),
    Variable(String),
    Tuple(Vec<Pattern>),
    Struct(String, Vec<(String, Pattern)>),
    Enum(String, Vec<Pattern>),
    Or(Box<Pattern>, Box<Pattern>),
    Range(Option<Box<Pattern>>, Option<Box<Pattern>>),
}

pub struct PatternMatcher {
    arms: Vec<(Pattern, bool)>,
}

impl PatternMatcher {
    pub fn new() -> Self {
        PatternMatcher { arms: Vec::new() }
    }

    pub fn add_arm(&mut self, pattern: Pattern, guard: bool) {
        self.arms.push((pattern, guard));
    }

    pub fn is_exhaustive(&self, root_type: &str) -> bool {
        match root_type {
            "bool" => {
                self.arms
                    .iter()
                    .any(|(p, _)| matches!(p, Pattern::Bool(true)))
                    && self
                        .arms
                        .iter()
                        .any(|(p, _)| matches!(p, Pattern::Bool(false)))
            }
            "i32" | "i64" | "u32" | "u64" => false,
            _ => false,
        }
    }

    pub fn match_value(&self, value: &PatternValue) -> Option<HashMap<String, PatternValue>> {
        for (pattern, guard) in &self.arms {
            if let Some(bindings) = self.match_pattern(pattern, value) {
                if *guard {
                    return Some(bindings);
                }
            }
        }
        None
    }

    fn match_pattern(
        &self,
        pattern: &Pattern,
        value: &PatternValue,
    ) -> Option<HashMap<String, PatternValue>> {
        match (pattern, value) {
            (Pattern::Wildcard, _) => Some(HashMap::new()),
            (Pattern::Literal(l), PatternValue::Literal(v)) if l == v => Some(HashMap::new()),
            (Pattern::Int(n), PatternValue::Int(v)) if n == v => Some(HashMap::new()),
            (Pattern::Bool(b), PatternValue::Bool(v)) if b == v => Some(HashMap::new()),
            (Pattern::Variable(name), _) => {
                let mut bindings = HashMap::new();
                bindings.insert(name.clone(), value.clone());
                Some(bindings)
            }
            (Pattern::Tuple(ps), PatternValue::Tuple(vs)) if ps.len() == vs.len() => {
                let mut bindings = HashMap::new();
                for (p, v) in ps.iter().zip(vs.iter()) {
                    if let Some(b) = self.match_pattern(p, v) {
                        bindings.extend(b);
                    } else {
                        return None;
                    }
                }
                Some(bindings)
            }
            (Pattern::Or(a, b), v) => {
                if let Some(bindings) = self.match_pattern(a, v) {
                    Some(bindings)
                } else {
                    self.match_pattern(b, v)
                }
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PatternValue {
    Literal(String),
    Int(i64),
    Bool(bool),
    Tuple(Vec<PatternValue>),
    Struct(String, Vec<(String, PatternValue)>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_expand() {
        let mut expander = MacroExpander::new();
        let mut macro_def = MacroDefinition::new("double");
        macro_def.add_rule(
            vec![MacroArg::Literal("$x".to_string())],
            vec![MacroArg::Literal("$x + $x".to_string())],
        );
        expander.define("double", macro_def);

        let result = expander.expand("double", &[MacroArg::Literal("5".to_string())]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_comptime() {
        let mut ctx = ComptimeContext::new(1000);
        let expr = ComptimeExpr::BinOp(
            "+".to_string(),
            Box::new(ComptimeExpr::Int(2)),
            Box::new(ComptimeExpr::Int(3)),
        );
        let result = ctx.eval(&expr);
        assert!(result.is_ok());
    }

    #[test]
    fn test_pattern_matcher() {
        let mut matcher = PatternMatcher::new();
        matcher.add_arm(Pattern::Int(1), false);
        matcher.add_arm(Pattern::Int(2), false);
        matcher.add_arm(Pattern::Wildcard, false);

        assert!(!matcher.is_exhaustive("i32"));
    }
}
