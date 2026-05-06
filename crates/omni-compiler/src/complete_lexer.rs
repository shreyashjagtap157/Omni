//! Complete lexer for Omni - full spec implementation.
//! This lexer handles all token types required by the Omni specification.

use std::collections::HashMap;

/// Complete TokenKind enum with all spec keywords and operators
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Identifiers and literals
    Ident,
    StringLiteral,
    InterpolatedString,
    Number,
    Float,
    HexNumber,
    OctNumber,
    BinNumber,
    RawString,
    ByteString,
    Newline,
    Indent,
    Dedent,
    LineComment,
    BlockComment,
    DocComment,
    
    // Punctuation
    Equals,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    EqEq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    AndAnd,
    OrOr,
    Bang,
    LParen,
    RParen,
    Arrow,
    FatArrow,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Colon,
    ColonColon,
    Dot,
    DotDot,
    DotDotDot,
    Semi,
    At,
    Question,
    Tilde,
    
    // Keywords - Core
    True,
    False,
    Linear,
    Unsafe,
    Enum,
    Variant,
    Match,
    If,
    Then,
    Else,
    Pipe,
    Mod,
    
    // Keywords - Functions
    Fn,
    Pub,
    Async,
    Await,
    
    // Keywords - Effects
    Effect,
    Yield,
    Spawn,
    Cap,
    Friend,
    
    // Keywords - Types
    Trait,
    Impl,
    Struct,
    Class,
    Type,
    
    // Keywords - Values
    Let,
    Mut,
    Const,
    Static,
    Return,
    Break,
    Continue,
    
    // Keywords - Control Flow
    Loop,
    While,
    For,
    In,
    Where,
    
    // Keywords - Modules
    Use,
    Import,
    Export,
    From,
    As,
    Self_,
    SelfType,
    
    // Keywords - Other
    Inout,
    
    // Built-in types
    Int,
    Int8,
    Int16,
    Int32,
    Int64,
    UInt,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float32,
    Float64,
    Char,
    Bool,
    String,
    Void,
    
    // Special
    Eof,
}

/// Token representation
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub line: usize,
    pub col: usize,
}

/// Complete lexer with full spec support
pub struct CompleteLexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    indent_stack: Vec<usize>,
    at_line_start: bool,
    nesting: usize,
    keywords: HashMap<String, TokenKind>,
}

impl CompleteLexer {
    pub fn new(src: &str) -> Self {
        let mut keywords = HashMap::new();
        
        // Core keywords
        keywords.insert("true".to_string(), TokenKind::True);
        keywords.insert("false".to_string(), TokenKind::False);
        keywords.insert("linear".to_string(), TokenKind::Linear);
        keywords.insert("unsafe".to_string(), TokenKind::Unsafe);
        keywords.insert("enum".to_string(), TokenKind::Enum);
        keywords.insert("variant".to_string(), TokenKind::Variant);
        keywords.insert("match".to_string(), TokenKind::Match);
        keywords.insert("if".to_string(), TokenKind::If);
        keywords.insert("then".to_string(), TokenKind::Then);
        keywords.insert("else".to_string(), TokenKind::Else);
        keywords.insert("pipe".to_string(), TokenKind::Pipe);
        keywords.insert("mod".to_string(), TokenKind::Mod);
        
        // Function keywords
        keywords.insert("fn".to_string(), TokenKind::Fn);
        keywords.insert("pub".to_string(), TokenKind::Pub);
        keywords.insert("async".to_string(), TokenKind::Async);
        keywords.insert("await".to_string(), TokenKind::Await);
        
        // Effect keywords
        keywords.insert("effect".to_string(), TokenKind::Effect);
        keywords.insert("yield".to_string(), TokenKind::Yield);
        keywords.insert("spawn".to_string(), TokenKind::Spawn);
        keywords.insert("cap".to_string(), TokenKind::Cap);
        keywords.insert("friend".to_string(), TokenKind::Friend);
        
        // Type keywords
        keywords.insert("trait".to_string(), TokenKind::Trait);
        keywords.insert("impl".to_string(), TokenKind::Impl);
        keywords.insert("struct".to_string(), TokenKind::Struct);
        keywords.insert("class".to_string(), TokenKind::Class);
        keywords.insert("type".to_string(), TokenKind::Type);
        
        // Value keywords
        keywords.insert("let".to_string(), TokenKind::Let);
        keywords.insert("mut".to_string(), TokenKind::Mut);
        keywords.insert("const".to_string(), TokenKind::Const);
        keywords.insert("static".to_string(), TokenKind::Static);
        keywords.insert("return".to_string(), TokenKind::Return);
        keywords.insert("break".to_string(), TokenKind::Break);
        keywords.insert("continue".to_string(), TokenKind::Continue);
        
        // Control flow keywords
        keywords.insert("loop".to_string(), TokenKind::Loop);
        keywords.insert("while".to_string(), TokenKind::While);
        keywords.insert("for".to_string(), TokenKind::For);
        keywords.insert("in".to_string(), TokenKind::In);
        keywords.insert("where".to_string(), TokenKind::Where);
        
        // Module keywords
        keywords.insert("use".to_string(), TokenKind::Use);
        keywords.insert("mod".to_string(), TokenKind::Mod);
        keywords.insert("import".to_string(), TokenKind::Import);
        keywords.insert("export".to_string(), TokenKind::Export);
        keywords.insert("from".to_string(), TokenKind::From);
        keywords.insert("as".to_string(), TokenKind::As);
        keywords.insert("self".to_string(), TokenKind::Self_);
        keywords.insert("Self".to_string(), TokenKind::SelfType);
        
        // Other keywords
        keywords.insert("inout".to_string(), TokenKind::Inout);
        
        // Built-in types
        keywords.insert("int".to_string(), TokenKind::Int);
        keywords.insert("int8".to_string(), TokenKind::Int8);
        keywords.insert("int16".to_string(), TokenKind::Int16);
        keywords.insert("int32".to_string(), TokenKind::Int32);
        keywords.insert("int64".to_string(), TokenKind::Int64);
        keywords.insert("uint".to_string(), TokenKind::UInt);
        keywords.insert("uint8".to_string(), TokenKind::UInt8);
        keywords.insert("uint16".to_string(), TokenKind::UInt16);
        keywords.insert("uint32".to_string(), TokenKind::UInt32);
        keywords.insert("uint64".to_string(), TokenKind::UInt64);
        keywords.insert("float32".to_string(), TokenKind::Float32);
        keywords.insert("float64".to_string(), TokenKind::Float64);
        keywords.insert("char".to_string(), TokenKind::Char);
        keywords.insert("bool".to_string(), TokenKind::Bool);
        keywords.insert("string".to_string(), TokenKind::String);
        keywords.insert("void".to_string(), TokenKind::Void);
        
        CompleteLexer {
            chars: src.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            indent_stack: vec![0],
            at_line_start: true,
            nesting: 0,
            keywords,
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn next_char(&mut self) -> Option<char> {
        if let Some(&ch) = self.chars.get(self.pos) {
            self.pos += 1;
            if ch == '\n' {
                self.line += 1;
                self.col = 1;
            } else if ch != '\r' {  // Skip \r (carriage return for Windows line endings)
                self.col += 1;
            }
            Some(ch)
        } else {
            None
        }
    }
    
    fn indent_of(&mut self) -> usize {
        let mut indent = 0;
        while let Some(c) = self.peek_char() {
            if c == ' ' {
                indent += 1;
                self.next_char();
            } else if c == '\t' {
                indent += 4;
                self.next_char();
            } else {
                break;
            }
        }
        indent
    }

    fn skip_inline_indent(&mut self) {
        while let Some(c) = self.peek_char() {
            if c == ' ' || c == '\t' || c == '\r' {
                self.next_char();
            } else {
                break;
            }
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        
        // NO initial indent - match stage0 exactly
        
        while let Some(c) = self.peek_char() {
            // Handle newlines first (line start detection)
            if self.at_line_start {
                if c == '\n' {
                    let line = self.line;
                    let col = self.col;
                    self.next_char();
                    tokens.push(Token {
                        kind: TokenKind::Newline,
                        text: "\n".to_string(),
                        line,
                        col,
                    });
                    continue;
                }

                // Nesting check - ignore layout inside grouping
                if self.nesting > 0 {
                    self.skip_inline_indent();
                    if self.peek_char() == Some('\n') || self.peek_char().is_none() {
                        self.at_line_start = true;
                        continue;
                    }
                    self.at_line_start = false;
                } else {
                    let indent = self.indent_of();
                    if self.peek_char() == Some('\n') || self.peek_char().is_none() {
                        self.at_line_start = true;
                        continue;
                    }

                    let current = *self.indent_stack.last().unwrap();
                    if indent > current {
                        self.indent_stack.push(indent);
                        tokens.push(Token {
                            kind: TokenKind::Indent,
                            text: "".to_string(),
                            line: self.line,
                            col: self.col,
                        });
                    } else if indent < current {
                        while let Some(&top) = self.indent_stack.last() {
                            if indent < top {
                                self.indent_stack.pop();
                                tokens.push(Token {
                                    kind: TokenKind::Dedent,
                                    text: "".to_string(),
                                    line: self.line,
                                    col: self.col,
                                });
                            } else {
                                break;
                            }
                        }
                        if indent != *self.indent_stack.last().unwrap() {
                            return Err(format!("Inconsistent indentation at line {}", self.line));
                        }
                    }
                    self.at_line_start = false;
                }
                continue; // Re-read c from the advanced cursor position
            }
            
            // Regular newline detection
            if c == '\n' {
                self.next_char();
                tokens.push(Token {
                    kind: TokenKind::Newline,
                    text: "\n".to_string(),
                    line: self.line,
                    col: self.col,
                });
                self.at_line_start = true;
                continue;
            }
            
            // Skip whitespace (not at line start)
            if c == ' ' || c == '\t' || c == '\r' {
                self.next_char();
                continue;
            }
            
            self.at_line_start = false;
            
            // Line comments -- (Omni style)
            if c == '-' {
                self.next_char();
                if let Some('-') = self.peek_char() {
                    self.next_char();
                    let mut text = String::new();
                    while let Some(ch) = self.peek_char() {
                        if ch == '\n' {
                            break;
                        }
                        text.push(ch);
                        self.next_char();
                    }
                    tokens.push(Token {
                        kind: TokenKind::LineComment,
                        text,
                        line: self.line,
                        col: self.col - 2,
                    });
                    continue;
                }
            }
            
            // Line comments // (C-style)
            if c == '/' {
                self.next_char();
                if let Some('/') = self.peek_char() {
                    self.next_char();
                    let mut text = String::new();
                    while let Some(ch) = self.peek_char() {
                        if ch == '\n' {
                            break;
                        }
                        text.push(ch);
                        self.next_char();
                    }
                    tokens.push(Token {
                        kind: TokenKind::LineComment,
                        text,
                        line: self.line,
                        col: self.col - 2,
                    });
                    continue;
                }
                // Block comment /* */
                if let Some('*') = self.peek_char() {
                    self.next_char();
                    let mut text = String::new();
                    let mut closed = false;
                    while let Some(ch) = self.peek_char() {
                        if ch == '*' {
                            self.next_char();
                            if let Some('/') = self.peek_char() {
                                self.next_char();
                                closed = true;
                                break;
                            }
                            text.push('*');
                        } else {
                            text.push(ch);
                            self.next_char();
                        }
                    }
                    tokens.push(Token {
                        kind: if closed { TokenKind::BlockComment } else { TokenKind::DocComment },
                        text,
                        line: self.line,
                        col: self.col - 2,
                    });
                    continue;
                }
                // Not a comment, put '/' back
                tokens.push(Token {
                    kind: TokenKind::Slash,
                    text: "/".to_string(),
                    line: self.line,
                    col: self.col,
                });
                continue;
            }
            
            // String literals
            if c == '"' {
                let start_line = self.line;
                let start_col = self.col;
                self.next_char();
                let mut s = String::new();
                let mut interp = false;
                loop {
                    match self.peek_char() {
                        Some('"') => {
                            self.next_char();
                            break;
                        }
                        Some('`') => {
                            self.next_char();
                            interp = true;
                        }
                        Some('\\') => {
                            self.next_char();
                            if let Some(esc) = self.next_char() {
                                match esc {
                                    'n' => s.push('\n'),
                                    't' => s.push('\t'),
                                    'r' => s.push('\r'),
                                    '\\' => s.push('\\'),
                                    '"' => s.push('"'),
                                    '0' => s.push('\0'),
                                    other => s.push(other),
                                }
                            }
                        }
                        Some(ch) => {
                            s.push(ch);
                            self.next_char();
                        }
                        None => return Err("Unterminated string".to_string()),
                    }
                }
                tokens.push(Token {
                    kind: if interp { TokenKind::InterpolatedString } else { TokenKind::StringLiteral },
                    text: s,
                    line: start_line,
                    col: start_col,
                });
                continue;
            }
            
            // Raw strings r#"..."#
            if c == 'r' && self.peek_nChar(1) == Some('#') {
                self.next_char(); // consume 'r'
                if let Some('#') = self.peek_char() {
                    let mut hashes = String::new();
                    while let Some('#') = self.peek_char() {
                        hashes.push('#');
                        self.next_char();
                    }
                    if let Some('"') = self.peek_char() {
                        self.next_char();
                        let mut s = String::new();
                        let closer = format!("\"{}", hashes);
                        loop {
                            let mut collected = String::new();
                            while let Some(ch) = self.peek_char() {
                                collected.push(ch);
                                self.next_char();
                                if collected.ends_with(&closer) {
                                    break;
                                }
                            }
                            if collected.ends_with(&closer) {
                                s.push_str(&collected[..collected.len() - closer.len()]);
                                break;
                            } else if self.peek_char().is_none() {
                                return Err("Unterminated raw string".to_string());
                            } else {
                                s.push_str(&collected);
                            }
                        }
                        tokens.push(Token {
                            kind: TokenKind::RawString,
                            text: s,
                            line: self.line,
                            col: self.col,
                        });
                        continue;
                    }
                }
                // Not a raw string — should not reach here since we checked peek ahead
                // but fall through to identifier parsing by putting 'r' back
                tokens.push(Token {
                    kind: TokenKind::Ident,
                    text: "r".to_string(),
                    line: self.line,
                    col: self.col,
                });
                continue;
            }
            
            // Byte string b"..."
            if c == 'b' && self.peek_nChar(1) == Some('"') {
                self.next_char(); // consume 'b'
                if let Some('"') = self.peek_char() {
                    self.next_char();
                    let mut s = String::new();
                    while let Some(ch) = self.peek_char() {
                        if ch == '"' {
                            break;
                        }
                        s.push(ch);
                        self.next_char();
                    }
                    if self.peek_char() == Some('"') {
                        self.next_char();
                    }
                    tokens.push(Token {
                        kind: TokenKind::ByteString,
                        text: s,
                        line: self.line,
                        col: self.col,
                    });
                    continue;
                }
            }
            
            // Identifiers and keywords
            if c.is_alphabetic() || c == '_' {
                let start_line = self.line;
                let start_col = self.col;
                let mut id = String::new();
                while let Some(ch) = self.peek_char() {
                    if ch.is_alphanumeric() || ch == '_' {
                        id.push(ch);
                        self.next_char();
                    } else {
                        break;
                    }
                }
                let kind = self.keywords.get(&id).cloned().unwrap_or(TokenKind::Ident);
                tokens.push(Token {
                    kind,
                    text: id,
                    line: start_line,
                    col: start_col,
                });
                continue;
            }
            
            // Numbers
            if c.is_ascii_digit() {
                let start_line = self.line;
                let start_col = self.col;
                let mut num = String::new();
                
                // Check for hex, octal, binary
                if c == '0' {
                    num.push(c);
                    self.next_char();
                    match self.peek_char() {
                        Some('x') | Some('X') => {
                            num.push('x');
                            self.next_char();
                            while let Some(ch) = self.peek_char() {
                                if ch.is_ascii_hexdigit() {
                                    num.push(ch);
                                    self.next_char();
                                } else {
                                    break;
                                }
                            }
                            tokens.push(Token {
                                kind: TokenKind::HexNumber,
                                text: num,
                                line: start_line,
                                col: start_col,
                            });
                            continue;
                        }
                        Some('o') | Some('O') => {
                            num.push('o');
                            self.next_char();
                            while let Some(ch) = self.peek_char() {
                // octal digits: 0-7
                if ch >= '0' && ch <= '7' {
                                    num.push(ch);
                                    self.next_char();
                                } else {
                                    break;
                                }
                            }
                            tokens.push(Token {
                                kind: TokenKind::OctNumber,
                                text: num,
                                line: start_line,
                                col: start_col,
                            });
                            continue;
                        }
                        Some('b') | Some('B') => {
                            num.push('b');
                            self.next_char();
                            while let Some(ch) = self.peek_char() {
                                if ch == '0' || ch == '1' {
                                    num.push(ch);
                                    self.next_char();
                                } else {
                                    break;
                                }
                            }
                            tokens.push(Token {
                                kind: TokenKind::BinNumber,
                                text: num,
                                line: start_line,
                                col: start_col,
                            });
                            continue;
                        }
                        _ => {}
                    }
                }
                
                // Decimal number
                while let Some(ch) = self.peek_char() {
                    if ch.is_ascii_digit() || ch == '_' {
                        num.push(ch);
                        self.next_char();
                    } else if ch == '.' {
                        let next_c = self.peek_nChar(1);
                        if next_c.is_some() && next_c.unwrap().is_ascii_digit() {
                            num.push('.');
                            self.next_char();
                            while let Some(ch2) = self.peek_char() {
                                if ch2.is_ascii_digit() || ch2 == '_' {
                                    num.push(ch2);
                                    self.next_char();
                                } else {
                                    break;
                                }
                            }
                        }
                        break;
                    } else {
                        break;
                    }
                }
                
                // Check for float suffix
                let kind = if num.contains('.') {
                    TokenKind::Float
                } else {
                    TokenKind::Number
                };
                
                // Optional type suffix
                if let Some(ch) = self.peek_char() {
                    if ch.is_alphabetic() {
                        num.push(ch);
                        self.next_char();
                    }
                }
                
                tokens.push(Token {
                    kind,
                    text: num,
                    line: start_line,
                    col: start_col,
                });
                continue;
            }
            
            // Single and multi-character operators
            let start_line = self.line;
            let start_col = self.col;
            self.next_char();
            let kind = match c {
                '=' => {
                    if self.peek_char() == Some('=') {
                        self.next_char();
                        TokenKind::EqEq
                    } else if self.peek_char() == Some('>') {
                        self.next_char();
                        TokenKind::FatArrow
                    } else {
                        TokenKind::Equals
                    }
                }
                '+' => TokenKind::Plus,
                '-' => {
                    if self.peek_char() == Some('>') {
                        self.next_char();
                        TokenKind::Arrow
                    } else {
                        TokenKind::Minus
                    }
                }
                '*' => TokenKind::Star,
                '/' => TokenKind::Slash,
                '%' => TokenKind::Percent,
                '!' => {
                    if self.peek_char() == Some('=') {
                        self.next_char();
                        TokenKind::NotEq
                    } else {
                        TokenKind::Bang
                    }
                }
                '<' => {
                    if self.peek_char() == Some('=') {
                        self.next_char();
                        TokenKind::LtEq
                    } else {
                        TokenKind::Lt
                    }
                }
                '>' => {
                    if self.peek_char() == Some('=') {
                        self.next_char();
                        TokenKind::GtEq
                    } else {
                        TokenKind::Gt
                    }
                }
                '&' => {
                    if self.peek_char() == Some('&') {
                        self.next_char();
                        TokenKind::AndAnd
                    } else {
                        TokenKind::Bang // Error case
                    }
                }
                '|' => {
                    if self.peek_char() == Some('|') {
                        self.next_char();
                        TokenKind::OrOr
                    } else {
                        TokenKind::Pipe
                    }
                }
                '(' => { self.nesting += 1; TokenKind::LParen }
                ')' => { self.nesting = self.nesting.saturating_sub(1); TokenKind::RParen }
                '[' => { self.nesting += 1; TokenKind::LBracket }
                ']' => { self.nesting = self.nesting.saturating_sub(1); TokenKind::RBracket }
                '{' => { self.nesting += 1; TokenKind::LBrace }
                '}' => { self.nesting = self.nesting.saturating_sub(1); TokenKind::RBrace }
                ';' => TokenKind::Semi,
                ',' => TokenKind::Comma,
                ':' => {
                    if self.peek_char() == Some(':') {
                        self.next_char();
                        TokenKind::ColonColon
                    } else {
                        TokenKind::Colon
                    }
                }
                '.' => {
                    if self.peek_char() == Some('.') {
                        self.next_char();
                        if self.peek_char() == Some('.') {
                            self.next_char();
                            TokenKind::DotDotDot
                        } else {
                            TokenKind::DotDot
                        }
                    } else {
                        TokenKind::Dot
                    }
                }
                '@' => TokenKind::At,
                '?' => TokenKind::Question,
                '~' => TokenKind::Tilde,
                _ => TokenKind::Ident,
            };
            tokens.push(Token { kind, text: c.to_string(), line: start_line, col: start_col });
        }
        
        // Emit remaining dedents
        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            tokens.push(Token {
                kind: TokenKind::Dedent,
                text: "dedent".to_string(),
                line: self.line,
                col: 1,
            });
        }
        
        tokens.push(Token {
            kind: TokenKind::Eof,
            text: "".to_string(),
            line: self.line,
            col: self.col,
        });
        
        Ok(tokens)
    }
    
    fn emit_eof_dedents(&mut self, tokens: &mut Vec<Token>) {
        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            tokens.push(Token {
                kind: TokenKind::Dedent,
                text: "".to_string(),
                line: self.line,
                col: self.col,
            });
        }
    }
    
    fn peek_nChar(&self, n: usize) -> Option<char> {
        self.chars.get(self.pos + n).copied()
    }
}

/// Tokenize a complete source file using the complete lexer
pub fn tokenize_complete(src: &str) -> Result<Vec<Token>, String> {
    let mut lexer = CompleteLexer::new(src);
    lexer.tokenize()
}
