// Minimal lexer for Stage0 — single, clean implementation
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Ident,
    StringLiteral,
    InterpolatedString,
    Number,
    Newline,
    Indent,
    Dedent,
    LineComment,
    BlockComment,
    DocComment,
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
    Comma,
    Colon,
    Dot,
    DotDot,
    DotDotDot,
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
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub line: usize,
    pub col: usize,
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    indent_stack: Vec<usize>,
    at_line_start: bool,
}

impl Lexer {
    pub fn new(src: &str) -> Self {
        Lexer {
            chars: src.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            indent_stack: vec![0],
            at_line_start: true,
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
            } else {
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

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();

        while let Some(ch) = self.peek_char() {
            if self.at_line_start {
                if ch == '\n' {
                    let line = self.line;
                    let col = self.col;
                    self.next_char();
                    tokens.push(Token {
                        kind: TokenKind::Newline,
                        text: "\n".into(),
                        line,
                        col,
                    });
                    continue;
                }

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
                        text: "".into(),
                        line: self.line,
                        col: self.col,
                    });
                } else if indent < current {
                    while let Some(&top) = self.indent_stack.last() {
                        if indent < top {
                            self.indent_stack.pop();
                            tokens.push(Token {
                                kind: TokenKind::Dedent,
                                text: "".into(),
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

            if let Some(c) = self.peek_char() {
                if c == '\n' {
                    let line = self.line;
                    let col = self.col;
                    self.next_char();
                    tokens.push(Token {
                        kind: TokenKind::Newline,
                        text: "\n".into(),
                        line,
                        col,
                    });
                    self.at_line_start = true;
                    continue;
                }
                if c.is_whitespace() {
                    self.next_char();
                    continue;
                }

                // comments / arrow
                if c == '-' {
                    let second = self.chars.get(self.pos + 1).copied();
                    if second == Some('-') {
                        let third = self.chars.get(self.pos + 2).copied();
                        if third == Some('-') {
                            self.next_char();
                            self.next_char();
                            self.next_char();
                            let start_line = self.line;
                            let start_col = self.col;
                            let mut s = String::new();
                            loop {
                                if self.peek_char() == Some('-')
                                    && self.chars.get(self.pos + 1).copied() == Some('-')
                                    && self.chars.get(self.pos + 2).copied() == Some('-')
                                {
                                    self.next_char();
                                    self.next_char();
                                    self.next_char();
                                    break;
                                }
                                match self.next_char() {
                                    Some(ch) => s.push(ch),
                                    None => break,
                                }
                            }
                            tokens.push(Token {
                                kind: TokenKind::BlockComment,
                                text: s,
                                line: start_line,
                                col: start_col,
                            });
                            continue;
                        } else {
                            self.next_char();
                            self.next_char();
                            let start_line = self.line;
                            let start_col = self.col;
                            let mut s = String::new();
                            while let Some(ch2) = self.peek_char() {
                                if ch2 == '\n' {
                                    break;
                                }
                                s.push(ch2);
                                self.next_char();
                            }
                            tokens.push(Token {
                                kind: TokenKind::LineComment,
                                text: s,
                                line: start_line,
                                col: start_col,
                            });
                            continue;
                        }
                    } else if second == Some('>') {
                        let line = self.line;
                        let col = self.col;
                        self.next_char();
                        self.next_char();
                        tokens.push(Token {
                            kind: TokenKind::Arrow,
                            text: "->".into(),
                            line,
                            col,
                        });
                        continue;
                    }
                }

                if c == '/'
                    && self.chars.get(self.pos + 1).copied() == Some('/')
                    && self.chars.get(self.pos + 2).copied() == Some('/')
                {
                    self.next_char();
                    self.next_char();
                    self.next_char();
                    let start_line = self.line;
                    let start_col = self.col;
                    let mut s = String::new();
                    while let Some(ch2) = self.peek_char() {
                        if ch2 == '\n' {
                            break;
                        }
                        s.push(ch2);
                        self.next_char();
                    }
                    tokens.push(Token {
                        kind: TokenKind::DocComment,
                        text: s,
                        line: start_line,
                        col: start_col,
                    });
                    continue;
                }

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
                            Some('{') => {
                                self.next_char();
                                if self.peek_char() == Some('{') {
                                    s.push('{');
                                    self.next_char();
                                } else {
                                    interp = true;
                                    s.push('`');
                                }
                            }
                            Some('}') => {
                                self.next_char();
                                if self.peek_char() == Some('}') {
                                    s.push('}');
                                    self.next_char();
                                } else {
                                    s.push('`');
                                }
                            }
                            Some('\\') => {
                                self.next_char();
                                if let Some(esc) = self.next_char() {
                                    match esc {
                                        'n' => s.push('\n'),
                                        't' => s.push('\t'),
                                        '\\' => s.push('\\'),
                                        '"' => s.push('"'),
                                        other => s.push(other),
                                    }
                                } else {
                                    return Err("Unterminated escape in string".into());
                                }
                            }
                            Some(ch2) => {
                                s.push(ch2);
                                self.next_char();
                            }
                            None => return Err("Unterminated string literal".into()),
                        }
                    }
                    tokens.push(Token {
                        kind: if interp {
                            TokenKind::InterpolatedString
                        } else {
                            TokenKind::StringLiteral
                        },
                        text: s,
                        line: start_line,
                        col: start_col,
                    });
                    continue;
                }

                if c.is_ascii_alphabetic() || c == '_' {
                    let start_line = self.line;
                    let start_col = self.col;
                    let mut id = String::new();
                    while let Some(ch2) = self.peek_char() {
                        if ch2.is_ascii_alphanumeric() || ch2 == '_' {
                            id.push(ch2);
                            self.next_char();
                        } else {
                            break;
                        }
                    }
                    let kind = match id.as_str() {
                        "true" => TokenKind::True,
                        "false" => TokenKind::False,
                        "linear" => TokenKind::Linear,
                        "unsafe" => TokenKind::Unsafe,
                        "enum" => TokenKind::Enum,
                        "variant" => TokenKind::Variant,
                        "match" => TokenKind::Match,
                        "if" => TokenKind::If,
                        "then" => TokenKind::Then,
                        "else" => TokenKind::Else,
                        "pipe" => TokenKind::Pipe,
                        _ => TokenKind::Ident,
                    };
                    tokens.push(Token {
                        kind,
                        text: id,
                        line: start_line,
                        col: start_col,
                    });
                    continue;
                }

                if c.is_ascii_digit() {
                    let start_line = self.line;
                    let start_col = self.col;
                    let mut num = String::new();
                    while let Some(ch2) = self.peek_char() {
                        if ch2.is_ascii_digit() {
                            num.push(ch2);
                            self.next_char();
                        } else {
                            break;
                        }
                    }
                    tokens.push(Token {
                        kind: TokenKind::Number,
                        text: num,
                        line: start_line,
                        col: start_col,
                    });
                    continue;
                }

                // single/double char operators and punctuation
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
                    '-' => TokenKind::Minus,
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
                            return Err(format!("Unexpected '&' at {}:{}", self.line, self.col));
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
                    '(' => TokenKind::LParen,
                    ')' => TokenKind::RParen,
                    '[' => TokenKind::LBracket,
                    ']' => TokenKind::RBracket,
                    ',' => TokenKind::Comma,
                    ':' => TokenKind::Colon,
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
                    _ => continue,
                };
                tokens.push(Token {
                    kind,
                    text: "".into(),
                    line: start_line,
                    col: start_col,
                });
            }
        }

        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            tokens.push(Token {
                kind: TokenKind::Dedent,
                text: "".into(),
                line: self.line,
                col: self.col,
            });
        }
        tokens.push(Token {
            kind: TokenKind::Eof,
            text: "".into(),
            line: self.line,
            col: self.col,
        });
        Ok(tokens)
    }
}
