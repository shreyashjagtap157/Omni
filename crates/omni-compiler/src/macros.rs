use std::collections::HashMap;

use crate::complete_lexer::{Token, TokenKind};
use crate::diagnostics::{error_codes, Diagnostic};

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

impl Default for MacroExpander {
    fn default() -> Self {
        MacroExpander::new()
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
    Unit,
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
        self.ops_used = self
            .ops_used
            .checked_add(1)
            .ok_or_else(|| "Comptime operation counter overflow".to_string())?;
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
            ("+", ComptimeValue::Int(ai), ComptimeValue::Int(bi)) => ai
                .checked_add(*bi)
                .map(ComptimeValue::Int)
                .ok_or_else(|| "comptime integer addition overflow".to_string()),
            ("-", ComptimeValue::Int(ai), ComptimeValue::Int(bi)) => ai
                .checked_sub(*bi)
                .map(ComptimeValue::Int)
                .ok_or_else(|| "comptime integer subtraction overflow".to_string()),
            ("*", ComptimeValue::Int(ai), ComptimeValue::Int(bi)) => ai
                .checked_mul(*bi)
                .map(ComptimeValue::Int)
                .ok_or_else(|| "comptime integer multiplication overflow".to_string()),
            ("/", ComptimeValue::Int(ai), ComptimeValue::Int(bi)) => ai
                .checked_div(*bi)
                .map(ComptimeValue::Int)
                .ok_or_else(|| "comptime integer division fault".to_string()),
            ("%", ComptimeValue::Int(ai), ComptimeValue::Int(bi)) => ai
                .checked_rem(*bi)
                .map(ComptimeValue::Int)
                .ok_or_else(|| "comptime integer remainder fault".to_string()),
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
            ("-", ComptimeValue::Int(n)) => n
                .checked_neg()
                .map(ComptimeValue::Int)
                .ok_or_else(|| "comptime integer negation overflow".to_string()),
            ("!", ComptimeValue::Bool(b)) => Ok(ComptimeValue::Bool(!b)),
            _ => Err(format!("Invalid unary operation: {} {:?}", op, a)),
        }
    }

    fn call(&mut self, name: &str, args: &[ComptimeExpr]) -> Result<ComptimeValue, String> {
        match name {
            "sizeof" => {
                if let Some(ComptimeExpr::TypeOf(t)) = args.first() {
                    let size = self.sizeof_type(t)?;
                    let size = i64::try_from(size)
                        .map_err(|_| "sizeof result does not fit i64".to_string())?;
                    Ok(ComptimeValue::Int(size))
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

    fn sizeof_type(&self, t: &TypeInfo) -> Result<usize, String> {
        match t.name.as_str() {
            "i8" | "u8" => Ok(1),
            "i16" | "u16" => Ok(2),
            "i32" | "u32" | "f32" => Ok(4),
            "i64" | "u64" | "f64" => Ok(8),
            other => Err(format!(
                "sizeof is not qualified for type '{}' before the aggregate/layout milestone",
                other
            )),
        }
    }

    fn type_of_value(&self, v: &ComptimeValue) -> TypeInfo {
        match v {
            ComptimeValue::Unit => TypeInfo {
                name: "Unit".to_string(),
                params: vec![],
                fields: vec![],
            },
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
        let mut result = ComptimeValue::Unit;
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
                Ok(ComptimeValue::Unit)
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
                    {
                        let b = self.match_pattern(p, v)?;
                        bindings.extend(b);
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

impl Default for PatternMatcher {
    fn default() -> Self {
        PatternMatcher::new()
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

// ---------------------------------------------------------------------------
// Pre-parse macro expansion — operates on token streams before the parser.
// ---------------------------------------------------------------------------

/// Convert a flat token slice to MacroArgs (each token becomes a Literal).
fn tokens_to_flat_macro_args(tokens: &[Token]) -> Vec<MacroArg> {
    tokens
        .iter()
        .map(|t| MacroArg::Literal(t.text.clone()))
        .collect()
}

/// Convert MacroArgs back to a string that can be re-lexed.
fn macro_args_to_string(args: &[MacroArg]) -> String {
    let mut s = String::new();
    for arg in args {
        match arg {
            MacroArg::Literal(text) => {
                s.push_str(text);
                s.push(' ');
            }
            MacroArg::TokenTree(children) => {
                s.push('(');
                s.push_str(macro_args_to_string(children).trim());
                s.push_str(") ");
            }
            MacroArg::Repetition(inner, kind) => {
                let inner_str = macro_args_to_string(std::slice::from_ref(&**inner));
                let delim = match kind {
                    RepetitionKind::ZeroOrMore => "*",
                    RepetitionKind::ZeroOrOne => "?",
                    RepetitionKind::OneOrMore => "+",
                };
                s.push_str(&inner_str);
                s.push_str(delim);
                s.push(' ');
            }
        }
    }
    s.trim().to_string()
}

/// Find the matching closing token for an opening delimiter, handling nesting.
/// Returns the index of the closing token, or None if not found.
fn find_closing(
    tokens: &[Token],
    open_index: usize,
    open_kind: &TokenKind,
    close_kind: &TokenKind,
) -> Option<usize> {
    let mut depth = 1;
    let mut i = open_index + 1;
    while i < tokens.len() {
        if tokens[i].kind == *open_kind {
            depth += 1;
        } else if tokens[i].kind == *close_kind {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Try to find a macro invocation at position `pos`.
/// Matches `<name> ! ( <tokens> )` or `<name> ! { <tokens> }` or `<name> ! [ <tokens> ]`.
/// Returns (end_index_of_close_delim, name, inner_tokens_between_delimiters).
fn find_macro_invocation(tokens: &[Token], pos: usize) -> Option<(usize, String, Vec<Token>)> {
    if pos + 2 >= tokens.len() {
        return None;
    }
    // Must be: Ident, Bang, then one of LParen/LBrace/LBracket
    if !matches!(tokens[pos].kind, TokenKind::Ident) {
        return None;
    }
    if tokens[pos + 1].kind != TokenKind::Bang {
        return None;
    }

    let name = tokens[pos].text.clone();

    let (open_kind, close_kind) = match tokens[pos + 2].kind {
        TokenKind::LParen => (TokenKind::LParen, TokenKind::RParen),
        TokenKind::LBrace => (TokenKind::LBrace, TokenKind::RBrace),
        TokenKind::LBracket => (TokenKind::LBracket, TokenKind::RBracket),
        _ => return None,
    };

    let close = find_closing(tokens, pos + 2, &open_kind, &close_kind)?;

    // Extract inner tokens between opener and closer (exclusive)
    let inner: Vec<Token> = tokens[pos + 3..close]
        .iter()
        .filter(|t| {
            !matches!(
                t.kind,
                TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent
            )
        })
        .cloned()
        .collect();

    Some((close, name, inner))
}

/// Try to parse a macro definition starting at position `pos`.
/// Format: `macro <name> { ( <pattern> ) => ( <template> ) }`
/// Returns (end_index_of_close, name, rules) where rules = vec![(pattern_args, template_args)].
#[allow(clippy::type_complexity)]
fn parse_macro_definition(
    tokens: &[Token],
    pos: usize,
) -> Option<(usize, String, Vec<(Vec<MacroArg>, Vec<MacroArg>)>)> {
    // Check for `macro` keyword (Ident with text "macro")
    if pos + 2 >= tokens.len() {
        return None;
    }
    if tokens[pos].kind != TokenKind::Ident || tokens[pos].text != "macro" {
        return None;
    }
    if tokens[pos + 1].kind != TokenKind::Ident {
        return None;
    }

    let name = tokens[pos + 1].text.clone();

    // Body starts at pos + 2 — scan past newlines
    let mut body_start = pos + 2;
    while body_start < tokens.len() && tokens[body_start].kind == TokenKind::Newline {
        body_start += 1;
    }

    if body_start >= tokens.len() {
        return None;
    }

    // Determine delimiters for the macro body
    let (body_open_kind, body_close_kind) = match tokens[body_start].kind {
        TokenKind::LBrace => (TokenKind::LBrace, TokenKind::RBrace),
        TokenKind::Indent => (TokenKind::Indent, TokenKind::Dedent),
        _ => return None,
    };

    let body_end = find_closing(tokens, body_start, &body_open_kind, &body_close_kind)?;

    // Parse rules inside body
    let mut rules = Vec::new();
    let mut i = body_start + 1;

    while i < body_end {
        // Skip newlines, comments, indents, dedents
        while i < body_end
            && matches!(
                tokens[i].kind,
                TokenKind::Newline | TokenKind::LineComment | TokenKind::BlockComment
            )
        {
            i += 1;
        }
        if i >= body_end {
            break;
        }

        // Expect `(` for pattern
        if tokens[i].kind != TokenKind::LParen {
            break;
        }

        let pattern_close = find_closing(tokens, i, &TokenKind::LParen, &TokenKind::RParen)?;
        let pattern_args: Vec<MacroArg> = tokens[i + 1..pattern_close]
            .iter()
            .filter(|t| {
                !matches!(
                    t.kind,
                    TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent
                )
            })
            .map(|t| MacroArg::Literal(t.text.clone()))
            .collect();

        i = pattern_close + 1;

        // Skip to `=>`
        while i < body_end && !matches!(tokens[i].kind, TokenKind::FatArrow | TokenKind::Equals) {
            i += 1;
        }
        if i >= body_end {
            break;
        }
        i += 1; // consume FatArrow/Equals

        // Skip whitespace/newlines
        while i < body_end
            && matches!(
                tokens[i].kind,
                TokenKind::Newline | TokenKind::LineComment | TokenKind::BlockComment
            )
        {
            i += 1;
        }

        // Expect `(` for template
        if i >= body_end || tokens[i].kind != TokenKind::LParen {
            break;
        }

        let template_close = find_closing(tokens, i, &TokenKind::LParen, &TokenKind::RParen)?;
        let template_args: Vec<MacroArg> = tokens[i + 1..template_close]
            .iter()
            .filter(|t| {
                !matches!(
                    t.kind,
                    TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent
                )
            })
            .map(|t| MacroArg::Literal(t.text.clone()))
            .collect();

        rules.push((pattern_args, template_args));
        i = template_close + 1;
    }

    Some((body_end, name, rules))
}

/// Expand macros in a token stream **before** parsing.
///
/// # Two-pass algorithm
/// 1. **Pass 1** — Scan for `macro name { ... }` definitions, extract them
///    into the `MacroExpander`, and remove them from the token stream.
/// 2. **Pass 2** — Scan for `name!(...)` invocations, expand each via the
///    `MacroExpander`, re-lex the expanded output, and splice the new tokens
///    into the result stream.
///
/// Unknown macros (not defined) are passed through verbatim so the parser
/// can attempt to handle them or produce a better error message.
#[allow(clippy::type_complexity)]
pub fn expand_tokens_pre_parse(tokens: &[Token]) -> Result<Vec<Token>, Vec<Diagnostic>> {
    let mut expander = MacroExpander::new();
    let mut diagnostics = Vec::new();

    // Pass 1: Extract macro definitions
    let mut no_defs: Vec<Token> = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if let Some((end, name, rules)) = parse_macro_definition(tokens, i) {
            let mut def = MacroDefinition::new(&name);
            for (pattern, template) in rules {
                def.add_rule(pattern, template);
            }
            expander.define(&name, def);
            i = end + 1;
        } else {
            no_defs.push(tokens[i].clone());
            i += 1;
        }
    }

    // Pass 2: Expand macro invocations
    let mut result: Vec<Token> = Vec::new();
    let mut i = 0;
    while i < no_defs.len() {
        if let Some((end, name, inner_tokens)) = find_macro_invocation(&no_defs, i) {
            if expander.is_defined(&name) {
                let args = tokens_to_flat_macro_args(&inner_tokens);
                match expander.expand(&name, &args) {
                    Ok(expanded) => {
                        let expanded_str = macro_args_to_string(&expanded);
                        if !expanded_str.is_empty() {
                            match crate::complete_lexer::tokenize_complete(&expanded_str) {
                                Ok(expanded_tokens) => {
                                    // Omit the automatically-generated Eof from re-lex
                                    let useful: Vec<Token> = expanded_tokens
                                        .into_iter()
                                        .filter(|t| !matches!(t.kind, TokenKind::Eof))
                                        .collect();
                                    result.extend(useful);
                                }
                                Err(e) => {
                                    diagnostics.push(Diagnostic::warning(
                                        error_codes::CODEGEN_UNSUPPORTED_FEATURE,
                                        format!(
                                            "Failed to lexicalize expanded macro '{}': {} (expanded to: '{}')",
                                            name, e.message, expanded_str
                                        ),
                                    ));
                                    // Keep original tokens on failure
                                    result.extend(no_defs[i..=end].iter().cloned());
                                }
                            }
                        }
                    }
                    Err(e) => {
                        diagnostics.push(Diagnostic::warning(
                            error_codes::CODEGEN_UNSUPPORTED_FEATURE,
                            format!("Macro expansion failed for '{}': {}", name, e),
                        ));
                        // Keep original tokens on failure
                        result.extend(no_defs[i..=end].iter().cloned());
                    }
                }
            } else {
                // Unknown macro — pass through for the parser to handle
                result.extend(no_defs[i..=end].iter().cloned());
            }
            i = end + 1;
        } else {
            result.push(no_defs[i].clone());
            i += 1;
        }
    }

    if diagnostics.is_empty() {
        Ok(result)
    } else {
        Err(diagnostics)
    }
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

    #[test]
    fn test_pre_parse_macro_definition_and_expansion() {
        // Build a token stream representing:
        //   macro double {
        //     ($x) => ($x + $x)
        //   }
        //   fn main() -> int
        //     double!(5)
        let tokens = vec![
            Token {
                kind: TokenKind::Ident,
                text: "macro".into(),
                line: 1,
                col: 1,
            },
            Token {
                kind: TokenKind::Ident,
                text: "double".into(),
                line: 1,
                col: 7,
            },
            Token {
                kind: TokenKind::Newline,
                text: "\n".into(),
                line: 1,
                col: 13,
            },
            Token {
                kind: TokenKind::Indent,
                text: String::new(),
                line: 2,
                col: 1,
            },
            Token {
                kind: TokenKind::LParen,
                text: "(".into(),
                line: 2,
                col: 5,
            },
            Token {
                kind: TokenKind::Ident,
                text: "$x".into(),
                line: 2,
                col: 6,
            },
            Token {
                kind: TokenKind::RParen,
                text: ")".into(),
                line: 2,
                col: 8,
            },
            Token {
                kind: TokenKind::FatArrow,
                text: "=>".into(),
                line: 2,
                col: 10,
            },
            Token {
                kind: TokenKind::LParen,
                text: "(".into(),
                line: 2,
                col: 13,
            },
            Token {
                kind: TokenKind::Ident,
                text: "$x".into(),
                line: 2,
                col: 14,
            },
            Token {
                kind: TokenKind::Plus,
                text: "+".into(),
                line: 2,
                col: 16,
            },
            Token {
                kind: TokenKind::Ident,
                text: "$x".into(),
                line: 2,
                col: 18,
            },
            Token {
                kind: TokenKind::RParen,
                text: ")".into(),
                line: 2,
                col: 20,
            },
            Token {
                kind: TokenKind::Newline,
                text: "\n".into(),
                line: 2,
                col: 21,
            },
            Token {
                kind: TokenKind::Dedent,
                text: "dedent".into(),
                line: 3,
                col: 1,
            },
            Token {
                kind: TokenKind::Newline,
                text: "\n".into(),
                line: 3,
                col: 1,
            },
            Token {
                kind: TokenKind::Fn,
                text: "fn".into(),
                line: 4,
                col: 1,
            },
            Token {
                kind: TokenKind::Ident,
                text: "main".into(),
                line: 4,
                col: 4,
            },
            Token {
                kind: TokenKind::LParen,
                text: "(".into(),
                line: 4,
                col: 8,
            },
            Token {
                kind: TokenKind::RParen,
                text: ")".into(),
                line: 4,
                col: 9,
            },
            Token {
                kind: TokenKind::Arrow,
                text: "->".into(),
                line: 4,
                col: 11,
            },
            Token {
                kind: TokenKind::Int,
                text: "int".into(),
                line: 4,
                col: 14,
            },
            Token {
                kind: TokenKind::Newline,
                text: "\n".into(),
                line: 4,
                col: 17,
            },
            Token {
                kind: TokenKind::Indent,
                text: String::new(),
                line: 5,
                col: 1,
            },
            // double!(5)
            Token {
                kind: TokenKind::Ident,
                text: "double".into(),
                line: 5,
                col: 5,
            },
            Token {
                kind: TokenKind::Bang,
                text: "!".into(),
                line: 5,
                col: 11,
            },
            Token {
                kind: TokenKind::LParen,
                text: "(".into(),
                line: 5,
                col: 12,
            },
            Token {
                kind: TokenKind::Number,
                text: "5".into(),
                line: 5,
                col: 13,
            },
            Token {
                kind: TokenKind::RParen,
                text: ")".into(),
                line: 5,
                col: 14,
            },
            Token {
                kind: TokenKind::Newline,
                text: "\n".into(),
                line: 5,
                col: 15,
            },
            Token {
                kind: TokenKind::Dedent,
                text: "dedent".into(),
                line: 6,
                col: 1,
            },
            Token {
                kind: TokenKind::Eof,
                text: String::new(),
                line: 6,
                col: 1,
            },
        ];

        let result = expand_tokens_pre_parse(&tokens);
        assert!(
            result.is_ok(),
            "expansion should succeed: {:?}",
            result.err()
        );

        let expanded = result.unwrap();
        // The macro definition should be removed, and `double!(5)` replaced with `5 + 5`
        let macro_keyword = expanded.iter().find(|t| t.text == "macro");
        assert!(
            macro_keyword.is_none(),
            "macro definition should be removed"
        );

        let has_five = expanded.iter().any(|t| t.text == "5");
        let has_plus = expanded.iter().any(|t| t.text == "+");
        assert!(has_five, "expanded tokens should contain 5");
        assert!(has_plus, "expanded tokens should contain +");
    }

    #[test]
    fn test_macro_args_to_string() {
        let args = vec![
            MacroArg::Literal("5".into()),
            MacroArg::Literal("+".into()),
            MacroArg::Literal("5".into()),
        ];
        let s = macro_args_to_string(&args);
        assert_eq!(s, "5 + 5");
    }

    #[test]
    fn test_find_macro_invocation() {
        let tokens = vec![
            Token {
                kind: TokenKind::Ident,
                text: "foo".into(),
                line: 1,
                col: 1,
            },
            Token {
                kind: TokenKind::Bang,
                text: "!".into(),
                line: 1,
                col: 4,
            },
            Token {
                kind: TokenKind::LParen,
                text: "(".into(),
                line: 1,
                col: 5,
            },
            Token {
                kind: TokenKind::Number,
                text: "42".into(),
                line: 1,
                col: 6,
            },
            Token {
                kind: TokenKind::RParen,
                text: ")".into(),
                line: 1,
                col: 8,
            },
        ];
        let result = find_macro_invocation(&tokens, 0);
        assert!(result.is_some());
        let (end, name, inner) = result.unwrap();
        assert_eq!(end, 4);
        assert_eq!(name, "foo");
        assert_eq!(inner.len(), 1);
        assert_eq!(inner[0].text, "42");
    }
}
