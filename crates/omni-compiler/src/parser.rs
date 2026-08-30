use crate::ast::{Expr, Program, Stmt};
use crate::complete_lexer::{Token, TokenKind};
use crate::diagnostics::Span;
use crate::diagnostics::{error_codes, Diagnostic};

fn is_type_keyword(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Int
            | TokenKind::Int8
            | TokenKind::Int16
            | TokenKind::Int32
            | TokenKind::Int64
            | TokenKind::UInt
            | TokenKind::UInt8
            | TokenKind::UInt16
            | TokenKind::UInt32
            | TokenKind::UInt64
            | TokenKind::Float32
            | TokenKind::Float64
            | TokenKind::Char
            | TokenKind::Byte
            | TokenKind::Bool
            | TokenKind::String
            | TokenKind::Void
    )
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    allow_trailing_struct_literal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
enum Precedence {
    Lowest,
    OrOr,
    AndAnd,
    EqEq,
    Lt,
    Plus,
    Star,
}

impl Precedence {
    fn from_token(kind: &TokenKind) -> Precedence {
        match kind {
            TokenKind::OrOr => Precedence::OrOr,
            TokenKind::AndAnd => Precedence::AndAnd,
            TokenKind::EqEq | TokenKind::NotEq => Precedence::EqEq,
            TokenKind::Lt | TokenKind::LtEq | TokenKind::Gt | TokenKind::GtEq => Precedence::Lt,
            TokenKind::Plus | TokenKind::Minus => Precedence::Plus,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Precedence::Star,
            TokenKind::Equals => Precedence::Lowest,
            _ => Precedence::Lowest,
        }
    }
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        let mut parser = Parser {
            tokens,
            pos: 0,
            allow_trailing_struct_literal: true,
        };
        parser.skip_comments();
        parser
    }

    fn current(&self) -> &Token {
        static DEFAULT_TOKEN: Token = Token {
            kind: TokenKind::Eof,
            text: String::new(),
            line: 0,
            col: 0,
        };
        self.tokens.get(self.pos).unwrap_or(&DEFAULT_TOKEN)
    }

    fn peek(&self) -> &Token {
        static DEFAULT_TOKEN: Token = Token {
            kind: TokenKind::Eof,
            text: String::new(),
            line: 0,
            col: 0,
        };
        self.tokens.get(self.pos + 1).unwrap_or(&DEFAULT_TOKEN)
    }

    fn at(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.current().kind) == std::mem::discriminant(kind)
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        self.skip_comments();
    }

    fn parse_type_name(&mut self) -> Option<String> {
        if self.current().kind == TokenKind::Amp {
            self.advance();
            let mut out = String::from("&");
            if self.current().kind == TokenKind::Mut {
                out.push_str("mut ");
                self.advance();
            }
            let kind = &self.current().kind;
            if matches!(kind, TokenKind::Ident) || is_type_keyword(kind) {
                out.push_str(&self.current().text);
                self.advance();
                return Some(out);
            }
            return None;
        }
        let kind = &self.current().kind;
        if matches!(kind, TokenKind::Ident) || is_type_keyword(kind) {
            let t = Some(self.current().text.clone());
            self.advance();
            t
        } else {
            None
        }
    }

    fn parse_generic_params(&mut self) -> Vec<(String, Vec<String>)> {
        let mut type_params = Vec::new();
        if self.current().kind == TokenKind::LBracket {
            self.advance();
            while self.current().kind != TokenKind::RBracket
                && self.current().kind != TokenKind::Eof
            {
                if self.current().kind == TokenKind::Ident {
                    let param_name = self.current().text.clone();
                    self.advance();

                    let mut bounds = Vec::new();
                    if self.current().kind == TokenKind::Colon {
                        self.advance();
                        loop {
                            let mut bound = String::new();
                            if self.current().kind == TokenKind::Bang {
                                bound.push('!');
                                self.advance();
                            }
                            if self.current().kind == TokenKind::Ident {
                                bound.push_str(&self.current().text);
                                bounds.push(bound);
                                self.advance();
                            } else {
                                break;
                            }
                            if self.current().kind == TokenKind::Plus {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }

                    type_params.push((param_name, bounds));
                    if self.current().kind == TokenKind::Comma {
                        self.advance();
                    }
                } else {
                    self.advance();
                }
            }
            if self.current().kind == TokenKind::RBracket {
                self.advance();
            }
        }
        type_params
    }

    fn skip_comments(&mut self) {
        while self.pos < self.tokens.len() {
            match self.tokens[self.pos].kind {
                TokenKind::LineComment | TokenKind::BlockComment | TokenKind::DocComment => {
                    self.pos += 1;
                }
                _ => break,
            }
        }
    }

    fn parse_type_expr(&mut self) -> String {
        let mut type_str = String::new();
        while self.current().kind != TokenKind::Comma
            && self.current().kind != TokenKind::Newline
            && self.current().kind != TokenKind::Semi
            && self.current().kind != TokenKind::RBracket
            && self.current().kind != TokenKind::RBrace
            && self.current().kind != TokenKind::Dedent
            && self.current().kind != TokenKind::Eof
        {
            type_str.push_str(&self.current().text);
            self.advance();
        }
        type_str.trim().to_string()
    }

    fn recover_from_error(&mut self) {
        while self.current().kind != TokenKind::Newline && self.current().kind != TokenKind::Eof {
            self.advance();
        }
        // Skip statement separators and trivia after the recovery point.
        self.skip_parser_trivia();
    }

    pub fn parse_program(&mut self) -> Result<Program, Diagnostic> {
        let mut prog = Program::new();
        let mut errors: Vec<Diagnostic> = Vec::new();
        while self.current().kind != TokenKind::Eof {
            // Skip blank lines, explicit semicolon separators, and standalone comments.
            self.skip_parser_trivia();
            if self.current().kind == TokenKind::Eof {
                break;
            }

            match self.parse_statement() {
                Ok(stmt) => prog.stmts.push(stmt),
                Err(e) => {
                    errors.push(e);
                    self.recover_from_error();
                    continue;
                }
            }
        }

        if errors.is_empty() {
            Ok(prog)
        } else {
            let all_msgs: Vec<String> = errors.iter().map(|d| d.message.clone()).collect();
            Err(Diagnostic::error(
                error_codes::PARSER_UNEXPECTED_TOKEN,
                all_msgs.join("\n"),
            ))
        }
    }

    pub fn parse_statement(&mut self) -> Result<Stmt, Diagnostic> {
        // Skip any leading newlines or comments
        while self.current().kind == TokenKind::Newline
            || self.current().kind == TokenKind::Semi
            || self.current().kind == TokenKind::LineComment
            || self.current().kind == TokenKind::BlockComment
            || self.current().kind == TokenKind::DocComment
        {
            self.advance();
        }

        let mut fn_prefix_effects: Vec<String> = Vec::new();
        let mut visibility = crate::ast::Visibility::Private;
        loop {
            if self.current().kind == TokenKind::Pub
                || (self.current().kind == TokenKind::Ident && self.current().text == "pub")
            {
                self.advance();
                if self.current().kind == TokenKind::LParen {
                    self.advance();
                    if matches!(
                        self.current().kind,
                        TokenKind::Ident | TokenKind::Mod | TokenKind::Cap | TokenKind::Friend
                    ) {
                        let modifier = self.current().text.clone();
                        self.advance();
                        match modifier.as_str() {
                            "mod" => visibility = crate::ast::Visibility::PubMod,
                            "pkg" => visibility = crate::ast::Visibility::PubPkg,
                            "cap" => {
                                if self.current().kind == TokenKind::Colon {
                                    self.advance();
                                    if self.current().kind == TokenKind::Ident {
                                        let cap_name = self.current().text.clone();
                                        self.advance();
                                        visibility = crate::ast::Visibility::PubCap(cap_name);
                                    }
                                }
                            }
                            "friend" => {
                                if self.current().kind == TokenKind::Colon {
                                    self.advance();
                                    if self.current().kind == TokenKind::Ident {
                                        let friend_mod = self.current().text.clone();
                                        self.advance();
                                        visibility = crate::ast::Visibility::PubFriend(friend_mod);
                                    }
                                }
                            }
                            _ => visibility = crate::ast::Visibility::Pub,
                        }
                    } else {
                        visibility = crate::ast::Visibility::Pub;
                    }
                    if self.current().kind == TokenKind::RParen {
                        self.advance();
                    }
                } else {
                    visibility = crate::ast::Visibility::Pub;
                }
                continue;
            }
            if self.current().kind == TokenKind::Async
                || (self.current().kind == TokenKind::Ident && self.current().text == "async")
            {
                self.advance();
                fn_prefix_effects.push("async".to_string());
                continue;
            }
            if self.current().kind == TokenKind::Ident && self.current().text == "io" {
                self.advance();
                fn_prefix_effects.push("io".to_string());
                continue;
            }
            if self.current().kind == TokenKind::Ident && self.current().text == "panic" {
                self.advance();
                fn_prefix_effects.push("panic".to_string());
                continue;
            }
            if self.current().kind == TokenKind::Ident && self.current().text == "comptime" {
                self.advance();
                fn_prefix_effects.push("comptime".to_string());
                continue;
            }
            if self.current().kind == TokenKind::Ident && self.current().text == "pure" {
                self.advance();
                fn_prefix_effects.push("pure".to_string());
                continue;
            }
            break;
        }

        // If we accidentally start at an Indent (nested block), consume
        // it and parse a block of statements to avoid unexpected Indent
        // errors in nested contexts.
        if self.current().kind == TokenKind::Indent {
            let indent_tok = self.current().clone();
            self.advance();
            let mut stmts: Vec<Stmt> = Vec::new();
            while self.current().kind != TokenKind::Dedent && self.current().kind != TokenKind::Eof
            {
                // Skip statement separators and trivia inside indentation-delimited blocks.
                self.skip_parser_trivia();
                if self.current().kind == TokenKind::Dedent || self.current().kind == TokenKind::Eof
                {
                    break;
                }
                {
                    let s = self.parse_statement()?;
                    stmts.push(s)
                }
            }
            if self.current().kind == TokenKind::Dedent {
                self.advance();
            }
            return Ok(Stmt::Block(
                stmts,
                Span::from_token(indent_tok.line, indent_tok.col, &indent_tok.text),
            ));
        }

        let tok = self.current().clone();

        if tok.kind == TokenKind::At {
            self.advance();
            let name_tok = self.current();
            if name_tok.kind != TokenKind::Ident {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    format!(
                        "Expected identifier after '@' at {}:{}",
                        name_tok.line, name_tok.col
                    ),
                ));
            }
            let annot = name_tok.text.clone();
            self.advance();
            return Ok(Stmt::Annotation(
                annot,
                Span::from_token(tok.line, tok.col, &tok.text),
            ));
        }

        // Accept explicit `If` token from the lexer as a keyword.
        if tok.kind == TokenKind::If {
            return self.parse_if();
        }

        if tok.kind == TokenKind::Ident
            && tok.text == "sealed"
            && self.peek().kind == TokenKind::Enum
        {
            self.advance();
            let mut parsed = self.parse_enum(visibility)?;
            if let Stmt::Enum { is_sealed, .. } = &mut parsed {
                *is_sealed = true;
            }
            return Ok(parsed);
        }

        if tok.kind == TokenKind::Enum {
            return self.parse_enum(visibility);
        }

        // Handle keyword tokens from complete_lexer
        if tok.kind == TokenKind::Let {
            self.advance();
            let is_mut = if self.current().kind == TokenKind::Mut {
                self.advance();
                true
            } else {
                false
            };
            let name_tok = self.current();
            if name_tok.kind != TokenKind::Ident {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    format!(
                        "Expected identifier after 'let' at {}:{}",
                        name_tok.line, name_tok.col
                    ),
                ));
            }
            let name = name_tok.text.clone();
            self.advance();

            let type_annot = if self.current().kind == TokenKind::Colon {
                self.advance();
                self.parse_type_name()
            } else {
                None
            };

            if self.current().kind != TokenKind::Equals {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    format!(
                        "Expected '=' after identifier at {}:{}",
                        self.current().line,
                        self.current().col
                    ),
                ));
            }
            self.advance();
            let expr = self.parse_expression(Precedence::Lowest)?;
            let span = Span::from_token(tok.line, tok.col, &tok.text);
            return Ok(if is_mut {
                Stmt::LetMut(name, type_annot, expr, span)
            } else {
                Stmt::Let(name, type_annot, expr, span)
            });
        }

        if tok.kind == TokenKind::Fn {
            return self.parse_function(visibility, fn_prefix_effects);
        }

        if tok.kind == TokenKind::Return {
            self.advance();
            // Bare `return` (no expression) — return unit
            if matches!(
                self.current().kind,
                TokenKind::Newline
                    | TokenKind::Dedent
                    | TokenKind::Indent
                    | TokenKind::RBrace
                    | TokenKind::RBracket
                    | TokenKind::Semi
                    | TokenKind::Eof
            ) {
                return Ok(Stmt::Return(
                    Expr::Tuple(vec![], Span::from_token(tok.line, tok.col, &tok.text)),
                    Span::from_token(tok.line, tok.col, &tok.text),
                ));
            }
            let expr = self.parse_expression(Precedence::Lowest)?;
            return Ok(Stmt::Return(
                expr,
                Span::from_token(tok.line, tok.col, &tok.text),
            ));
        }

        if tok.kind == TokenKind::For {
            return self.parse_for();
        }

        if tok.kind == TokenKind::While {
            return self.parse_while();
        }

        if tok.kind == TokenKind::Loop {
            return self.parse_loop();
        }

        if tok.kind == TokenKind::Struct {
            return self.parse_struct(visibility);
        }

        if tok.kind == TokenKind::Impl {
            return self.parse_impl(visibility);
        }

        if tok.kind == TokenKind::Trait {
            return self.parse_trait(visibility);
        }

        if tok.kind == TokenKind::Type {
            return self.parse_type_alias(visibility);
        }

        if tok.kind == TokenKind::Use {
            return self.parse_use();
        }

        if tok.kind == TokenKind::Break {
            self.advance();
            return Ok(Stmt::Break(Span::from_token(tok.line, tok.col, &tok.text)));
        }

        if tok.kind == TokenKind::Continue {
            self.advance();
            return Ok(Stmt::Continue(Span::from_token(
                tok.line, tok.col, &tok.text,
            )));
        }

        if tok.kind == TokenKind::Defer {
            return self.parse_defer();
        }

        if tok.kind == TokenKind::Spawn {
            return self.parse_spawn();
        }

        if tok.kind == TokenKind::Ident && tok.text == "error" {
            let next_tok = self.peek();
            if next_tok.kind != TokenKind::Colon {
                return self.parse_error_set(visibility);
            }
        }

        if tok.kind == TokenKind::Ident {
            if tok.text == "print" {
                self.advance();
                let expr = self.parse_expression(Precedence::Lowest)?;
                return Ok(Stmt::Print(
                    expr,
                    Span::from_token(tok.line, tok.col, &tok.text),
                ));
            } else if tok.text == "let" {
                self.advance();
                let name_tok = self.current();
                if name_tok.kind != TokenKind::Ident {
                    return Err(Diagnostic::error(
                        error_codes::PARSER_MISSING_TOKEN,
                        format!(
                            "Expected identifier after 'let' at {}:{}",
                            name_tok.line, name_tok.col
                        ),
                    ));
                }
                let name = name_tok.text.clone();
                self.advance();

                let type_annot = if self.current().kind == TokenKind::Colon {
                    self.advance();
                    self.parse_type_name()
                } else {
                    None
                };

                if self.current().kind != TokenKind::Equals {
                    return Err(Diagnostic::error(
                        error_codes::PARSER_MISSING_TOKEN,
                        format!(
                            "Expected '=' after identifier at {}:{}",
                            self.current().line,
                            self.current().col
                        ),
                    ));
                }
                self.advance();
                let expr = self.parse_expression(Precedence::Lowest)?;
                return Ok(Stmt::Let(
                    name,
                    type_annot,
                    expr,
                    Span::from_token(tok.line, tok.col, &tok.text),
                ));
            } else if tok.text == "fn" {
                return self.parse_function(visibility, fn_prefix_effects);
            } else if tok.text == "if" {
                return self.parse_if();
            } else if tok.text == "loop" {
                return self.parse_loop();
            } else if tok.text == "return" {
                self.advance();
                let expr = self.parse_expression(Precedence::Lowest)?;
                return Ok(Stmt::Return(
                    expr,
                    Span::from_token(tok.line, tok.col, &tok.text),
                ));
            } else if tok.text == "for" {
                return self.parse_for();
            } else if tok.text == "while" {
                return self.parse_while();
            } else if tok.text == "struct" {
                return self.parse_struct(visibility);
            } else if tok.text == "impl" {
                return self.parse_impl(visibility);
            } else if tok.text == "trait" {
                return self.parse_trait(visibility);
            } else if tok.text == "type" {
                return self.parse_type_alias(visibility);
            } else if tok.text == "use" {
                return self.parse_use();
            } else if tok.text == "gc_mode" {
                return self.parse_gc_mode();
            } else if tok.text == "handle" {
                return self.parse_effect_handler();
            } else if tok.text == "spawn" {
                return self.parse_spawn();
            } else if tok.text == "channel" {
                return self.parse_channel();
            } else if tok.text == "actor" {
                return self.parse_actor();
            } else if tok.text == "executor" {
                return self.parse_work_stealing();
            } else if tok.text == "deterministic" {
                return self.parse_deterministic_runtime();
            } else if tok.text == "tensor" {
                return self.parse_tensor();
            } else if tok.text == "simd" {
                return self.parse_simd();
            } else if tok.text == "doc_comment" {
                return self.parse_doc_comment();
            } else if tok.text == "debug" {
                return self.parse_debug_session();
            } else if tok.text == "capability" {
                return self.parse_capability();
            } else if tok.text == "sandbox" {
                return self.parse_ffi_sandbox();
            } else if tok.text == "break" {
                self.advance();
                return Ok(Stmt::Break(Span::from_token(tok.line, tok.col, &tok.text)));
            } else if tok.text == "continue" {
                self.advance();
                return Ok(Stmt::Continue(Span::from_token(
                    tok.line, tok.col, &tok.text,
                )));
            }
        }

        if tok.kind == TokenKind::Linear {
            self.advance();
            // Historical v0.1.1 compatibility: accept `linear struct Name` while
            // the canonical formatter emits `struct Name linear`. Both produce
            // the same AST and later semantic stages decide whether the feature
            // is supported for the selected language edition.
            if self.current().kind == TokenKind::Struct {
                let mut stmt = self.parse_struct(visibility)?;
                if let Stmt::Struct { is_linear, .. } = &mut stmt {
                    *is_linear = true;
                }
                return Ok(stmt);
            }
            if self.current().kind != TokenKind::Ident {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    format!(
                        "Expected identifier after 'linear' at {}:{}",
                        self.current().line,
                        self.current().col
                    ),
                ));
            }
            let name_tok = self.current();
            let name = name_tok.text.clone();
            self.advance();

            let type_annot = if self.current().kind == TokenKind::Colon {
                self.advance();
                self.parse_type_name()
            } else {
                None
            };

            if self.current().kind != TokenKind::Equals {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    format!(
                        "Expected '=' after identifier at {}:{}",
                        self.current().line,
                        self.current().col
                    ),
                ));
            }
            self.advance();
            let expr = self.parse_expression(Precedence::Lowest)?;
            return Ok(Stmt::LetLinear(
                name,
                type_annot,
                expr,
                Span::from_token(tok.line, tok.col, &tok.text),
            ));
        }

        if tok.kind == TokenKind::Unsafe {
            return self.parse_unsafe();
        }

        if tok.kind == TokenKind::Mod {
            return self.parse_mod();
        }

        // Historical Stage-0 source used `module package.path;` declarations.
        // Preserve that spelling as a declaration so old source remains valid
        // without weakening the rule against executable top-level statements.
        if tok.kind == TokenKind::Ident && tok.text == "module" {
            return self.parse_historical_module_decl();
        }

        // Dereference assignment through a qualified mutable reference.
        if tok.kind == TokenKind::Star
            && self.peek().kind == TokenKind::Ident
            && self
                .tokens
                .get(self.pos + 2)
                .is_some_and(|t| t.kind == TokenKind::Equals)
        {
            let span = Span::from_token(tok.line, tok.col, &tok.text);
            self.advance(); // *
            let reference = Expr::Var(self.current().text.clone(), span.clone());
            self.advance(); // reference name
            self.advance(); // =
            let value = self.parse_expression(Precedence::Lowest)?;
            return Ok(Stmt::DerefAssign(Box::new(reference), value, span));
        }

        // Direct field assignment is also a statement. Keep the place structure
        // intact so ownership checking can distinguish reinitializing `p.x` from
        // assigning the whole aggregate `p`. Deeper projections are parsed by
        // the general expression grammar and remain fail-closed for now.
        if tok.kind == TokenKind::Ident
            && self.peek().kind == TokenKind::Dot
            && self
                .tokens
                .get(self.pos + 2)
                .is_some_and(|t| t.kind == TokenKind::Ident)
            && self
                .tokens
                .get(self.pos + 3)
                .is_some_and(|t| t.kind == TokenKind::Equals)
        {
            let base_name = tok.text.clone();
            let span = Span::from_token(tok.line, tok.col, &tok.text);
            self.advance(); // base
            self.advance(); // dot
            let field = self.current().text.clone();
            self.advance(); // field
            self.advance(); // equals
            let value = self.parse_expression(Precedence::Lowest)?;
            let base = Expr::Var(base_name, span.clone());
            return Ok(Stmt::ExprFieldAssign(Box::new(base), field, value, span));
        }

        // Plain assignment is a statement, not an expression in Edition 2026.
        // Recognize it before general expression parsing so `x = value` cannot
        // leave the `=` token stranded after parsing `x` as an expression.
        if tok.kind == TokenKind::Ident && self.peek().kind == TokenKind::Equals {
            let name = tok.text.clone();
            self.advance();
            self.advance();
            let value = self.parse_expression(Precedence::Lowest)?;
            return Ok(Stmt::Assign(
                name,
                value,
                Span::from_token(tok.line, tok.col, &tok.text),
            ));
        }

        let expr = self.parse_expression(Precedence::Lowest)?;
        Ok(Stmt::ExprStmt(
            expr,
            Span::from_token(tok.line, tok.col, &tok.text),
        ))
    }

    fn skip_parser_trivia(&mut self) {
        while matches!(
            self.current().kind.clone(),
            TokenKind::Newline
                | TokenKind::Semi
                | TokenKind::LineComment
                | TokenKind::BlockComment
                | TokenKind::DocComment
        ) {
            self.advance();
        }
    }

    fn parse_statement_block(&mut self, context: &str) -> Result<Vec<Stmt>, Diagnostic> {
        let (opener, closer) = match self.current().kind.clone() {
            TokenKind::LBrace => (TokenKind::LBrace, TokenKind::RBrace),
            TokenKind::LBracket => (TokenKind::LBracket, TokenKind::RBracket),
            TokenKind::Indent => (TokenKind::Indent, TokenKind::Dedent),
            _ => {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    format!(
                        "Expected block body for {context} at {}:{}",
                        self.current().line,
                        self.current().col
                    ),
                ));
            }
        };
        let start = self.current().clone();
        self.advance();

        let mut body = Vec::new();
        loop {
            self.skip_parser_trivia();
            if self.current().kind == closer {
                self.advance();
                return Ok(body);
            }
            if self.current().kind == TokenKind::Eof {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    format!(
                        "Unterminated {context} block opened by {:?} at {}:{}; expected {:?}",
                        opener, start.line, start.col, closer
                    ),
                ));
            }
            if matches!(
                self.current().kind,
                TokenKind::RBrace | TokenKind::RBracket | TokenKind::Dedent
            ) {
                return Err(Diagnostic::error(
                    error_codes::PARSER_UNEXPECTED_TOKEN,
                    format!(
                        "Mismatched closing delimiter {:?} in {context}; expected {:?} at {}:{}",
                        self.current().kind,
                        closer,
                        self.current().line,
                        self.current().col
                    ),
                ));
            }
            body.push(self.parse_statement()?);
        }
    }

    fn parse_function(
        &mut self,
        visibility: crate::ast::Visibility,
        mut effects: Vec<String>,
    ) -> Result<Stmt, Diagnostic> {
        let fn_tok = self.current().clone();
        self.advance(); // consume 'fn'
        let name_tok = self.current();
        if name_tok.kind != TokenKind::Ident {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                format!(
                    "Expected function name at {}:{}",
                    name_tok.line, name_tok.col
                ),
            ));
        }
        let name = name_tok.text.clone();
        self.advance();

        let type_params = self.parse_generic_params();

        if self.current().kind != TokenKind::LParen {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                format!(
                    "Expected '(' after function name at {}:{}",
                    self.current().line,
                    self.current().col
                ),
            ));
        }
        self.advance();

        let mut params = Vec::new();
        while self.current().kind != TokenKind::RParen {
            if self.current().kind == TokenKind::Eof {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    "Unterminated function parameter list; expected ')'",
                ));
            }
            if matches!(
                self.current().kind,
                TokenKind::LineComment
                    | TokenKind::BlockComment
                    | TokenKind::DocComment
                    | TokenKind::Newline
            ) {
                self.advance();
                continue;
            }
            if self.current().kind != TokenKind::Ident {
                return Err(Diagnostic::error(
                    error_codes::PARSER_UNEXPECTED_TOKEN,
                    format!(
                        "Expected function parameter or ')' at {}:{}, found {:?}",
                        self.current().line,
                        self.current().col,
                        self.current().kind
                    ),
                ));
            }
            let param_name = self.current().text.clone();
            self.advance();
            let param_type = if self.current().kind == TokenKind::Colon {
                self.advance();
                self.parse_type_name()
            } else {
                None
            };
            params.push((param_name, param_type));
            if self.current().kind == TokenKind::Comma {
                self.advance();
            } else if self.current().kind != TokenKind::RParen {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    format!(
                        "Expected ',' or ')' after function parameter at {}:{}",
                        self.current().line,
                        self.current().col
                    ),
                ));
            }
        }
        self.advance();

        // optional return type: '-> Type'
        let mut ret_type: Option<String> = None;
        if self.current().kind == TokenKind::Arrow {
            self.advance();
            ret_type = self.parse_type_name();
        }

        // Optional effect annotation: `fn foo() -> T / io + async`.
        if self.current().kind == TokenKind::Slash {
            self.advance();
            let mut current_effect = String::new();
            while self.current().kind != TokenKind::Newline
                && self.current().kind != TokenKind::Indent
                && self.current().kind != TokenKind::Dedent
                && self.current().kind != TokenKind::Eof
                && self.current().kind != TokenKind::Colon
                && self.current().kind != TokenKind::LBrace
                && self.current().kind != TokenKind::LBracket
                && self.current().kind != TokenKind::Semi
                && self.current().kind != TokenKind::At
            {
                if self.current().kind == TokenKind::Plus || self.current().kind == TokenKind::Comma
                {
                    if !current_effect.trim().is_empty() {
                        effects.push(current_effect.trim().to_string());
                        current_effect.clear();
                    }
                    self.advance();
                    continue;
                }
                current_effect.push_str(&self.current().text);
                self.advance();
            }
            if !current_effect.trim().is_empty() {
                effects.push(current_effect.trim().to_string());
            }
        }

        if self.current().kind == TokenKind::Colon {
            self.advance();
        }

        // Skip any newlines/comments before body
        while self.current().kind == TokenKind::Newline
            || self.current().kind == TokenKind::LineComment
            || self.current().kind == TokenKind::BlockComment
            || self.current().kind == TokenKind::DocComment
        {
            self.advance();
        }

        let mut body = Vec::new();
        let mut contracts = Vec::new();

        while self.current().kind == TokenKind::At {
            self.advance();
            if self.current().kind == TokenKind::Ident {
                let attr_name = self.current().text.clone();
                self.advance();
                match attr_name.as_str() {
                    "requires" if self.current().kind == TokenKind::LParen => {
                        self.advance();
                        let cond = self.parse_expression(Precedence::Lowest)?;
                        let mut msg = String::new();
                        if self.current().kind == TokenKind::Comma {
                            self.advance();
                            if self.current().kind != TokenKind::StringLiteral {
                                return Err(Diagnostic::error(
                                    error_codes::PARSER_MISSING_TOKEN,
                                    "Expected string message after ',' in @requires",
                                ));
                            }
                            msg = self.current().text.clone();
                            self.advance();
                        }
                        if self.current().kind != TokenKind::RParen {
                            return Err(Diagnostic::error(
                                error_codes::PARSER_MISSING_TOKEN,
                                "Expected ')' to close @requires",
                            ));
                        }
                        self.advance();
                        contracts.push(Stmt::ContractRequires {
                            condition: cond,
                            message: msg,
                            span: Span::default(),
                        });
                    }
                    "ensures" if self.current().kind == TokenKind::LParen => {
                        self.advance();
                        let cond = self.parse_expression(Precedence::Lowest)?;
                        let mut msg = String::new();
                        if self.current().kind == TokenKind::Comma {
                            self.advance();
                            if self.current().kind != TokenKind::StringLiteral {
                                return Err(Diagnostic::error(
                                    error_codes::PARSER_MISSING_TOKEN,
                                    "Expected string message after ',' in @ensures",
                                ));
                            }
                            msg = self.current().text.clone();
                            self.advance();
                        }
                        if self.current().kind != TokenKind::RParen {
                            return Err(Diagnostic::error(
                                error_codes::PARSER_MISSING_TOKEN,
                                "Expected ')' to close @ensures",
                            ));
                        }
                        self.advance();
                        contracts.push(Stmt::ContractEnsures {
                            condition: cond,
                            message: msg,
                            span: Span::default(),
                        });
                    }
                    "invariant" if self.current().kind == TokenKind::LParen => {
                        self.advance();
                        let cond = self.parse_expression(Precedence::Lowest)?;
                        let mut msg = String::new();
                        if self.current().kind == TokenKind::Comma {
                            self.advance();
                            if self.current().kind == TokenKind::StringLiteral {
                                msg = self.current().text.clone();
                                self.advance();
                            } else {
                                return Err(Diagnostic::error(
                                    error_codes::PARSER_MISSING_TOKEN,
                                    "Expected string message after ',' in @invariant",
                                ));
                            }
                        }
                        if self.current().kind != TokenKind::RParen {
                            return Err(Diagnostic::error(
                                error_codes::PARSER_MISSING_TOKEN,
                                "Expected ')' to close @invariant",
                            ));
                        }
                        self.advance();
                        contracts.push(Stmt::ContractInvariant {
                            condition: cond,
                            message: msg,
                            span: Span::default(),
                        });
                    }
                    "comptime_limit" if self.current().kind == TokenKind::LParen => {
                        self.advance();
                        if self.current().kind != TokenKind::Ident || self.current().text != "ops" {
                            return Err(Diagnostic::error(
                                error_codes::PARSER_MISSING_TOKEN,
                                "Expected 'ops' in @comptime_limit",
                            ));
                        }
                        self.advance();
                        if self.current().kind != TokenKind::Colon {
                            return Err(Diagnostic::error(
                                error_codes::PARSER_MISSING_TOKEN,
                                "Expected ':' after ops in @comptime_limit",
                            ));
                        }
                        self.advance();
                        if self.current().kind != TokenKind::Number {
                            return Err(Diagnostic::error(
                                error_codes::PARSER_MISSING_TOKEN,
                                "Expected numeric ops value in @comptime_limit",
                            ));
                        }
                        let max_ops = self.current().text.parse::<u64>().map_err(|_| {
                            Diagnostic::error(
                                error_codes::PARSER_UNEXPECTED_TOKEN,
                                format!(
                                    "Invalid @comptime_limit ops value '{}'",
                                    self.current().text
                                ),
                            )
                        })?;
                        self.advance();
                        if self.current().kind != TokenKind::RParen {
                            return Err(Diagnostic::error(
                                error_codes::PARSER_MISSING_TOKEN,
                                "Expected ')' to close @comptime_limit",
                            ));
                        }
                        self.advance();
                        contracts.push(Stmt::ComptimeLimit {
                            max_ops,
                            span: Span::default(),
                        });
                    }
                    _ => {
                        return Err(Diagnostic::error(
                            error_codes::PARSER_UNEXPECTED_TOKEN,
                            format!("Unsupported function contract attribute '@{}'", attr_name),
                        ));
                    }
                }
            }
            while self.current().kind == TokenKind::Newline
                || self.current().kind == TokenKind::LineComment
                || self.current().kind == TokenKind::BlockComment
                || self.current().kind == TokenKind::DocComment
            {
                self.advance();
            }
        }

        // Allow either an indented/braced block OR a single-line inline `return` after the signature.
        if matches!(
            self.current().kind,
            TokenKind::LBracket | TokenKind::LBrace | TokenKind::Indent
        ) {
            body = self.parse_statement_block("function")?;
        } else if self.current().kind == TokenKind::Return
            || (self.current().kind == TokenKind::Ident && self.current().text == "return")
        {
            // parse a single-line return as the function body
            {
                let s = self.parse_statement()?;
                body.push(s)
            }
        } else if self.current().kind == TokenKind::Semi {
            self.advance(); // Trait method without a body
        } else {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                format!(
                    "Expected function body or ';' at {}:{}",
                    self.current().line,
                    self.current().col
                ),
            ));
        }

        Ok(Stmt::Fn {
            name,
            visibility,
            is_async: effects.contains(&"async".to_string()),
            type_params,
            params,
            ret_type,
            effects,
            contracts,
            body,
            span: Span::from_token(fn_tok.line, fn_tok.col, &fn_tok.text),
        })
    }

    fn parse_if(&mut self) -> Result<Stmt, Diagnostic> {
        let if_tok = self.current().clone();
        self.advance();
        let cond = self.parse_expression_before_block()?;

        let bindings = Vec::new();
        self.skip_parser_trivia();
        if self.current().kind == TokenKind::Then {
            self.advance();
            self.skip_parser_trivia();
        }
        let then_body = self.parse_statement_block("if")?;

        self.skip_parser_trivia();
        let mut else_body = Vec::new();
        if self.current().kind == TokenKind::Else
            || (self.current().kind == TokenKind::Ident && self.current().text == "else")
        {
            self.advance();
            self.skip_parser_trivia();
            if self.current().kind == TokenKind::Then {
                self.advance();
                self.skip_parser_trivia();
            }
            if self.current().kind == TokenKind::If
                || (self.current().kind == TokenKind::Ident && self.current().text == "if")
            {
                else_body.push(self.parse_if()?);
            } else {
                else_body = self.parse_statement_block("else")?;
            }
        }

        Ok(Stmt::If {
            cond: Box::new(cond),
            bindings,
            then_body,
            else_body,
            span: Span::from_token(if_tok.line, if_tok.col, &if_tok.text),
        })
    }

    fn parse_loop(&mut self) -> Result<Stmt, Diagnostic> {
        let loop_tok = self.current().clone();
        self.advance();
        self.skip_parser_trivia();
        let body = self.parse_statement_block("loop")?;

        Ok(Stmt::Loop {
            body,
            span: Span::from_token(loop_tok.line, loop_tok.col, &loop_tok.text),
        })
    }

    fn parse_defer(&mut self) -> Result<Stmt, Diagnostic> {
        let defer_tok = self.current().clone();
        self.advance();
        self.skip_parser_trivia();
        let cleanup = self.parse_statement_block("defer")?;
        Ok(Stmt::Defer {
            cleanup: Box::new(Stmt::Block(
                cleanup,
                Span::from_token(defer_tok.line, defer_tok.col, &defer_tok.text),
            )),
            span: Span::from_token(defer_tok.line, defer_tok.col, &defer_tok.text),
        })
    }

    fn parse_for(&mut self) -> Result<Stmt, Diagnostic> {
        let for_tok = self.current().clone();
        self.advance();
        let var_tok = self.current();
        if var_tok.kind != TokenKind::Ident {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                format!(
                    "Expected variable name in for at {}:{}",
                    var_tok.line, var_tok.col
                ),
            ));
        }
        let var_name = var_tok.text.clone();
        self.advance();

        if self.current().kind == TokenKind::In
            || (self.current().kind == TokenKind::Ident && self.current().text == "in")
        {
            self.advance();
        } else {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                format!(
                    "Expected 'in' in for at {}:{}",
                    self.current().line,
                    self.current().col
                ),
            ));
        }

        let iterable = self.parse_expression_before_block()?;
        self.skip_parser_trivia();
        let body = self.parse_statement_block("for")?;

        Ok(Stmt::For {
            var_name,
            iterable: Box::new(iterable),
            body,
            span: Span::from_token(for_tok.line, for_tok.col, &for_tok.text),
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, Diagnostic> {
        let while_tok = self.current().clone();
        self.advance();

        // Transitional v0.1.x compatibility form: `while item in iterable { ... }`.
        // Edition-1 does not rely on this construct, but the historical v0.1.2
        // formatter milestone promised semantic preservation for the existing AST.
        if self.current().kind == TokenKind::Ident
            && (self.peek().kind == TokenKind::In
                || (self.peek().kind == TokenKind::Ident && self.peek().text == "in"))
        {
            let var_name = self.current().text.clone();
            self.advance();
            self.advance(); // `in`
            let iterable = self.parse_expression_before_block()?;
            self.skip_parser_trivia();
            let body = self.parse_statement_block("while-in")?;
            return Ok(Stmt::WhileIn {
                var_name,
                iterable: Box::new(iterable),
                body,
                span: Span::from_token(while_tok.line, while_tok.col, &while_tok.text),
            });
        }

        let cond = self.parse_expression_before_block()?;
        self.skip_parser_trivia();
        let body = self.parse_statement_block("while")?;

        Ok(Stmt::While {
            cond: Box::new(cond),
            body,
            span: Span::from_token(while_tok.line, while_tok.col, &while_tok.text),
        })
    }

    fn parse_struct(&mut self, visibility: crate::ast::Visibility) -> Result<Stmt, Diagnostic> {
        let struct_tok = self.current().clone();
        self.advance();
        let name_tok = self.current();
        if name_tok.kind != TokenKind::Ident {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                format!("Expected struct name at {}:{}", name_tok.line, name_tok.col),
            ));
        }
        let name = name_tok.text.clone();
        self.advance();

        let mut is_linear = false;
        if self.current().kind == TokenKind::Linear {
            is_linear = true;
            self.advance();
        }

        let mut fields = Vec::new();
        let mut generic_params: Vec<String> = Vec::new();
        if self.current().kind == TokenKind::LBracket {
            self.advance();
            while self.current().kind != TokenKind::RBracket
                && self.current().kind != TokenKind::Eof
            {
                if self.current().kind == TokenKind::Ident {
                    let param = self.current().text.clone();
                    self.advance();
                    if self.current().kind == TokenKind::Colon {
                        self.advance();
                        let field_type = self.parse_type_expr();
                        generic_params.clear();
                        fields.push((param, field_type));
                    } else {
                        generic_params.push(param);
                        if self.current().kind == TokenKind::Comma {
                            self.advance();
                        }
                    }
                } else {
                    self.advance();
                }
            }
            if self.current().kind == TokenKind::RBracket {
                self.advance();
            }
            if !generic_params.is_empty()
                && (self.current().kind == TokenKind::LBrace
                    || self.current().kind == TokenKind::Colon)
            {
                let is_brace = self.current().kind == TokenKind::LBrace;
                if is_brace {
                    self.advance();
                } else {
                    self.advance();
                    while self.current().kind == TokenKind::Newline {
                        self.advance();
                    }
                }
                while self.current().kind != TokenKind::Dedent
                    && self.current().kind != TokenKind::Eof
                    && self.current().kind != TokenKind::RBrace
                {
                    if self.current().kind == TokenKind::Newline
                        || self.current().kind == TokenKind::Indent
                    {
                        self.advance();
                    } else if self.current().kind == TokenKind::Ident {
                        let field_name = self.current().text.clone();
                        self.advance();
                        if self.current().kind == TokenKind::Colon {
                            self.advance();
                            let field_type = self.parse_type_expr();
                            fields.push((field_name, field_type));
                        }
                        while self.current().kind == TokenKind::Newline {
                            self.advance();
                        }
                        if self.current().kind == TokenKind::Comma {
                            self.advance();
                        }
                    } else if self.current().kind == TokenKind::RBrace {
                        self.advance();
                        break;
                    } else {
                        self.advance();
                    }
                }
                if is_brace && self.current().kind == TokenKind::RBrace {
                    self.advance();
                }
            }
        } else if self.current().kind == TokenKind::Colon {
            self.advance();
            while self.current().kind == TokenKind::Newline {
                self.advance();
            }
            while self.current().kind != TokenKind::Dedent && self.current().kind != TokenKind::Eof
            {
                if self.current().kind == TokenKind::Newline
                    || self.current().kind == TokenKind::Indent
                {
                    self.advance();
                } else if self.current().kind == TokenKind::Dedent {
                    self.advance();
                    break;
                } else {
                    let field_name_tok = self.current();
                    if field_name_tok.kind == TokenKind::Ident {
                        let field_name = field_name_tok.text.clone();
                        self.advance();
                        if self.current().kind == TokenKind::Colon {
                            self.advance();
                            let field_type = self.parse_type_expr();
                            fields.push((field_name, field_type));
                        }
                        while self.current().kind == TokenKind::Newline {
                            self.advance();
                        }
                        if self.current().kind == TokenKind::Comma {
                            self.advance();
                        }
                    } else if self.current().kind == TokenKind::RBrace {
                        self.advance();
                        break;
                    } else {
                        self.advance();
                    }
                }
            }
        } else if self.current().kind == TokenKind::LBrace {
            self.advance();
            while self.current().kind != TokenKind::Dedent
                && self.current().kind != TokenKind::Eof
                && self.current().kind != TokenKind::RBrace
            {
                if self.current().kind == TokenKind::Newline
                    || self.current().kind == TokenKind::Indent
                {
                    self.advance();
                } else if self.current().kind == TokenKind::Ident {
                    let field_name = self.current().text.clone();
                    self.advance();
                    if self.current().kind == TokenKind::Colon {
                        self.advance();
                        let field_type = self.parse_type_expr();
                        fields.push((field_name, field_type));
                    }
                    while self.current().kind == TokenKind::Newline {
                        self.advance();
                    }
                    if self.current().kind == TokenKind::Comma {
                        self.advance();
                    }
                } else if self.current().kind == TokenKind::RBrace {
                    self.advance();
                    break;
                } else {
                    self.advance();
                }
            }
            if self.current().kind == TokenKind::RBrace {
                self.advance();
            }
        }

        Ok(Stmt::Struct {
            name,
            visibility,
            fields,
            is_linear,
            span: Span::from_token(struct_tok.line, struct_tok.col, &struct_tok.text),
        })
    }

    fn parse_impl(&mut self, visibility: crate::ast::Visibility) -> Result<Stmt, Diagnostic> {
        let impl_tok = self.current().clone();
        self.advance();
        let target_tok = self.current();
        if target_tok.kind != TokenKind::Ident {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                format!(
                    "Expected impl target name at {}:{}",
                    target_tok.line, target_tok.col
                ),
            ));
        }
        let target = target_tok.text.clone();
        self.advance();
        let type_params = self.parse_generic_params();

        let mut for_type: Option<String> = None;
        self.skip_parser_trivia();
        if (self.current().kind == TokenKind::Ident && self.current().text == "for")
            || self.current().kind == TokenKind::For
        {
            self.advance();
            if self.current().kind != TokenKind::Ident {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    format!(
                        "Expected type name after 'for' at {}:{}",
                        self.current().line,
                        self.current().col
                    ),
                ));
            }
            for_type = Some(self.current().text.clone());
            self.advance();
        }

        self.skip_parser_trivia();
        if self.current().kind == TokenKind::Colon {
            self.advance();
            self.skip_parser_trivia();
        }

        let close = match self.current().kind.clone() {
            TokenKind::LBrace => TokenKind::RBrace,
            TokenKind::LBracket => TokenKind::RBracket,
            TokenKind::Indent => TokenKind::Dedent,
            _ => {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    format!(
                        "Expected impl body at {}:{}",
                        self.current().line,
                        self.current().col
                    ),
                ));
            }
        };
        self.advance();

        let mut methods = Vec::new();
        loop {
            self.skip_parser_trivia();
            if self.current().kind == close || self.current().kind == TokenKind::Eof {
                break;
            }
            let method = self.parse_statement()?;
            if !matches!(method, Stmt::Fn { .. }) {
                return Err(Diagnostic::error(
                    error_codes::PARSER_INVALID_STATEMENT,
                    "Only function declarations are permitted directly inside impl blocks in v0.1.4",
                ));
            }
            methods.push(method);
        }
        if self.current().kind != close {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                "Unterminated impl body",
            ));
        }
        self.advance();

        Ok(Stmt::Impl {
            target,
            visibility,
            type_params,
            for_type,
            methods,
            span: Span::from_token(impl_tok.line, impl_tok.col, &impl_tok.text),
        })
    }

    fn parse_trait(&mut self, visibility: crate::ast::Visibility) -> Result<Stmt, Diagnostic> {
        let trait_tok = self.current().clone();
        self.advance();
        let name_tok = self.current();
        if name_tok.kind != TokenKind::Ident {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                format!("Expected trait name at {}:{}", name_tok.line, name_tok.col),
            ));
        }
        let name = name_tok.text.clone();
        self.advance();
        let type_params = self.parse_generic_params();
        self.skip_parser_trivia();
        if self.current().kind == TokenKind::Colon {
            self.advance();
            self.skip_parser_trivia();
        }

        let close = match self.current().kind.clone() {
            TokenKind::LBrace => TokenKind::RBrace,
            TokenKind::LBracket => TokenKind::RBracket,
            TokenKind::Indent => TokenKind::Dedent,
            _ => {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    format!(
                        "Expected trait body at {}:{}",
                        self.current().line,
                        self.current().col
                    ),
                ));
            }
        };
        self.advance();

        let mut methods = Vec::new();
        let mut diagnostic_attrs = Vec::new();
        loop {
            self.skip_parser_trivia();
            if self.current().kind == close || self.current().kind == TokenKind::Eof {
                break;
            }
            if self.current().kind == TokenKind::At {
                self.advance();
                if self.current().kind != TokenKind::Ident || self.current().text != "diagnostic" {
                    return Err(Diagnostic::error(
                        error_codes::PARSER_UNEXPECTED_TOKEN,
                        "Only @diagnostic::on_unimplemented is supported directly inside traits in v0.1.4",
                    ));
                }
                self.advance();
                if self.current().kind != TokenKind::ColonColon {
                    return Err(Diagnostic::error(
                        error_codes::PARSER_MISSING_TOKEN,
                        "Expected '::' after @diagnostic",
                    ));
                }
                self.advance();
                if self.current().kind != TokenKind::Ident
                    || self.current().text != "on_unimplemented"
                {
                    return Err(Diagnostic::error(
                        error_codes::PARSER_UNEXPECTED_TOKEN,
                        "Expected on_unimplemented diagnostic attribute",
                    ));
                }
                self.advance();
                if self.current().kind != TokenKind::LParen {
                    return Err(Diagnostic::error(
                        error_codes::PARSER_MISSING_TOKEN,
                        "Expected '(' after on_unimplemented",
                    ));
                }
                self.advance();
                let mut message = String::new();
                let mut label = None;
                while self.current().kind != TokenKind::RParen
                    && self.current().kind != TokenKind::Eof
                {
                    if self.current().kind != TokenKind::Ident {
                        return Err(Diagnostic::error(
                            error_codes::PARSER_UNEXPECTED_TOKEN,
                            "Expected diagnostic attribute key",
                        ));
                    }
                    let key = self.current().text.clone();
                    self.advance();
                    if self.current().kind != TokenKind::Equals {
                        return Err(Diagnostic::error(
                            error_codes::PARSER_MISSING_TOKEN,
                            "Expected '=' in diagnostic attribute",
                        ));
                    }
                    self.advance();
                    if self.current().kind != TokenKind::StringLiteral {
                        return Err(Diagnostic::error(
                            error_codes::PARSER_MISSING_TOKEN,
                            "Expected string value in diagnostic attribute",
                        ));
                    }
                    let value = self.current().text.clone();
                    self.advance();
                    match key.as_str() {
                        "message" => message = value,
                        "label" => label = Some(value),
                        _ => {
                            return Err(Diagnostic::error(
                                error_codes::PARSER_UNEXPECTED_TOKEN,
                                format!("Unknown diagnostic attribute key '{key}'"),
                            ))
                        }
                    }
                    if self.current().kind == TokenKind::Comma {
                        self.advance();
                    } else {
                        break;
                    }
                }
                if self.current().kind != TokenKind::RParen {
                    return Err(Diagnostic::error(
                        error_codes::PARSER_MISSING_TOKEN,
                        "Unterminated diagnostic attribute",
                    ));
                }
                self.advance();
                diagnostic_attrs.push(crate::ast::DiagnosticAttribute { message, label });
                continue;
            }

            let method = self.parse_statement()?;
            if !matches!(method, Stmt::Fn { .. }) {
                return Err(Diagnostic::error(
                    error_codes::PARSER_INVALID_STATEMENT,
                    "Only function declarations are permitted directly inside trait blocks in v0.1.4",
                ));
            }
            methods.push(method);
        }
        if self.current().kind != close {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                "Unterminated trait body",
            ));
        }
        self.advance();

        Ok(Stmt::Trait {
            name,
            visibility,
            type_params,
            methods,
            diagnostic_attrs,
            span: Span::from_token(trait_tok.line, trait_tok.col, &trait_tok.text),
        })
    }

    fn parse_type_alias(&mut self, visibility: crate::ast::Visibility) -> Result<Stmt, Diagnostic> {
        let type_tok = self.current().clone();
        self.advance();
        let name_tok = self.current();
        if name_tok.kind != TokenKind::Ident {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                format!(
                    "Expected type alias name at {}:{}",
                    name_tok.line, name_tok.col
                ),
            ));
        }
        let name = name_tok.text.clone();
        self.advance();

        let type_params = self.parse_generic_params();

        while self.current().kind == TokenKind::Newline
            || self.current().kind == TokenKind::LineComment
            || self.current().kind == TokenKind::BlockComment
            || self.current().kind == TokenKind::DocComment
        {
            self.advance();
        }

        if self.current().kind == TokenKind::Equals {
            self.advance();
            while self.current().kind == TokenKind::Newline
                || self.current().kind == TokenKind::LineComment
                || self.current().kind == TokenKind::BlockComment
                || self.current().kind == TokenKind::DocComment
            {
                self.advance();
            }
        }

        let target = self.parse_type_expr();
        while self.current().kind != TokenKind::Newline
            && self.current().kind != TokenKind::Dedent
            && self.current().kind != TokenKind::Eof
        {
            self.advance();
        }

        Ok(Stmt::TypeAlias {
            name,
            visibility,
            type_params,
            target,
            span: Span::from_token(type_tok.line, type_tok.col, &type_tok.text),
        })
    }

    fn parse_use(&mut self) -> Result<Stmt, Diagnostic> {
        let use_tok = self.current().clone();
        self.advance();

        let mut path = String::new();
        let mut aliases: Vec<(String, Option<String>)> = Vec::new();

        if self.current().kind == TokenKind::Ident {
            path = self.current().text.clone();
            self.advance();

            while matches!(self.current().kind, TokenKind::ColonColon | TokenKind::Dot) {
                let separator = if self.current().kind == TokenKind::Dot {
                    "."
                } else {
                    "::"
                };
                self.advance();
                if self.current().kind != TokenKind::Ident {
                    return Err(Diagnostic::error(
                        error_codes::PARSER_MISSING_TOKEN,
                        format!(
                            "Expected import path segment after '{separator}' at {}:{}",
                            self.current().line,
                            self.current().col
                        ),
                    ));
                }
                path.push_str(separator);
                path.push_str(&self.current().text);
                self.advance();
            }
        }

        while self.current().kind == TokenKind::Newline
            || self.current().kind == TokenKind::LineComment
            || self.current().kind == TokenKind::BlockComment
            || self.current().kind == TokenKind::DocComment
        {
            self.advance();
        }

        if self.current().text == "as" {
            self.advance();
            if self.current().kind == TokenKind::Ident {
                aliases.push((path.clone(), Some(self.current().text.clone())));
                path.clear();
                self.advance();
            }
        }

        if self.current().kind == TokenKind::Comma {
            self.advance();
            while self.current().kind == TokenKind::Ident {
                let item_path = self.current().text.clone();
                self.advance();
                let mut item_alias = None;
                if self.current().text == "as" {
                    self.advance();
                    if self.current().kind == TokenKind::Ident {
                        item_alias = Some(self.current().text.clone());
                        self.advance();
                    }
                }
                aliases.push((item_path, item_alias));
                if self.current().kind == TokenKind::Comma {
                    self.advance();
                }
            }
        }

        while self.current().kind == TokenKind::Newline
            || self.current().kind == TokenKind::LineComment
            || self.current().kind == TokenKind::BlockComment
            || self.current().kind == TokenKind::DocComment
        {
            self.advance();
        }

        if self.current().text == "in" {
            self.advance();
            if self.current().kind == TokenKind::Colon {
                self.advance();
            }
            while self.current().kind == TokenKind::Newline
                || self.current().kind == TokenKind::LineComment
                || self.current().kind == TokenKind::BlockComment
                || self.current().kind == TokenKind::DocComment
            {
                self.advance();
            }

            let mut body = Vec::new();
            if self.current().kind == TokenKind::Indent {
                self.advance();
                while self.current().kind != TokenKind::Dedent
                    && self.current().kind != TokenKind::Eof
                {
                    self.skip_parser_trivia();
                    if self.current().kind == TokenKind::Dedent
                        || self.current().kind == TokenKind::Eof
                    {
                        break;
                    }
                    {
                        let s = self.parse_statement()?;
                        body.push(s)
                    }
                }
                if self.current().kind == TokenKind::Dedent {
                    self.advance();
                }
            }

            return Ok(Stmt::UseScoped {
                path,
                aliases,
                body,
                span: Span::from_token(use_tok.line, use_tok.col, &use_tok.text),
            });
        }

        Ok(Stmt::Use {
            path,
            alias: aliases.first().and_then(|(_, a)| a.clone()),
            span: Span::from_token(use_tok.line, use_tok.col, &use_tok.text),
        })
    }

    fn parse_gc_mode(&mut self) -> Result<Stmt, Diagnostic> {
        let gc_tok = self.current().clone();
        self.advance();
        let mode_tok = self.current();
        if mode_tok.kind != TokenKind::Ident {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                format!("Expected GC mode at {}:{}", mode_tok.line, mode_tok.col),
            ));
        }
        let mode = mode_tok.text.clone();
        if mode_tok.kind != TokenKind::Eof {
            self.advance();
        }
        Ok(Stmt::GcMode {
            mode,
            span: Span::from_token(gc_tok.line, gc_tok.col, &gc_tok.text),
        })
    }

    fn parse_effect_handler(&mut self) -> Result<Stmt, Diagnostic> {
        let handle_tok = self.current().clone();
        self.advance();
        if self.current().kind != TokenKind::Ident {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                format!(
                    "Expected effect name after 'handle' at {}:{}",
                    self.current().line,
                    self.current().col
                ),
            ));
        }
        let effect = self.current().text.clone();
        self.advance();

        if self.current().kind == TokenKind::In
            || (self.current().kind == TokenKind::Ident && self.current().text == "in")
        {
            self.advance();
        }
        if self.current().kind == TokenKind::Colon {
            self.advance();
        }
        while self.current().kind == TokenKind::Newline
            || self.current().kind == TokenKind::LineComment
            || self.current().kind == TokenKind::BlockComment
            || self.current().kind == TokenKind::DocComment
        {
            self.advance();
        }

        if self.current().kind != TokenKind::LBracket
            && self.current().kind != TokenKind::LBrace
            && self.current().kind != TokenKind::Indent
        {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                format!(
                    "Expected handler block after effect '{}' at {}:{}",
                    effect,
                    self.current().line,
                    self.current().col
                ),
            ));
        }

        let body = self.parse_statement_block("effect handler")?;

        Ok(Stmt::EffectHandler {
            effect,
            handler: Box::new(Expr::Block(
                body,
                Span::from_token(handle_tok.line, handle_tok.col, &handle_tok.text),
            )),
            span: Span::from_token(handle_tok.line, handle_tok.col, &handle_tok.text),
        })
    }

    fn parse_spawn(&mut self) -> Result<Stmt, Diagnostic> {
        let spawn_tok = self.current().clone();
        self.advance();
        let task = self.parse_expression(Precedence::Lowest)?;
        Ok(Stmt::Spawn {
            task: Box::new(task),
            span: Span::from_token(spawn_tok.line, spawn_tok.col, &spawn_tok.text),
        })
    }

    fn parse_channel(&mut self) -> Result<Stmt, Diagnostic> {
        let channel_tok = self.current().clone();
        self.advance();
        if self.current().kind != TokenKind::Ident {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                format!(
                    "Expected channel element type at {}:{}",
                    self.current().line,
                    self.current().col
                ),
            ));
        }
        let elem_type = self.current().text.clone();
        self.advance();
        let mut capacity = None;
        if self.current().kind == TokenKind::LBracket {
            self.advance();
            if self.current().kind != TokenKind::Number {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    "Expected channel capacity",
                ));
            }
            capacity = Some(self.current().text.parse::<u32>().map_err(|_| {
                Diagnostic::error(
                    error_codes::PARSER_UNEXPECTED_TOKEN,
                    format!("Invalid channel capacity '{}'", self.current().text),
                )
            })?);
            self.advance();
            if self.current().kind != TokenKind::RBracket {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    "Expected ']' after channel capacity",
                ));
            }
            self.advance();
        }
        Ok(Stmt::Channel {
            elem_type,
            capacity,
            span: Span::from_token(channel_tok.line, channel_tok.col, &channel_tok.text),
        })
    }

    fn parse_actor(&mut self) -> Result<Stmt, Diagnostic> {
        let actor_tok = self.current().clone();
        self.advance();
        let name_tok = self.current();
        if name_tok.kind != TokenKind::Ident {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                format!("Expected actor name at {}:{}", name_tok.line, name_tok.col),
            ));
        }
        let name = name_tok.text.clone();
        self.advance();
        let state_type = if self.current().kind == TokenKind::Ident {
            let t = self.current().text.clone();
            self.advance();
            t
        } else {
            "()".to_string()
        };
        let handlers = Vec::new();
        Ok(Stmt::Actor {
            name,
            state: state_type,
            handlers,
            span: Span::from_token(actor_tok.line, actor_tok.col, &actor_tok.text),
        })
    }

    fn parse_work_stealing(&mut self) -> Result<Stmt, Diagnostic> {
        let executor_tok = self.current().clone();
        self.advance();
        let num_threads = if self.current().kind == TokenKind::LBracket {
            self.advance();
            if self.current().kind != TokenKind::Number {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    "Expected executor thread count",
                ));
            }
            let n = self.current().text.parse::<u32>().map_err(|_| {
                Diagnostic::error(
                    error_codes::PARSER_UNEXPECTED_TOKEN,
                    format!("Invalid executor thread count '{}'", self.current().text),
                )
            })?;
            self.advance();
            if self.current().kind != TokenKind::RBracket {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    "Expected ']' after executor thread count",
                ));
            }
            self.advance();
            n
        } else {
            4
        };
        let queue_type = "mpsc".to_string();
        Ok(Stmt::WorkStealingExecutor {
            num_threads,
            queue_type,
            span: Span::from_token(executor_tok.line, executor_tok.col, &executor_tok.text),
        })
    }

    fn parse_deterministic_runtime(&mut self) -> Result<Stmt, Diagnostic> {
        let det_tok = self.current().clone();
        self.advance();
        let max_tasks = if self.current().kind == TokenKind::LBracket {
            self.advance();
            if self.current().kind != TokenKind::Number {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    "Expected deterministic task limit",
                ));
            }
            let n = self.current().text.parse::<u32>().map_err(|_| {
                Diagnostic::error(
                    error_codes::PARSER_UNEXPECTED_TOKEN,
                    format!("Invalid deterministic task limit '{}'", self.current().text),
                )
            })?;
            self.advance();
            if self.current().kind != TokenKind::RBracket {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    "Expected ']' after deterministic task limit",
                ));
            }
            self.advance();
            n
        } else {
            1000
        };
        Ok(Stmt::DeterministicRuntime {
            max_tasks,
            span: Span::from_token(det_tok.line, det_tok.col, &det_tok.text),
        })
    }

    fn parse_tensor(&mut self) -> Result<Stmt, Diagnostic> {
        let tensor_tok = self.current().clone();
        self.advance();
        let mut shape = Vec::new();
        if self.current().kind == TokenKind::LBracket {
            self.advance();
            while self.current().kind != TokenKind::RBracket
                && self.current().kind != TokenKind::Eof
            {
                if self.current().kind != TokenKind::Number {
                    return Err(Diagnostic::error(
                        error_codes::PARSER_UNEXPECTED_TOKEN,
                        "Expected numeric tensor dimension",
                    ));
                }
                let n = self.current().text.parse::<u32>().map_err(|_| {
                    Diagnostic::error(
                        error_codes::PARSER_UNEXPECTED_TOKEN,
                        format!("Invalid tensor dimension '{}'", self.current().text),
                    )
                })?;
                shape.push(n);
                self.advance();
                if self.current().kind == TokenKind::Comma {
                    self.advance();
                } else if self.current().kind != TokenKind::RBracket {
                    return Err(Diagnostic::error(
                        error_codes::PARSER_MISSING_TOKEN,
                        "Expected ',' or ']' in tensor shape",
                    ));
                }
            }
            if self.current().kind != TokenKind::RBracket {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    "Unterminated tensor shape",
                ));
            }
            self.advance();
        }
        let dtype = if self.current().kind == TokenKind::Ident {
            let t = self.current().text.clone();
            self.advance();
            t
        } else {
            "f32".to_string()
        };
        Ok(Stmt::Tensor {
            shape,
            dtype,
            span: Span::from_token(tensor_tok.line, tensor_tok.col, &tensor_tok.text),
        })
    }

    fn parse_simd(&mut self) -> Result<Stmt, Diagnostic> {
        let simd_tok = self.current().clone();
        self.advance();
        let width = if self.current().kind == TokenKind::LBracket {
            self.advance();
            if self.current().kind != TokenKind::Number {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    "Expected SIMD width",
                ));
            }
            let n = self.current().text.parse::<u32>().map_err(|_| {
                Diagnostic::error(
                    error_codes::PARSER_UNEXPECTED_TOKEN,
                    format!("Invalid SIMD width '{}'", self.current().text),
                )
            })?;
            self.advance();
            if self.current().kind != TokenKind::RBracket {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    "Expected ']' after SIMD width",
                ));
            }
            self.advance();
            n
        } else {
            4
        };
        let elem_type = if self.current().kind == TokenKind::Ident {
            let t = self.current().text.clone();
            self.advance();
            t
        } else {
            "f32".to_string()
        };
        Ok(Stmt::Simd {
            width,
            elem_type,
            span: Span::from_token(simd_tok.line, simd_tok.col, &simd_tok.text),
        })
    }

    fn parse_doc_comment(&mut self) -> Result<Stmt, Diagnostic> {
        let doc_tok = self.current().clone();
        self.advance();
        let target = if self.current().kind == TokenKind::Ident {
            let t = self.current().text.clone();
            self.advance();
            t
        } else {
            "function".to_string()
        };
        let mut content = String::new();
        while self.current().kind == TokenKind::Newline
            || self.current().kind == TokenKind::LineComment
        {
            if self.current().kind == TokenKind::LineComment {
                content.push_str(&self.current().text);
                content.push('\n');
            }
            self.advance();
        }
        Ok(Stmt::DocComment {
            target,
            content,
            span: Span::from_token(doc_tok.line, doc_tok.col, &doc_tok.text),
        })
    }

    fn parse_debug_session(&mut self) -> Result<Stmt, Diagnostic> {
        let debug_tok = self.current().clone();
        self.advance();
        let port = if self.current().kind == TokenKind::LBracket {
            self.advance();
            if self.current().kind != TokenKind::Number {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    "Expected debug port",
                ));
            }
            let p = self.current().text.parse::<u32>().map_err(|_| {
                Diagnostic::error(
                    error_codes::PARSER_UNEXPECTED_TOKEN,
                    format!("Invalid debug port '{}'", self.current().text),
                )
            })?;
            self.advance();
            if self.current().kind != TokenKind::RBracket {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    "Expected ']' after debug port",
                ));
            }
            self.advance();
            p
        } else {
            4711
        };
        Ok(Stmt::DebugSession {
            port,
            breakpoints: Vec::new(),
            span: Span::from_token(debug_tok.line, debug_tok.col, &debug_tok.text),
        })
    }

    fn parse_capability(&mut self) -> Result<Stmt, Diagnostic> {
        let cap_tok = self.current().clone();
        self.advance();
        if self.current().kind != TokenKind::Ident {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                format!(
                    "Expected capability name at {}:{}",
                    self.current().line,
                    self.current().col
                ),
            ));
        }
        let name = self.current().text.clone();
        self.advance();
        let mut permissions = Vec::new();
        if self.current().kind == TokenKind::LBracket {
            self.advance();
            while self.current().kind != TokenKind::RBracket
                && self.current().kind != TokenKind::Eof
            {
                if self.current().kind == TokenKind::Ident {
                    permissions.push(self.current().text.clone());
                    self.advance();
                }
                if self.current().kind == TokenKind::Comma {
                    self.advance();
                }
            }
            if self.current().kind == TokenKind::RBracket {
                self.advance();
            }
        }
        Ok(Stmt::Capability {
            name,
            permissions,
            span: Span::from_token(cap_tok.line, cap_tok.col, &cap_tok.text),
        })
    }

    fn parse_ffi_sandbox(&mut self) -> Result<Stmt, Diagnostic> {
        let sandbox_tok = self.current().clone();
        self.advance();
        let mut allow_list = Vec::new();
        if self.current().kind == TokenKind::LBracket {
            self.advance();
            while self.current().kind != TokenKind::RBracket
                && self.current().kind != TokenKind::Eof
            {
                if self.current().kind == TokenKind::Ident {
                    allow_list.push(self.current().text.clone());
                    self.advance();
                }
                if self.current().kind == TokenKind::Comma {
                    self.advance();
                }
            }
            if self.current().kind == TokenKind::RBracket {
                self.advance();
            }
        }
        Ok(Stmt::FfiSandbox {
            allow_list,
            span: Span::from_token(sandbox_tok.line, sandbox_tok.col, &sandbox_tok.text),
        })
    }

    fn parse_enum(&mut self, visibility: crate::ast::Visibility) -> Result<Stmt, Diagnostic> {
        let enum_tok = self.current().clone();
        self.advance();
        let name_tok = self.current();
        if name_tok.kind != TokenKind::Ident {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                format!("Expected enum name at {}:{}", name_tok.line, name_tok.col),
            ));
        }
        let name = name_tok.text.clone();
        self.advance();

        let mut is_sealed = false;
        if self.current().kind == TokenKind::Ident && self.current().text == "sealed" {
            is_sealed = true;
            self.advance();
        }

        let mut variants = Vec::new();
        if self.current().kind == TokenKind::LBracket
            || self.current().kind == TokenKind::LBrace
            || self.current().kind == TokenKind::Indent
        {
            self.advance();
            while self.current().kind != TokenKind::RBracket
                && self.current().kind != TokenKind::RBrace
                && self.current().kind != TokenKind::Dedent
                && self.current().kind != TokenKind::Eof
            {
                while self.current().kind == TokenKind::Newline
                    || self.current().kind == TokenKind::LineComment
                    || self.current().kind == TokenKind::BlockComment
                    || self.current().kind == TokenKind::DocComment
                {
                    self.advance();
                }
                if self.current().kind == TokenKind::RBracket
                    || self.current().kind == TokenKind::RBrace
                    || self.current().kind == TokenKind::Dedent
                    || self.current().kind == TokenKind::Eof
                {
                    break;
                }

                if self.current().kind == TokenKind::Variant
                    || (self.current().kind == TokenKind::Ident && self.current().text == "variant")
                {
                    self.advance();
                }

                let variant_name_tok = self.current();
                if variant_name_tok.kind != TokenKind::Ident {
                    return Err(Diagnostic::error(
                        error_codes::PARSER_MISSING_TOKEN,
                        format!(
                            "Expected enum variant name at {}:{}",
                            variant_name_tok.line, variant_name_tok.col
                        ),
                    ));
                }
                let variant_name = variant_name_tok.text.clone();
                self.advance();

                let mut fields = Vec::new();
                if self.current().kind == TokenKind::LBracket {
                    self.advance();
                    while self.current().kind != TokenKind::RBracket
                        && self.current().kind != TokenKind::Eof
                    {
                        if self.current().kind == TokenKind::Ident {
                            let field_name = self.current().text.clone();
                            self.advance();
                            if self.current().kind == TokenKind::Colon {
                                self.advance();
                                let field_type_tok = self.current();
                                let field_type = field_type_tok.text.clone();
                                fields.push((field_name, field_type));
                                self.advance();
                            }
                            if self.current().kind == TokenKind::Comma {
                                self.advance();
                            }
                        } else {
                            self.advance();
                        }
                    }
                    if self.current().kind == TokenKind::RBracket {
                        self.advance();
                    }
                }

                variants.push(crate::ast::EnumVariant {
                    name: variant_name,
                    fields,
                });

                if self.current().kind == TokenKind::Comma {
                    self.advance();
                }
            }
            if self.current().kind == TokenKind::RBracket
                || self.current().kind == TokenKind::RBrace
                || self.current().kind == TokenKind::Dedent
            {
                self.advance();
            }
        }

        Ok(Stmt::Enum {
            name,
            visibility,
            variants,
            is_sealed,
            span: Span::from_token(enum_tok.line, enum_tok.col, &enum_tok.text),
        })
    }

    fn parse_error_set(&mut self, visibility: crate::ast::Visibility) -> Result<Stmt, Diagnostic> {
        let error_tok = self.current().clone();
        self.advance();
        // Accept both the historical `error Name [...]` spelling and the
        // canonical formatter spelling `error set Name { ... }`.
        if self.current().kind == TokenKind::Ident && self.current().text == "set" {
            self.advance();
        }
        let name_tok = self.current();
        if name_tok.kind != TokenKind::Ident {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                format!(
                    "Expected error set name at {}:{}",
                    name_tok.line, name_tok.col
                ),
            ));
        }
        let name = name_tok.text.clone();
        self.advance();
        self.skip_parser_trivia();
        if self.current().kind == TokenKind::Colon {
            self.advance();
            self.skip_parser_trivia();
        }

        let close = match self.current().kind.clone() {
            TokenKind::LBracket => TokenKind::RBracket,
            TokenKind::LBrace => TokenKind::RBrace,
            TokenKind::Indent => TokenKind::Dedent,
            _ => {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    format!(
                        "Expected error set body at {}:{}",
                        self.current().line,
                        self.current().col
                    ),
                ));
            }
        };
        self.advance();
        let mut variants = Vec::new();
        loop {
            self.skip_parser_trivia();
            if self.current().kind == close || self.current().kind == TokenKind::Eof {
                break;
            }
            if self.current().kind == TokenKind::Variant
                || (self.current().kind == TokenKind::Ident && self.current().text == "variant")
            {
                self.advance();
            }
            if self.current().kind != TokenKind::Ident {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    format!(
                        "Expected error variant name at {}:{}",
                        self.current().line,
                        self.current().col
                    ),
                ));
            }
            let variant_name = self.current().text.clone();
            self.advance();
            variants.push(crate::ast::EnumVariant {
                name: variant_name,
                fields: vec![],
            });
            if self.current().kind == TokenKind::Comma {
                self.advance();
            }
        }
        if self.current().kind != close {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                "Unterminated error set body",
            ));
        }
        self.advance();

        Ok(Stmt::ErrorSet {
            name,
            visibility,
            variants,
            span: Span::from_token(error_tok.line, error_tok.col, &error_tok.text),
        })
    }

    fn parse_historical_module_decl(&mut self) -> Result<Stmt, Diagnostic> {
        let module_tok = self.current().clone();
        self.advance();
        if self.current().kind != TokenKind::Ident {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                format!(
                    "Expected module path after 'module' at {}:{}",
                    self.current().line,
                    self.current().col
                ),
            ));
        }

        let mut path = self.current().text.clone();
        self.advance();
        while matches!(self.current().kind, TokenKind::Dot | TokenKind::ColonColon) {
            let separator = if self.current().kind == TokenKind::Dot {
                "."
            } else {
                "::"
            };
            self.advance();
            if self.current().kind != TokenKind::Ident {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    format!(
                        "Expected module path segment at {}:{}",
                        self.current().line,
                        self.current().col
                    ),
                ));
            }
            path.push_str(separator);
            path.push_str(&self.current().text);
            self.advance();
        }
        if self.current().kind == TokenKind::Semi {
            self.advance();
        }

        Ok(Stmt::Mod(
            path,
            Span::from_token(module_tok.line, module_tok.col, &module_tok.text),
        ))
    }

    fn parse_mod(&mut self) -> Result<Stmt, Diagnostic> {
        let mod_tok = self.current().clone();
        self.advance();

        let name_tok = self.current();
        if name_tok.kind != TokenKind::Ident {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                format!(
                    "Expected module name after 'mod' at {}:{}",
                    name_tok.line, name_tok.col
                ),
            ));
        }
        let name = name_tok.text.clone();
        self.advance();

        while self.current().kind == TokenKind::Newline
            || self.current().kind == TokenKind::LineComment
            || self.current().kind == TokenKind::BlockComment
            || self.current().kind == TokenKind::DocComment
        {
            self.advance();
        }

        if matches!(
            self.current().kind,
            TokenKind::LBracket | TokenKind::LBrace | TokenKind::Indent
        ) {
            let body = self.parse_statement_block("module")?;
            return Ok(Stmt::ModBlock(
                name,
                body,
                Span::from_token(mod_tok.line, mod_tok.col, &mod_tok.text),
            ));
        }

        Ok(Stmt::Mod(
            name,
            Span::from_token(mod_tok.line, mod_tok.col, &mod_tok.text),
        ))
    }

    fn parse_unsafe(&mut self) -> Result<Stmt, Diagnostic> {
        let unsafe_tok = self.current().clone();
        self.advance();
        self.skip_parser_trivia();
        let body = self.parse_statement_block("unsafe")?;
        Ok(Stmt::Unsafe {
            body,
            span: Span::from_token(unsafe_tok.line, unsafe_tok.col, &unsafe_tok.text),
        })
    }

    fn parse_expression_before_block(&mut self) -> Result<Expr, Diagnostic> {
        let previous = self.allow_trailing_struct_literal;
        self.allow_trailing_struct_literal = false;
        let result = self.parse_expression(Precedence::Lowest);
        self.allow_trailing_struct_literal = previous;
        result
    }

    fn parse_expression(&mut self, prec: Precedence) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_unary()?;
        while !self.at(&TokenKind::Eof)
            && !self.at(&TokenKind::Newline)
            && !self.at(&TokenKind::Indent)
            && !self.at(&TokenKind::Dedent)
            && !self.at(&TokenKind::RParen)
            && !self.at(&TokenKind::Comma)
        {
            // Do not treat '=' as an infix operator here; '=' is handled by
            // statement-level parsing (e.g. `let a = expr`). Stop parsing
            // the expression if we encounter an Equals token.
            if self.current().kind == TokenKind::Equals {
                break;
            }
            if !matches!(
                self.current().kind,
                TokenKind::OrOr
                    | TokenKind::AndAnd
                    | TokenKind::EqEq
                    | TokenKind::NotEq
                    | TokenKind::Lt
                    | TokenKind::LtEq
                    | TokenKind::Gt
                    | TokenKind::GtEq
                    | TokenKind::Plus
                    | TokenKind::Minus
                    | TokenKind::Star
                    | TokenKind::Slash
                    | TokenKind::Percent
                    | TokenKind::Dot
                    | TokenKind::DotDot
                    | TokenKind::DotDotDot
            ) {
                break;
            }
            let op_prec = Precedence::from_token(&self.current().kind);
            if op_prec < prec {
                break;
            }
            let op_tok = self.current().clone();
            let op = self.current().kind.clone();
            self.advance(); // consume operator token
            let right = self.parse_expression(op_prec)?;

            if op == TokenKind::Dot {
                left = Expr::FieldAccess {
                    base: Box::new(left),
                    field: match right {
                        Expr::Var(name, _) => name,
                        _ => {
                            return Err(Diagnostic::error(
                                error_codes::PARSER_INVALID_EXPRESSION,
                                format!(
                                    "Expected identifier after '.' at {}:{}",
                                    right.span().start_line,
                                    right.span().start_col
                                ),
                            ))
                        }
                    },
                    span: Span::from_token(op_tok.line, op_tok.col, &op_tok.text),
                };
            } else if op == TokenKind::DotDot || op == TokenKind::DotDotDot {
                left = Expr::Range {
                    start: Box::new(left),
                    end: Box::new(right),
                    inclusive: op == TokenKind::DotDotDot,
                    span: Span::from_token(op_tok.line, op_tok.col, &op_tok.text),
                };
            } else {
                left = Expr::BinaryOp {
                    op: op.clone(),
                    left: Box::new(left),
                    right: Box::new(right),
                    span: Span::from_token(op_tok.line, op_tok.col, &op_tok.text),
                };
            }
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, Diagnostic> {
        if self.current().kind == TokenKind::Amp {
            let op_tok = self.current().clone();
            self.advance();
            let mutable = if self.current().kind == TokenKind::Mut {
                self.advance();
                true
            } else {
                false
            };
            let inner = self.parse_unary()?;
            return Ok(Expr::Borrow {
                mutable,
                inner: Box::new(inner),
                span: Span::from_token(op_tok.line, op_tok.col, &op_tok.text),
            });
        }
        if self.current().kind == TokenKind::Star {
            let op_tok = self.current().clone();
            self.advance();
            let inner = self.parse_unary()?;
            return Ok(Expr::Deref {
                inner: Box::new(inner),
                span: Span::from_token(op_tok.line, op_tok.col, &op_tok.text),
            });
        }
        if self.current().kind == TokenKind::Bang || self.current().kind == TokenKind::Minus {
            let op_tok = self.current().clone();
            let op = self.current().kind.clone();
            self.advance();
            let inner = self.parse_unary()?;
            return Ok(Expr::UnaryOp {
                op,
                inner: Box::new(inner),
                span: Span::from_token(op_tok.line, op_tok.col, &op_tok.text),
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.current().kind == TokenKind::LParen {
                let lparen_tok = self.current().clone();
                self.advance();
                let mut args = Vec::new();
                while self.current().kind != TokenKind::RParen
                    && self.current().kind != TokenKind::Eof
                {
                    args.push(self.parse_expression(Precedence::Lowest)?);
                    if self.current().kind == TokenKind::Comma {
                        self.advance();
                    }
                }
                if self.current().kind == TokenKind::RParen {
                    self.advance();
                }
                let func_name = match expr {
                    Expr::Var(name, _) => name,
                    _ => String::new(),
                };
                expr = Expr::Call(
                    func_name,
                    args,
                    Span::from_token(lparen_tok.line, lparen_tok.col, &lparen_tok.text),
                );
            } else if self.current().kind == TokenKind::Dot {
                let dot_tok = self.current().clone();
                self.advance();
                let field_tok = self.current();
                if field_tok.kind == TokenKind::Ident {
                    let field = field_tok.text.clone();
                    self.advance();
                    expr = Expr::FieldAccess {
                        base: Box::new(expr),
                        field,
                        span: Span::from_token(dot_tok.line, dot_tok.col, &dot_tok.text),
                    };
                }
            } else if self.current().kind == TokenKind::LBracket {
                let bracket_tok = self.current().clone();
                self.advance();
                let index_expr = self.parse_expression(Precedence::Lowest)?;
                if self.current().kind == TokenKind::RBracket {
                    self.advance();
                }
                expr = Expr::Index(
                    Box::new(expr),
                    Box::new(index_expr),
                    Span::from_token(bracket_tok.line, bracket_tok.col, &bracket_tok.text),
                );
            } else if self.current().kind == TokenKind::Question {
                let question_tok = self.current().clone();
                self.advance();
                expr = Expr::Try(
                    Box::new(expr),
                    Span::from_token(question_tok.line, question_tok.col, &question_tok.text),
                );
            } else if self.current().kind == TokenKind::LBrace {
                if self.allow_trailing_struct_literal {
                    if let Expr::Var(name, _) = &expr {
                        return self.parse_struct_lit(name.clone());
                    }
                }
                break;
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, Diagnostic> {
        let tok = self.current().clone();
        match tok.kind {
            TokenKind::Ident => {
                let mut name = tok.text.clone();
                self.advance();
                while self.current().kind == TokenKind::ColonColon {
                    self.advance();
                    let segment = self.current().clone();
                    if segment.kind != TokenKind::Ident {
                        return Err(Diagnostic::error(
                            error_codes::PARSER_MISSING_TOKEN,
                            format!(
                                "Expected identifier after '::' at {}:{}",
                                segment.line, segment.col
                            ),
                        ));
                    }
                    name.push_str("::");
                    name.push_str(&segment.text);
                    self.advance();
                }
                Ok(Expr::Var(
                    name,
                    Span::from_token(tok.line, tok.col, &tok.text),
                ))
            }
            TokenKind::Number => {
                let normalized = tok.text.replace('_', "");
                let n = normalized.parse::<i64>().map_err(|e| {
                    Diagnostic::error(
                        error_codes::PARSER_INVALID_EXPRESSION,
                        format!("Invalid integer literal '{}': {}", tok.text, e),
                    )
                })?;
                self.advance();
                Ok(Expr::Number(
                    n,
                    Span::from_token(tok.line, tok.col, &tok.text),
                ))
            }
            TokenKind::HexNumber | TokenKind::OctNumber | TokenKind::BinNumber => {
                let normalized = tok.text.replace('_', "");
                let (digits, radix) =
                    if normalized.starts_with("0x") || normalized.starts_with("0X") {
                        (normalized.get(2..), 16)
                    } else if normalized.starts_with("0o") || normalized.starts_with("0O") {
                        (normalized.get(2..), 8)
                    } else if normalized.starts_with("0b") || normalized.starts_with("0B") {
                        (normalized.get(2..), 2)
                    } else {
                        (None, 10)
                    };
                let digits = digits.ok_or_else(|| {
                    Diagnostic::error(
                        error_codes::PARSER_INVALID_EXPRESSION,
                        format!("Malformed radix literal '{}'", tok.text),
                    )
                })?;
                let n = i64::from_str_radix(digits, radix).map_err(|e| {
                    Diagnostic::error(
                        error_codes::PARSER_INVALID_EXPRESSION,
                        format!("Invalid radix literal '{}': {}", tok.text, e),
                    )
                })?;
                self.advance();
                Ok(Expr::Number(
                    n,
                    Span::from_token(tok.line, tok.col, &tok.text),
                ))
            }
            TokenKind::Float => {
                let normalized = tok.text.replace('_', "");
                let n = normalized.parse::<f64>().map_err(|e| {
                    Diagnostic::error(
                        error_codes::PARSER_INVALID_EXPRESSION,
                        format!("Invalid floating-point literal '{}': {}", tok.text, e),
                    )
                })?;
                self.advance();
                Ok(Expr::Float(
                    n,
                    Span::from_token(tok.line, tok.col, &tok.text),
                ))
            }
            TokenKind::Char => {
                let mut chars = tok.text.chars();
                let ch = chars.next().ok_or_else(|| {
                    Diagnostic::error(
                        error_codes::PARSER_INVALID_EXPRESSION,
                        "Empty character literal",
                    )
                })?;
                if chars.next().is_some() {
                    return Err(Diagnostic::error(
                        error_codes::PARSER_INVALID_EXPRESSION,
                        "Character literal must contain exactly one Unicode scalar",
                    ));
                }
                self.advance();
                Ok(Expr::Char(
                    ch,
                    Span::from_token(tok.line, tok.col, &tok.text),
                ))
            }
            TokenKind::RawString => {
                let s = tok.text.clone();
                self.advance();
                Ok(Expr::StringLit(
                    s,
                    Span::from_token(tok.line, tok.col, &tok.text),
                ))
            }
            TokenKind::ByteString => {
                let bytes: Vec<u8> = tok
                    .text
                    .chars()
                    .map(|ch| {
                        u8::try_from(ch as u32)
                            .expect("lexer stores byte-string token text as U+0000..U+00FF")
                    })
                    .collect();
                self.advance();
                Ok(Expr::ByteString(
                    bytes,
                    Span::from_token(tok.line, tok.col, &tok.text),
                ))
            }
            TokenKind::ByteLiteral => {
                let value = tok.text.parse::<u8>().map_err(|_| {
                    Diagnostic::error(
                        error_codes::PARSER_INVALID_EXPRESSION,
                        "Invalid byte literal value",
                    )
                })?;
                self.advance();
                Ok(Expr::Byte(
                    value,
                    Span::from_token(tok.line, tok.col, &tok.text),
                ))
            }
            TokenKind::StringLiteral => {
                let s = tok.text.clone();
                self.advance();
                Ok(Expr::StringLit(
                    s,
                    Span::from_token(tok.line, tok.col, &tok.text),
                ))
            }
            TokenKind::InterpolatedString => {
                let s = tok.text.clone();
                self.advance();
                self.parse_interpolated_string(&s)
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Bool(
                    true,
                    Span::from_token(tok.line, tok.col, &tok.text),
                ))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Bool(
                    false,
                    Span::from_token(tok.line, tok.col, &tok.text),
                ))
            }
            TokenKind::LBracket => {
                self.advance();
                let mut items = Vec::new();
                while self.current().kind != TokenKind::RBracket
                    && self.current().kind != TokenKind::Eof
                {
                    items.push(self.parse_expression(Precedence::Lowest)?);
                    if self.current().kind == TokenKind::Comma {
                        self.advance();
                    } else if self.current().kind != TokenKind::RBracket {
                        return Err(Diagnostic::error(
                            error_codes::PARSER_MISSING_TOKEN,
                            format!(
                                "Expected ',' or ']' in array literal at {}:{}",
                                self.current().line,
                                self.current().col
                            ),
                        ));
                    }
                }
                if self.current().kind != TokenKind::RBracket {
                    return Err(Diagnostic::error(
                        error_codes::PARSER_MISSING_TOKEN,
                        format!(
                            "Expected ']' to close array literal at {}:{}",
                            self.current().line,
                            self.current().col
                        ),
                    ));
                }
                self.advance();
                Ok(Expr::Array(
                    items,
                    Span::from_token(tok.line, tok.col, &tok.text),
                ))
            }
            TokenKind::LParen => {
                self.advance();
                if self.current().kind == TokenKind::RParen {
                    self.advance();
                    return Ok(Expr::Tuple(
                        Vec::new(),
                        Span::from_token(tok.line, tok.col, &tok.text),
                    ));
                }

                let first = self.parse_expression(Precedence::Lowest)?;
                if self.current().kind == TokenKind::Comma {
                    let mut items = vec![first];
                    while self.current().kind == TokenKind::Comma {
                        self.advance();
                        if self.current().kind == TokenKind::RParen {
                            break;
                        }
                        items.push(self.parse_expression(Precedence::Lowest)?);
                    }
                    if self.current().kind != TokenKind::RParen {
                        return Err(Diagnostic::error(
                            error_codes::PARSER_MISSING_TOKEN,
                            format!(
                                "Expected ')' to close tuple at {}:{}",
                                self.current().line,
                                self.current().col
                            ),
                        ));
                    }
                    self.advance();
                    Ok(Expr::Tuple(
                        items,
                        Span::from_token(tok.line, tok.col, &tok.text),
                    ))
                } else {
                    if self.current().kind != TokenKind::RParen {
                        return Err(Diagnostic::error(
                            error_codes::PARSER_MISSING_TOKEN,
                            format!(
                                "Expected ')' to close parenthesized expression at {}:{}",
                                self.current().line,
                                self.current().col
                            ),
                        ));
                    }
                    self.advance();
                    Ok(first)
                }
            }
            TokenKind::Match => self.parse_match_expr(),
            TokenKind::Fn => self.parse_lambda_expr(),
            TokenKind::Await => {
                let await_tok = self.current().clone();
                self.advance();
                let inner = self.parse_expression(Precedence::Lowest)?;
                Ok(Expr::Await(
                    Box::new(inner),
                    Span::from_token(await_tok.line, await_tok.col, &await_tok.text),
                ))
            }
            TokenKind::LBrace => {
                self.advance();
                let mut stmts = Vec::new();
                while self.current().kind != TokenKind::RBrace
                    && self.current().kind != TokenKind::Eof
                {
                    while self.current().kind == TokenKind::Newline
                        || self.current().kind == TokenKind::LineComment
                        || self.current().kind == TokenKind::BlockComment
                        || self.current().kind == TokenKind::DocComment
                    {
                        self.advance();
                    }
                    if self.current().kind == TokenKind::RBrace
                        || self.current().kind == TokenKind::Eof
                    {
                        break;
                    }
                    stmts.push(self.parse_statement()?);
                }
                if self.current().kind != TokenKind::RBrace {
                    return Err(Diagnostic::error(
                        error_codes::PARSER_MISSING_TOKEN,
                        format!(
                            "Expected '}}' to close block expression at {}:{}",
                            self.current().line,
                            self.current().col
                        ),
                    ));
                }
                self.advance();
                Ok(Expr::Block(
                    stmts,
                    Span::from_token(tok.line, tok.col, &tok.text),
                ))
            }
            _ => Err(Diagnostic::error(
                error_codes::PARSER_UNEXPECTED_TOKEN,
                format!(
                    "Unexpected token {:?} at {}:{}",
                    tok.kind, tok.line, tok.col
                ),
            )),
        }
    }

    fn parse_lambda_expr(&mut self) -> Result<Expr, Diagnostic> {
        let fn_tok = self.current().clone();
        self.advance(); // consume 'fn'
        if self.current().kind != TokenKind::LParen {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                format!(
                    "Expected '(' in lambda at {}:{}",
                    self.current().line,
                    self.current().col
                ),
            ));
        }
        self.advance();
        let mut params = Vec::new();
        while self.current().kind != TokenKind::RParen && self.current().kind != TokenKind::Eof {
            if self.current().kind == TokenKind::Ident {
                let param_name = self.current().text.clone();
                self.advance();
                let param_type = if self.current().kind == TokenKind::Colon {
                    self.advance();
                    self.parse_type_name()
                } else {
                    None
                };
                params.push((param_name, param_type));
                if self.current().kind == TokenKind::Comma {
                    self.advance();
                }
            } else {
                return Err(Diagnostic::error(
                    error_codes::PARSER_UNEXPECTED_TOKEN,
                    format!(
                        "Expected lambda parameter at {}:{}",
                        self.current().line,
                        self.current().col
                    ),
                ));
            }
        }
        if self.current().kind != TokenKind::RParen {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                format!(
                    "Expected ')' to close lambda parameters at {}:{}",
                    self.current().line,
                    self.current().col
                ),
            ));
        }
        self.advance();
        let body = self.parse_expression(Precedence::Lowest)?;
        Ok(Expr::Lambda {
            params,
            body: Box::new(body),
            span: Span::from_token(fn_tok.line, fn_tok.col, &fn_tok.text),
        })
    }

    fn parse_struct_lit(&mut self, name: String) -> Result<Expr, Diagnostic> {
        let brace_tok = self.current().clone();
        self.advance(); // consume '{'
        let mut fields = Vec::new();
        while self.current().kind != TokenKind::RBrace && self.current().kind != TokenKind::Eof {
            while self.current().kind == TokenKind::Newline
                || self.current().kind == TokenKind::LineComment
                || self.current().kind == TokenKind::BlockComment
                || self.current().kind == TokenKind::DocComment
            {
                self.advance();
            }
            if self.current().kind == TokenKind::RBrace || self.current().kind == TokenKind::Eof {
                break;
            }
            let field_name_tok = self.current();
            if field_name_tok.kind != TokenKind::Ident {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    format!(
                        "Expected field name at {}:{}",
                        field_name_tok.line, field_name_tok.col
                    ),
                ));
            }
            let field_name = field_name_tok.text.clone();
            let field_sh_span = Span::from_token(
                field_name_tok.line,
                field_name_tok.col,
                &field_name_tok.text,
            );
            self.advance();
            if self.current().kind == TokenKind::Colon {
                self.advance();
                let value = self.parse_expression(Precedence::Lowest)?;
                fields.push((field_name, value));
            } else {
                fields.push((field_name.clone(), Expr::Var(field_name, field_sh_span)));
            }
            if self.current().kind == TokenKind::Comma {
                self.advance();
            }
        }
        if self.current().kind != TokenKind::RBrace {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                format!(
                    "Expected '}}' to close struct literal at {}:{}",
                    self.current().line,
                    self.current().col
                ),
            ));
        }
        self.advance();
        Ok(Expr::StructLit {
            name,
            fields,
            span: Span::from_token(brace_tok.line, brace_tok.col, &brace_tok.text),
        })
    }

    fn parse_match_expr(&mut self) -> Result<Expr, Diagnostic> {
        let match_tok = self.current().clone();
        self.advance();
        let expr = self.parse_expression_before_block()?;

        self.skip_parser_trivia();
        let close = match self.current().kind.clone() {
            TokenKind::LBracket => TokenKind::RBracket,
            TokenKind::LBrace => TokenKind::RBrace,
            TokenKind::Indent => TokenKind::Dedent,
            _ => {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    format!(
                        "Expected match body at {}:{}",
                        self.current().line,
                        self.current().col
                    ),
                ));
            }
        };
        self.advance();

        let mut arms = Vec::new();
        loop {
            self.skip_parser_trivia();
            if self.current().kind == close || self.current().kind == TokenKind::Eof {
                break;
            }

            let arm_start_tok = self.current().clone();
            if self.current().kind == TokenKind::Pipe {
                self.advance();
            }

            let pattern = self.parse_pattern()?;
            let guard = if self.current().kind == TokenKind::If
                || (self.current().kind == TokenKind::Ident && self.current().text == "if")
            {
                self.advance();
                Some(Box::new(self.parse_expression(Precedence::Lowest)?))
            } else {
                None
            };

            if self.current().kind != TokenKind::FatArrow {
                return Err(Diagnostic::error(
                    error_codes::PARSER_MISSING_TOKEN,
                    format!(
                        "Expected '=>' in match arm at {}:{}",
                        self.current().line,
                        self.current().col
                    ),
                ));
            }
            self.advance();

            let body = self.parse_expression(Precedence::Lowest)?;
            arms.push(crate::ast::MatchArm {
                pattern,
                guard,
                body: Box::new(body),
                span: Span::from_token(arm_start_tok.line, arm_start_tok.col, &arm_start_tok.text),
            });

            self.skip_parser_trivia();
            if self.current().kind == TokenKind::Comma {
                self.advance();
            }
        }

        if self.current().kind != close {
            return Err(Diagnostic::error(
                error_codes::PARSER_MISSING_TOKEN,
                "Unterminated match body",
            ));
        }
        self.advance();

        Ok(Expr::Match {
            expr: Box::new(expr),
            arms,
            span: Span::from_token(match_tok.line, match_tok.col, &match_tok.text),
        })
    }

    fn parse_pattern(&mut self) -> Result<crate::ast::Pattern, Diagnostic> {
        let tok = self.current().clone();
        match tok.kind {
            TokenKind::Number => {
                self.advance();
                let value = tok.text.parse::<i64>().map_err(|e| {
                    Diagnostic::error(
                        error_codes::PARSER_INVALID_EXPRESSION,
                        format!("Invalid pattern literal: {}", e),
                    )
                })?;
                Ok(crate::ast::Pattern::Literal(value))
            }
            TokenKind::Ident => {
                if tok.text == "_" {
                    self.advance();
                    return Ok(crate::ast::Pattern::Wildcard);
                }

                let mut name = tok.text.clone();
                self.advance();
                while self.current().kind == TokenKind::ColonColon {
                    self.advance();
                    let segment = self.current().clone();
                    if segment.kind != TokenKind::Ident {
                        return Err(Diagnostic::error(
                            error_codes::PARSER_MISSING_TOKEN,
                            format!(
                                "Expected pattern identifier after '::' at {}:{}",
                                segment.line, segment.col
                            ),
                        ));
                    }
                    name.push_str("::");
                    name.push_str(&segment.text);
                    self.advance();
                }
                if self.current().kind == TokenKind::LBracket {
                    self.advance();
                    let mut fields = Vec::new();
                    while self.current().kind != TokenKind::RBracket
                        && self.current().kind != TokenKind::Eof
                    {
                        while self.current().kind == TokenKind::Newline
                            || self.current().kind == TokenKind::LineComment
                            || self.current().kind == TokenKind::BlockComment
                            || self.current().kind == TokenKind::DocComment
                        {
                            self.advance();
                        }
                        if self.current().kind == TokenKind::RBracket
                            || self.current().kind == TokenKind::Eof
                        {
                            break;
                        }

                        let field_name_tok = self.current().clone();
                        if field_name_tok.kind != TokenKind::Ident {
                            self.advance();
                            continue;
                        }
                        let field_name = field_name_tok.text.clone();
                        self.advance();
                        if self.current().kind == TokenKind::Colon {
                            self.advance();
                            let nested = self.parse_pattern()?;
                            fields.push((field_name, nested));
                        } else {
                            fields.push((field_name.clone(), crate::ast::Pattern::Var(field_name)));
                        }
                        if self.current().kind == TokenKind::Comma {
                            self.advance();
                        }
                    }
                    if self.current().kind == TokenKind::RBracket {
                        self.advance();
                    }
                    Ok(crate::ast::Pattern::Struct(name, fields))
                } else {
                    Ok(crate::ast::Pattern::Var(name))
                }
            }
            _ => Err(Diagnostic::error(
                error_codes::PARSER_UNEXPECTED_TOKEN,
                format!(
                    "Unexpected token {:?} in pattern at {}:{}",
                    tok.kind, tok.line, tok.col
                ),
            )),
        }
    }

    fn parse_interpolated_string(&mut self, s: &str) -> Result<Expr, Diagnostic> {
        use crate::ast::InterpolatedFragment;

        fn find_matching_brace(text: &str, open: usize) -> Option<usize> {
            let bytes = text.as_bytes();
            let mut depth = 1usize;
            let mut i = open + 1;
            let mut quote: Option<u8> = None;
            let mut escaped = false;
            while i < bytes.len() {
                let b = bytes[i];
                if let Some(q) = quote {
                    if escaped {
                        escaped = false;
                    } else if b == b'\\' {
                        escaped = true;
                    } else if b == q {
                        quote = None;
                    }
                    i += 1;
                    continue;
                }
                match b {
                    b'"' | b'\'' => quote = Some(b),
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(i);
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            None
        }

        fn parse_fragment_expr(text: &str) -> Result<Expr, Diagnostic> {
            let toks = crate::complete_lexer::tokenize_complete(text).map_err(|e| {
                Diagnostic::error(
                    error_codes::PARSER_INVALID_EXPRESSION,
                    format!("Lexer error in interpolated fragment: {e}"),
                )
            })?;
            let mut parser = Parser::new(toks);
            let expr = parser.parse_expression(Precedence::Lowest)?;
            parser.skip_parser_trivia();
            if parser.current().kind != TokenKind::Eof {
                return Err(Diagnostic::error(
                    error_codes::PARSER_INVALID_EXPRESSION,
                    format!(
                        "Unexpected token {:?} after interpolated expression",
                        parser.current().kind
                    ),
                ));
            }
            Ok(expr)
        }

        let legacy_mode = !s.contains("${") && s.contains('{');
        let mut fragments = Vec::new();
        let mut literal = String::new();
        let bytes = s.as_bytes();
        let mut i = 0usize;

        while i < bytes.len() {
            // The lexer preserves `\$` only to distinguish an escaped literal
            // dollar from an interpolation marker.  Other escapes are already
            // decoded by the lexer.
            if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1] == b'$' {
                literal.push('$');
                i += 2;
                continue;
            }

            let marker = if i + 1 < bytes.len() && bytes[i] == b'$' && bytes[i + 1] == b'{' {
                Some((i + 1, 2usize))
            } else if legacy_mode && bytes[i] == b'{' {
                Some((i, 1usize))
            } else {
                None
            };

            if let Some((open, marker_len)) = marker {
                if !literal.is_empty() {
                    fragments.push(InterpolatedFragment::Literal(
                        std::mem::take(&mut literal),
                        Span::default(),
                    ));
                }
                let close = find_matching_brace(s, open).ok_or_else(|| {
                    Diagnostic::error(
                        error_codes::PARSER_MISSING_TOKEN,
                        "Unterminated interpolation expression; expected '}'",
                    )
                })?;
                let expr_text = &s[open + 1..close];
                if expr_text.trim().is_empty() {
                    return Err(Diagnostic::error(
                        error_codes::PARSER_INVALID_EXPRESSION,
                        "Interpolation expression cannot be empty",
                    ));
                }
                fragments.push(InterpolatedFragment::Expr(Box::new(parse_fragment_expr(
                    expr_text,
                )?)));
                i = close + 1;
                let _ = marker_len; // documents the accepted `${`/`{` compatibility markers
                continue;
            }

            let ch = s[i..].chars().next().ok_or_else(|| {
                Diagnostic::error(
                    error_codes::PARSER_INVALID_EXPRESSION,
                    "Invalid UTF-8 boundary",
                )
            })?;
            literal.push(ch);
            i += ch.len_utf8();
        }

        if !literal.is_empty() || fragments.is_empty() {
            fragments.push(InterpolatedFragment::Literal(literal, Span::default()));
        }

        Ok(Expr::Interpolated(fragments, Span::default()))
    }
}
