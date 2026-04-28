use crate::ast::{Expr, Program, Stmt};
use crate::lexer::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
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
        let mut parser = Parser { tokens, pos: 0 };
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
            && self.current().kind != TokenKind::RBracket
            && self.current().kind != TokenKind::Dedent
            && self.current().kind != TokenKind::Eof
        {
            match self.current().kind {
                TokenKind::Ident | TokenKind::Number => {
                    type_str.push_str(&self.current().text);
                    self.advance();
                }
                TokenKind::LBracket | TokenKind::LParen => {
                    type_str.push_str(&self.current().text);
                    self.advance();
                }
                TokenKind::RBracket | TokenKind::RParen => {
                    type_str.push_str(&self.current().text);
                    self.advance();
                }
                TokenKind::Comma | TokenKind::Colon => {
                    type_str.push_str(&self.current().text);
                    self.advance();
                }
                _ => {
                    self.advance();
                }
            }
        }
        type_str.trim().to_string()
    }

    fn recover_from_error(&mut self) {
        while self.current().kind != TokenKind::Newline && self.current().kind != TokenKind::Eof {
            self.advance();
        }
        // Skip any comment tokens after newline
        while self.current().kind == TokenKind::Newline
            || self.current().kind == TokenKind::LineComment
            || self.current().kind == TokenKind::BlockComment
            || self.current().kind == TokenKind::DocComment
        {
            self.advance();
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, String> {
        let mut prog = Program::new();
        let mut errors: Vec<String> = Vec::new();
        while self.current().kind != TokenKind::Eof {
            // skip blank lines and standalone comments
            while self.current().kind == TokenKind::Newline
                || self.current().kind == TokenKind::LineComment
                || self.current().kind == TokenKind::BlockComment
                || self.current().kind == TokenKind::DocComment
            {
                self.advance();
            }
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
            Err(errors.join("\n"))
        }
    }

    pub fn parse_statement(&mut self) -> Result<Stmt, String> {
        // Skip any leading newlines or comments
        while self.current().kind == TokenKind::Newline
            || self.current().kind == TokenKind::LineComment
            || self.current().kind == TokenKind::BlockComment
            || self.current().kind == TokenKind::DocComment
        {
            self.advance();
        }

        let mut fn_prefix_effects: Vec<String> = Vec::new();
        let mut is_public = false;
        loop {
            if self.current().kind == TokenKind::Ident && self.current().text == "pub" {
                is_public = true;
                self.advance();
                continue;
            }
            if self.current().kind == TokenKind::Ident && self.current().text == "async" {
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
            self.advance();
            let mut stmts: Vec<Stmt> = Vec::new();
            while self.current().kind != TokenKind::Dedent && self.current().kind != TokenKind::Eof
            {
                // Skip blank lines and comments
                while self.current().kind == TokenKind::Newline
                    || self.current().kind == TokenKind::LineComment
                    || self.current().kind == TokenKind::BlockComment
                    || self.current().kind == TokenKind::DocComment
                {
                    self.advance();
                }
                if self.current().kind == TokenKind::Dedent || self.current().kind == TokenKind::Eof
                {
                    break;
                }
                match self.parse_statement() {
                    Ok(s) => stmts.push(s),
                    Err(e) => return Err(e),
                }
            }
            if self.current().kind == TokenKind::Dedent {
                self.advance();
            }
            return Ok(Stmt::Block(stmts));
        }

        let tok = self.current();
        // Accept explicit `If` token from the lexer as a keyword.
        if tok.kind == TokenKind::If {
            return self.parse_if();
        }

        if tok.kind == TokenKind::Enum {
            return self.parse_enum();
        }

        if tok.kind == TokenKind::Ident && tok.text == "error" {
            let next_tok = self.peek();
            if next_tok.kind != TokenKind::Colon {
                return self.parse_error_set();
            }
        }

        if tok.kind == TokenKind::Ident {
            if tok.text == "print" {
                self.advance();
                let expr = self.parse_expression(Precedence::Lowest)?;
                return Ok(Stmt::Print(expr));
            } else if tok.text == "let" {
                self.advance();
                let name_tok = self.current();
                if name_tok.kind != TokenKind::Ident {
                    return Err(format!(
                        "Expected identifier after 'let' at {}:{}",
                        name_tok.line, name_tok.col
                    ));
                }
                let name = name_tok.text.clone();
                self.advance();
                if self.current().kind != TokenKind::Equals {
                    return Err(format!(
                        "Expected '=' after identifier at {}:{}",
                        self.current().line,
                        self.current().col
                    ));
                }
                self.advance();
                let expr = self.parse_expression(Precedence::Lowest)?;
                return Ok(Stmt::Let(name, expr));
            } else if tok.text == "fn" {
                return self.parse_function(is_public, fn_prefix_effects);
            } else if tok.text == "if" {
                return self.parse_if();
            } else if tok.text == "loop" {
                return self.parse_loop();
            } else if tok.text == "return" {
                self.advance();
                let expr = self.parse_expression(Precedence::Lowest)?;
                return Ok(Stmt::Return(expr));
            } else if tok.text == "for" {
                return self.parse_for();
            } else if tok.text == "while" {
                return self.parse_while();
            } else if tok.text == "struct" {
                return self.parse_struct();
            } else if tok.text == "impl" {
                return self.parse_impl();
            } else if tok.text == "trait" {
                return self.parse_trait();
            } else if tok.text == "type" {
                return self.parse_type_alias();
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
                return Ok(Stmt::Break);
            } else if tok.text == "continue" {
                self.advance();
                return Ok(Stmt::Continue);
            }
        }

        if tok.kind == TokenKind::Linear {
            self.advance();
            if self.current().kind != TokenKind::Ident {
                return Err(format!(
                    "Expected identifier after 'linear' at {}:{}",
                    self.current().line,
                    self.current().col
                ));
            }
            let name_tok = self.current();
            let name = name_tok.text.clone();
            self.advance();
            if self.current().kind != TokenKind::Equals {
                return Err(format!(
                    "Expected '=' after identifier at {}:{}",
                    self.current().line,
                    self.current().col
                ));
            }
            self.advance();
            let expr = self.parse_expression(Precedence::Lowest)?;
            return Ok(Stmt::LetLinear(name, expr));
        }

        if tok.kind == TokenKind::Unsafe {
            return self.parse_unsafe();
        }

        let expr = self.parse_expression(Precedence::Lowest)?;
        Ok(Stmt::ExprStmt(expr))
    }

    fn parse_function(
        &mut self,
        is_public: bool,
        mut effects: Vec<String>,
    ) -> Result<Stmt, String> {
        self.advance(); // consume 'fn'
        let name_tok = self.current();
        if name_tok.kind != TokenKind::Ident {
            return Err(format!(
                "Expected function name at {}:{}",
                name_tok.line, name_tok.col
            ));
        }
        let name = name_tok.text.clone();
        self.advance();

        let mut type_params = Vec::new();
        if self.current().kind == TokenKind::Lt {
            self.advance();
            while self.current().kind != TokenKind::Gt && self.current().kind != TokenKind::Eof {
                if self.current().kind == TokenKind::Ident {
                    type_params.push(self.current().text.clone());
                    self.advance();
                    if self.current().kind == TokenKind::Comma {
                        self.advance();
                    }
                } else {
                    self.advance();
                }
            }
            if self.current().kind == TokenKind::Gt {
                self.advance();
            }
        }

        if self.current().kind != TokenKind::LParen {
            return Err(format!(
                "Expected '(' after function name at {}:{}",
                self.current().line,
                self.current().col
            ));
        }
        self.advance();

        let mut params = Vec::new();
        while self.current().kind != TokenKind::RParen && self.current().kind != TokenKind::Eof {
            if self.current().kind == TokenKind::Ident {
                params.push(self.current().text.clone());
                self.advance();
                if self.current().kind == TokenKind::Comma {
                    self.advance();
                }
            } else {
                // Skip non-identifier tokens (like comments)
                self.advance();
            }
        }
        if self.current().kind == TokenKind::RParen {
            self.advance();
        }

        // optional return type: '-> Type'
        let mut ret_type: Option<String> = None;
        if self.current().kind == TokenKind::Arrow {
            self.advance();
            if self.current().kind == TokenKind::Ident {
                ret_type = Some(self.current().text.clone());
                self.advance();
            }
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
        // Allow either an indented/braced block OR a single-line inline `return` after the signature.
        if self.current().kind == TokenKind::LBracket || self.current().kind == TokenKind::Indent {
            self.advance();
            while self.current().kind != TokenKind::RBracket
                && self.current().kind != TokenKind::Dedent
                && self.current().kind != TokenKind::Eof
            {
                // Skip blank lines and comments
                while self.current().kind == TokenKind::Newline
                    || self.current().kind == TokenKind::LineComment
                    || self.current().kind == TokenKind::BlockComment
                    || self.current().kind == TokenKind::DocComment
                {
                    self.advance();
                }
                if self.current().kind == TokenKind::RBracket
                    || self.current().kind == TokenKind::Dedent
                    || self.current().kind == TokenKind::Eof
                {
                    break;
                }
                match self.parse_statement() {
                    Ok(s) => body.push(s),
                    Err(e) => return Err(e),
                }
            }
            if self.current().kind == TokenKind::RBracket
                || self.current().kind == TokenKind::Dedent
            {
                self.advance();
            }
        } else if self.current().kind == TokenKind::Ident && self.current().text == "return" {
            // parse a single-line return as the function body
            match self.parse_statement() {
                Ok(s) => body.push(s),
                Err(e) => return Err(e),
            }
        }

        Ok(Stmt::Fn {
            name,
            is_public,
            is_async: false,
            type_params,
            params,
            ret_type,
            effects,
            body,
        })
    }

    fn parse_if(&mut self) -> Result<Stmt, String> {
        self.advance();
        let cond = self.parse_expression(Precedence::Lowest)?;

        let mut bindings = Vec::new();
        let final_cond = cond;

        let mut then_body = Vec::new();
        while self.current().kind == TokenKind::Newline
            || self.current().kind == TokenKind::LineComment
            || self.current().kind == TokenKind::BlockComment
            || self.current().kind == TokenKind::DocComment
        {
            self.advance();
        }
        if self.current().kind == TokenKind::Then {
            self.advance();
            while self.current().kind == TokenKind::Newline
                || self.current().kind == TokenKind::LineComment
                || self.current().kind == TokenKind::BlockComment
                || self.current().kind == TokenKind::DocComment
            {
                self.advance();
            }
        }
        if self.current().kind == TokenKind::LBracket || self.current().kind == TokenKind::Indent {
            self.advance();
            while self.current().kind != TokenKind::RBracket
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
                    || self.current().kind == TokenKind::Dedent
                    || self.current().kind == TokenKind::Eof
                {
                    break;
                }
                match self.parse_statement() {
                    Ok(s) => then_body.push(s),
                    Err(e) => return Err(e),
                }
            }
            if self.current().kind == TokenKind::RBracket
                || self.current().kind == TokenKind::Dedent
            {
                self.advance();
            }
        }

        let mut else_body = Vec::new();
        while self.current().kind == TokenKind::Newline
            || self.current().kind == TokenKind::LineComment
            || self.current().kind == TokenKind::BlockComment
            || self.current().kind == TokenKind::DocComment
        {
            self.advance();
        }
        if self.current().kind == TokenKind::Else
            || (self.current().kind == TokenKind::Ident && self.current().text == "else")
        {
            self.advance();
            while self.current().kind == TokenKind::Newline
                || self.current().kind == TokenKind::LineComment
                || self.current().kind == TokenKind::BlockComment
                || self.current().kind == TokenKind::DocComment
            {
                self.advance();
            }
            if self.current().kind == TokenKind::Then {
                self.advance();
            }
            if self.current().kind == TokenKind::If
                || (self.current().kind == TokenKind::Ident && self.current().text == "if")
            {
                let nested_if = self.parse_if()?;
                else_body.push(nested_if);
            } else if self.current().kind == TokenKind::LBracket
                || self.current().kind == TokenKind::Indent
            {
                self.advance();
                while self.current().kind != TokenKind::RBracket
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
                        || self.current().kind == TokenKind::Dedent
                        || self.current().kind == TokenKind::Eof
                    {
                        break;
                    }
                    match self.parse_statement() {
                        Ok(s) => else_body.push(s),
                        Err(e) => return Err(e),
                    }
                }
                if self.current().kind == TokenKind::RBracket
                    || self.current().kind == TokenKind::Dedent
                {
                    self.advance();
                }
            }
        }

        Ok(Stmt::If {
            cond: Box::new(final_cond),
            bindings,
            then_body,
            else_body,
        })
    }

    fn parse_loop(&mut self) -> Result<Stmt, String> {
        self.advance();
        let mut body = Vec::new();
        while self.current().kind == TokenKind::Newline
            || self.current().kind == TokenKind::LineComment
            || self.current().kind == TokenKind::BlockComment
            || self.current().kind == TokenKind::DocComment
        {
            self.advance();
        }
        if self.current().kind == TokenKind::LBracket || self.current().kind == TokenKind::Indent {
            self.advance();
            while self.current().kind != TokenKind::RBracket
                && self.current().kind != TokenKind::Dedent
                && self.current().kind != TokenKind::Eof
            {
                if self.current().kind == TokenKind::Newline
                    || self.current().kind == TokenKind::LineComment
                    || self.current().kind == TokenKind::BlockComment
                    || self.current().kind == TokenKind::DocComment
                {
                    self.advance();
                    continue;
                }
                if self.current().kind == TokenKind::RBracket
                    || self.current().kind == TokenKind::Dedent
                    || self.current().kind == TokenKind::Eof
                {
                    break;
                }
                match self.parse_statement() {
                    Ok(s) => body.push(s),
                    Err(e) => return Err(e),
                }
            }
            if self.current().kind == TokenKind::RBracket
                || self.current().kind == TokenKind::Dedent
            {
                self.advance();
            }
        }

        Ok(Stmt::Loop { body })
    }

    fn parse_for(&mut self) -> Result<Stmt, String> {
        self.advance();
        let var_tok = self.current();
        if var_tok.kind != TokenKind::Ident {
            return Err(format!(
                "Expected variable name in for at {}:{}",
                var_tok.line, var_tok.col
            ));
        }
        let var_name = var_tok.text.clone();
        self.advance();

        if self.current().kind != TokenKind::Ident || self.current().text != "in" {
            return Err(format!(
                "Expected 'in' in for at {}:{}",
                self.current().line,
                self.current().col
            ));
        }
        self.advance();

        let iterable = self.parse_expression(Precedence::Lowest)?;

        let mut body = Vec::new();
        while self.current().kind == TokenKind::Newline {
            self.advance();
        }
        if self.current().kind == TokenKind::LBracket || self.current().kind == TokenKind::Indent {
            self.advance();
            while self.current().kind != TokenKind::RBracket
                && self.current().kind != TokenKind::Dedent
                && self.current().kind != TokenKind::Eof
            {
                match self.parse_statement() {
                    Ok(s) => body.push(s),
                    Err(e) => return Err(e),
                }
            }
            if self.current().kind == TokenKind::RBracket
                || self.current().kind == TokenKind::Dedent
            {
                self.advance();
            }
        }

        Ok(Stmt::For {
            var_name,
            iterable: Box::new(iterable),
            body,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, String> {
        self.advance();
        let cond = self.parse_expression(Precedence::Lowest)?;

        let mut body = Vec::new();
        while self.current().kind == TokenKind::Newline {
            self.advance();
        }
        if self.current().kind == TokenKind::LBracket || self.current().kind == TokenKind::Indent {
            self.advance();
            while self.current().kind != TokenKind::RBracket
                && self.current().kind != TokenKind::Dedent
                && self.current().kind != TokenKind::Eof
            {
                match self.parse_statement() {
                    Ok(s) => body.push(s),
                    Err(e) => return Err(e),
                }
            }
            if self.current().kind == TokenKind::RBracket
                || self.current().kind == TokenKind::Dedent
            {
                self.advance();
            }
        }

        Ok(Stmt::While {
            cond: Box::new(cond),
            body,
        })
    }

    fn parse_struct(&mut self) -> Result<Stmt, String> {
        self.advance();
        let name_tok = self.current();
        if name_tok.kind != TokenKind::Ident {
            return Err(format!(
                "Expected struct name at {}:{}",
                name_tok.line, name_tok.col
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
        if self.current().kind == TokenKind::Colon {
            self.advance();
            while self.current().kind == TokenKind::Newline {
                self.advance();
            }
            while self.current().kind != TokenKind::Dedent && self.current().kind != TokenKind::Eof
            {
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
                } else if self.current().kind == TokenKind::Indent {
                    self.advance();
                } else if self.current().kind == TokenKind::Dedent {
                    self.advance();
                    break;
                } else {
                    self.advance();
                }
            }
        } else if self.current().kind == TokenKind::LBracket {
            self.advance();
            while self.current().kind != TokenKind::RBracket
                && self.current().kind != TokenKind::Eof
            {
                let field_name_tok = self.current();
                if field_name_tok.kind == TokenKind::Ident {
                    let field_name = field_name_tok.text.clone();
                    self.advance();
                    if self.current().kind == TokenKind::Colon {
                        self.advance();
                        let field_type = self.parse_type_expr();
                        fields.push((field_name, field_type));
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

        Ok(Stmt::Struct {
            name,
            fields,
            is_linear,
        })
    }

    fn parse_impl(&mut self) -> Result<Stmt, String> {
        self.advance();
        let target_tok = self.current();
        if target_tok.kind != TokenKind::Ident {
            return Err(format!(
                "Expected impl target name at {}:{}",
                target_tok.line, target_tok.col
            ));
        }
        let target = target_tok.text.clone();
        self.advance();

        let mut type_params = Vec::new();
        if self.current().kind == TokenKind::LBracket {
            self.advance();
            while self.current().kind != TokenKind::RBracket
                && self.current().kind != TokenKind::Eof
            {
                if self.current().kind == TokenKind::Ident {
                    type_params.push(self.current().text.clone());
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

        while self.current().kind == TokenKind::Newline
            || self.current().kind == TokenKind::LineComment
            || self.current().kind == TokenKind::BlockComment
            || self.current().kind == TokenKind::DocComment
        {
            self.advance();
        }

        if self.current().kind == TokenKind::Colon {
            self.advance();
            while self.current().kind == TokenKind::Newline
                || self.current().kind == TokenKind::LineComment
                || self.current().kind == TokenKind::BlockComment
                || self.current().kind == TokenKind::DocComment
            {
                self.advance();
            }
        }

        let mut methods = Vec::new();
        while self.current().kind != TokenKind::Dedent && self.current().kind != TokenKind::Eof {
            if self.current().kind == TokenKind::Newline {
                self.advance();
                continue;
            }
            if self.current().kind == TokenKind::Ident && self.current().text == "pub" {
                self.advance();
            }
            if self.current().kind == TokenKind::Ident && self.current().text == "fn" {
                let method = self.parse_function(false, Vec::new())?;
                methods.push(method);
            } else {
                self.advance();
            }
        }

        Ok(Stmt::Impl {
            target,
            type_params,
            methods,
        })
    }

    fn parse_trait(&mut self) -> Result<Stmt, String> {
        self.advance();
        let name_tok = self.current();
        if name_tok.kind != TokenKind::Ident {
            return Err(format!(
                "Expected trait name at {}:{}",
                name_tok.line, name_tok.col
            ));
        }
        let name = name_tok.text.clone();
        self.advance();

        let mut type_params = Vec::new();
        if self.current().kind == TokenKind::LBracket {
            self.advance();
            while self.current().kind != TokenKind::RBracket
                && self.current().kind != TokenKind::Eof
            {
                if self.current().kind == TokenKind::Ident {
                    type_params.push(self.current().text.clone());
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

        while self.current().kind == TokenKind::Newline
            || self.current().kind == TokenKind::LineComment
            || self.current().kind == TokenKind::BlockComment
            || self.current().kind == TokenKind::DocComment
        {
            self.advance();
        }

        if self.current().kind == TokenKind::Colon {
            self.advance();
            while self.current().kind == TokenKind::Newline
                || self.current().kind == TokenKind::LineComment
                || self.current().kind == TokenKind::BlockComment
                || self.current().kind == TokenKind::DocComment
            {
                self.advance();
            }
        }

        let mut methods = Vec::new();
        while self.current().kind != TokenKind::Dedent && self.current().kind != TokenKind::Eof {
            if self.current().kind == TokenKind::Newline {
                self.advance();
                continue;
            }
            if self.current().kind == TokenKind::Ident && self.current().text == "pub" {
                self.advance();
            }
            if self.current().kind == TokenKind::Ident && self.current().text == "fn" {
                let method = self.parse_function(false, Vec::new())?;
                methods.push(method);
            } else {
                self.advance();
            }
        }

        Ok(Stmt::Trait {
            name,
            type_params,
            methods,
        })
    }

    fn parse_type_alias(&mut self) -> Result<Stmt, String> {
        self.advance();
        let name_tok = self.current();
        if name_tok.kind != TokenKind::Ident {
            return Err(format!(
                "Expected type alias name at {}:{}",
                name_tok.line, name_tok.col
            ));
        }
        let name = name_tok.text.clone();
        self.advance();

        let mut type_params = Vec::new();
        if self.current().kind == TokenKind::LBracket {
            self.advance();
            while self.current().kind != TokenKind::RBracket
                && self.current().kind != TokenKind::Eof
            {
                if self.current().kind == TokenKind::Ident {
                    type_params.push(self.current().text.clone());
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
            type_params,
            target,
        })
    }

    fn parse_use(&mut self) -> Result<Stmt, String> {
        self.advance();
        let path_tok = self.current();
        if path_tok.kind != TokenKind::Ident {
            return Err(format!(
                "Expected module path after 'use' at {}:{}",
                path_tok.line, path_tok.col
            ));
        }
        let path = path_tok.text.clone();
        self.advance();

        let mut alias = None;
        while self.current().kind == TokenKind::Newline
            || self.current().kind == TokenKind::LineComment
            || self.current().kind == TokenKind::BlockComment
            || self.current().kind == TokenKind::DocComment
        {
            self.advance();
        }

        if self.current().kind == TokenKind::Ident && self.current().text == "as" {
            self.advance();
            if self.current().kind == TokenKind::Ident {
                alias = Some(self.current().text.clone());
                self.advance();
            }
        }

        Ok(Stmt::Use { path, alias })
    }

    fn parse_gc_mode(&mut self) -> Result<Stmt, String> {
        self.advance();
        let mode_tok = self.current();
        let mode = if mode_tok.kind == TokenKind::Ident {
            mode_tok.text.clone()
        } else {
            "stop".to_string()
        };
        if mode_tok.kind != TokenKind::Eof {
            self.advance();
        }
        Ok(Stmt::GcMode { mode })
    }

    fn parse_effect_handler(&mut self) -> Result<Stmt, String> {
        self.advance();
        let effect_tok = self.current();
        let effect = if effect_tok.kind == TokenKind::Ident {
            effect_tok.text.clone()
        } else {
            "Error".to_string()
        };
        if effect_tok.kind != TokenKind::Eof {
            self.advance();
        }
        while self.current().kind == TokenKind::Newline {
            self.advance();
        }
        if self.current().kind == TokenKind::LBracket || self.current().kind == TokenKind::Indent {
            self.advance();
            while self.current().kind != TokenKind::RBracket
                && self.current().kind != TokenKind::Dedent
                && self.current().kind != TokenKind::Eof
            {
                self.advance();
            }
            if self.current().kind == TokenKind::RBracket
                || self.current().kind == TokenKind::Dedent
            {
                self.advance();
            }
        }
        Ok(Stmt::EffectHandler {
            effect,
            handler: Box::new(Expr::Var("noop".to_string())),
        })
    }

    fn parse_spawn(&mut self) -> Result<Stmt, String> {
        self.advance();
        let task = self.parse_expression(Precedence::Lowest)?;
        Ok(Stmt::Spawn {
            task: Box::new(task),
        })
    }

    fn parse_channel(&mut self) -> Result<Stmt, String> {
        self.advance();
        let elem_type = if self.current().kind == TokenKind::Ident {
            let t = self.current().text.clone();
            self.advance();
            t
        } else {
            "Dyn".to_string()
        };
        let mut capacity = None;
        if self.current().kind == TokenKind::LBracket {
            self.advance();
            if self.current().kind == TokenKind::Number {
                capacity = self.current().text.parse().ok();
                self.advance();
            }
            if self.current().kind == TokenKind::RBracket {
                self.advance();
            }
        }
        Ok(Stmt::Channel {
            elem_type,
            capacity,
        })
    }

    fn parse_actor(&mut self) -> Result<Stmt, String> {
        self.advance();
        let name_tok = self.current();
        let name = name_tok.text.clone();
        if name_tok.kind == TokenKind::Ident {
            self.advance();
        }
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
        })
    }

    fn parse_work_stealing(&mut self) -> Result<Stmt, String> {
        self.advance();
        let num_threads = if self.current().kind == TokenKind::LBracket {
            self.advance();
            let n = if self.current().kind == TokenKind::Number {
                self.current().text.parse().unwrap_or(4)
            } else {
                4
            };
            if self.current().kind == TokenKind::RBracket {
                self.advance();
            }
            n
        } else {
            4
        };
        let queue_type = "mpsc".to_string();
        Ok(Stmt::WorkStealingExecutor {
            num_threads,
            queue_type,
        })
    }

    fn parse_deterministic_runtime(&mut self) -> Result<Stmt, String> {
        self.advance();
        let max_tasks = if self.current().kind == TokenKind::LBracket {
            self.advance();
            let n = if self.current().kind == TokenKind::Number {
                self.current().text.parse().unwrap_or(1000)
            } else {
                1000
            };
            if self.current().kind == TokenKind::RBracket {
                self.advance();
            }
            n
        } else {
            1000
        };
        Ok(Stmt::DeterministicRuntime { max_tasks })
    }

    fn parse_tensor(&mut self) -> Result<Stmt, String> {
        self.advance();
        let mut shape = Vec::new();
        if self.current().kind == TokenKind::LBracket {
            self.advance();
            while self.current().kind != TokenKind::RBracket
                && self.current().kind != TokenKind::Eof
            {
                if self.current().kind == TokenKind::Number {
                    if let Ok(n) = self.current().text.parse::<u32>() {
                        shape.push(n);
                    }
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
        let dtype = if self.current().kind == TokenKind::Ident {
            let t = self.current().text.clone();
            self.advance();
            t
        } else {
            "f32".to_string()
        };
        Ok(Stmt::Tensor { shape, dtype })
    }

    fn parse_simd(&mut self) -> Result<Stmt, String> {
        self.advance();
        let width = if self.current().kind == TokenKind::LBracket {
            self.advance();
            let n = if self.current().kind == TokenKind::Number {
                self.current().text.parse().unwrap_or(4)
            } else {
                4
            };
            if self.current().kind == TokenKind::RBracket {
                self.advance();
            }
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
        Ok(Stmt::Simd { width, elem_type })
    }

    fn parse_doc_comment(&mut self) -> Result<Stmt, String> {
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
        Ok(Stmt::DocComment { target, content })
    }

    fn parse_debug_session(&mut self) -> Result<Stmt, String> {
        self.advance();
        let port = if self.current().kind == TokenKind::LBracket {
            self.advance();
            let p = if self.current().kind == TokenKind::Number {
                self.current().text.parse().unwrap_or(4711)
            } else {
                4711
            };
            if self.current().kind == TokenKind::RBracket {
                self.advance();
            }
            p
        } else {
            4711
        };
        Ok(Stmt::DebugSession {
            port,
            breakpoints: Vec::new(),
        })
    }

    fn parse_capability(&mut self) -> Result<Stmt, String> {
        self.advance();
        let name = if self.current().kind == TokenKind::Ident {
            let n = self.current().text.clone();
            self.advance();
            n
        } else {
            "default".to_string()
        };
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
        Ok(Stmt::Capability { name, permissions })
    }

    fn parse_ffi_sandbox(&mut self) -> Result<Stmt, String> {
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
        Ok(Stmt::FfiSandbox { allow_list })
    }

    fn parse_enum(&mut self) -> Result<Stmt, String> {
        self.advance();
        let name_tok = self.current();
        if name_tok.kind != TokenKind::Ident {
            return Err(format!(
                "Expected enum name at {}:{}",
                name_tok.line, name_tok.col
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
        if self.current().kind == TokenKind::LBracket || self.current().kind == TokenKind::Indent {
            self.advance();
            while self.current().kind != TokenKind::RBracket
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
                    return Err(format!(
                        "Expected enum variant name at {}:{}",
                        variant_name_tok.line, variant_name_tok.col
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
                || self.current().kind == TokenKind::Dedent
            {
                self.advance();
            }
        }

        Ok(Stmt::Enum {
            name,
            variants,
            is_sealed,
        })
    }

    fn parse_error_set(&mut self) -> Result<Stmt, String> {
        self.advance();
        let name_tok = self.current();
        if name_tok.kind != TokenKind::Ident {
            return Err(format!(
                "Expected error set name at {}:{}",
                name_tok.line, name_tok.col
            ));
        }
        let name = name_tok.text.clone();
        self.advance();

        let mut variants = Vec::new();
        if self.current().kind == TokenKind::LBracket {
            self.advance();
            while self.current().kind != TokenKind::RBracket
                && self.current().kind != TokenKind::Eof
            {
                if self.current().kind == TokenKind::Ident {
                    let variant_name = self.current().text.clone();
                    self.advance();
                    variants.push(crate::ast::EnumVariant {
                        name: variant_name,
                        fields: vec![],
                    });
                }
                if self.current().kind == TokenKind::Comma {
                    self.advance();
                }
            }
            if self.current().kind == TokenKind::RBracket {
                self.advance();
            }
        }

        Ok(Stmt::ErrorSet { name, variants })
    }

    fn parse_unsafe(&mut self) -> Result<Stmt, String> {
        self.advance();
        let mut body = Vec::new();
        while self.current().kind == TokenKind::Newline {
            self.advance();
        }
        if self.current().kind == TokenKind::LBracket || self.current().kind == TokenKind::Indent {
            self.advance();
            while self.current().kind != TokenKind::RBracket
                && self.current().kind != TokenKind::Dedent
                && self.current().kind != TokenKind::Eof
            {
                match self.parse_statement() {
                    Ok(s) => body.push(s),
                    Err(e) => return Err(e),
                }
            }
            if self.current().kind == TokenKind::RBracket
                || self.current().kind == TokenKind::Dedent
            {
                self.advance();
            }
        }
        Ok(Stmt::Unsafe { body })
    }

    fn parse_expression(&mut self, prec: Precedence) -> Result<Expr, String> {
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
                    | TokenKind::DotDot
                    | TokenKind::DotDotDot
            ) {
                break;
            }
            let op_prec = Precedence::from_token(&self.current().kind);
            if op_prec < prec {
                break;
            }
            let op = self.current().kind.clone();
            self.advance();
            let right = self.parse_expression(op_prec)?;

            if op == TokenKind::DotDot || op == TokenKind::DotDotDot {
                left = Expr::Range {
                    start: Box::new(left),
                    end: Box::new(right),
                    inclusive: op == TokenKind::DotDotDot,
                };
            } else {
                left = Expr::BinaryOp {
                    op: op.clone(),
                    left: Box::new(left),
                    right: Box::new(right),
                };
            }
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if self.current().kind == TokenKind::Bang || self.current().kind == TokenKind::Minus {
            let op = self.current().kind.clone();
            self.advance();
            let inner = self.parse_unary()?;
            return Ok(Expr::UnaryOp {
                op,
                inner: Box::new(inner),
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.current().kind == TokenKind::LParen {
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
                    Expr::Var(name) => name,
                    _ => String::new(),
                };
                expr = Expr::Call(func_name, args);
            } else if self.current().kind == TokenKind::Dot {
                self.advance();
                let field_tok = self.current();
                if field_tok.kind == TokenKind::Ident {
                    let field = field_tok.text.clone();
                    self.advance();
                    expr = Expr::FieldAccess {
                        base: Box::new(expr),
                        field,
                    };
                }
            } else if self.current().kind == TokenKind::LBracket {
                self.advance();
                let index_expr = self.parse_expression(Precedence::Lowest)?;
                if self.current().kind == TokenKind::RBracket {
                    self.advance();
                }
                expr = Expr::Index(Box::new(expr), Box::new(index_expr));
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        let tok = self.current();
        match tok.kind {
            TokenKind::Ident => {
                let name = tok.text.clone();
                self.advance();
                Ok(Expr::Var(name))
            }
            TokenKind::Number => {
                let n = tok
                    .text
                    .parse::<i64>()
                    .map_err(|e| format!("Invalid number: {}", e))?;
                self.advance();
                Ok(Expr::Number(n))
            }
            TokenKind::StringLiteral => {
                let s = tok.text.clone();
                self.advance();
                Ok(Expr::StringLit(s))
            }
            TokenKind::InterpolatedString => {
                let s = tok.text.clone();
                self.advance();
                self.parse_interpolated_string(&s)
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            TokenKind::LParen => {
                self.advance();
                if self.current().kind == TokenKind::RParen {
                    self.advance();
                    return Ok(Expr::Tuple(Vec::new()));
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
                    if self.current().kind == TokenKind::RParen {
                        self.advance();
                    }
                    Ok(Expr::Tuple(items))
                } else {
                    if self.current().kind == TokenKind::RParen {
                        self.advance();
                    }
                    Ok(first)
                }
            }
            TokenKind::Match => self.parse_match_expr(),
            _ => Err(format!(
                "Unexpected token {:?} at {}:{}",
                tok.kind, tok.line, tok.col
            )),
        }
    }

    fn parse_match_expr(&mut self) -> Result<Expr, String> {
        self.advance();
        let expr = self.parse_expression(Precedence::Lowest)?;

        while self.current().kind == TokenKind::Newline
            || self.current().kind == TokenKind::LineComment
            || self.current().kind == TokenKind::BlockComment
            || self.current().kind == TokenKind::DocComment
        {
            self.advance();
        }

        let mut arms = Vec::new();
        if self.current().kind == TokenKind::LBracket || self.current().kind == TokenKind::Indent {
            self.advance();
            while self.current().kind != TokenKind::RBracket
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
                    || self.current().kind == TokenKind::Dedent
                    || self.current().kind == TokenKind::Eof
                {
                    break;
                }

                if self.current().kind == TokenKind::Pipe {
                    self.advance();
                }

                let pattern = self.parse_pattern()?;
                let guard =
                    if self.current().kind == TokenKind::Ident && self.current().text == "if" {
                        self.advance();
                        Some(Box::new(self.parse_expression(Precedence::Lowest)?))
                    } else {
                        None
                    };

                if self.current().kind != TokenKind::FatArrow {
                    return Err(format!(
                        "Expected '=>' in match arm at {}:{}",
                        self.current().line,
                        self.current().col
                    ));
                }
                self.advance();

                let body = self.parse_expression(Precedence::Lowest)?;
                arms.push(crate::ast::MatchArm {
                    pattern,
                    guard,
                    body: Box::new(body),
                });

                while self.current().kind == TokenKind::Newline
                    || self.current().kind == TokenKind::LineComment
                    || self.current().kind == TokenKind::BlockComment
                    || self.current().kind == TokenKind::DocComment
                {
                    self.advance();
                }
                if self.current().kind == TokenKind::Comma {
                    self.advance();
                }
            }
            if self.current().kind == TokenKind::RBracket
                || self.current().kind == TokenKind::Dedent
            {
                self.advance();
            }
        }

        Ok(Expr::Match {
            expr: Box::new(expr),
            arms,
        })
    }

    fn parse_pattern(&mut self) -> Result<crate::ast::Pattern, String> {
        let tok = self.current().clone();
        match tok.kind {
            TokenKind::Number => {
                self.advance();
                let value = tok
                    .text
                    .parse::<i64>()
                    .map_err(|e| format!("Invalid pattern literal: {}", e))?;
                Ok(crate::ast::Pattern::Literal(value))
            }
            TokenKind::Ident => {
                if tok.text == "_" {
                    self.advance();
                    return Ok(crate::ast::Pattern::Wildcard);
                }

                let name = tok.text.clone();
                self.advance();
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
            _ => Err(format!(
                "Unexpected token {:?} in pattern at {}:{}",
                tok.kind, tok.line, tok.col
            )),
        }
    }

    fn parse_interpolated_string(&mut self, s: &str) -> Result<Expr, String> {
        use crate::ast::InterpolatedFragment;
        use crate::lexer::Lexer;

        let mut frags: Vec<InterpolatedFragment> = Vec::new();
        let parts: Vec<&str> = s.split('`').collect();
        for (i, part) in parts.iter().enumerate() {
            if i % 2 == 0 {
                frags.push(InterpolatedFragment::Literal(part.to_string()));
            } else {
                // Try to parse the embedded expression using a nested lexer+parser.
                let mut lexer = Lexer::new(part);
                let toks = lexer
                    .tokenize()
                    .map_err(|e| format!("Lexer error in interpolated fragment: {}", e))?;
                let mut p = Parser::new(toks);
                let expr = p.parse_expression(Precedence::Lowest)?;
                frags.push(InterpolatedFragment::Expr(Box::new(expr)));
            }
        }
        Ok(Expr::Interpolated(frags))
    }
}
