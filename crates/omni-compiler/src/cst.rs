use crate::lexer::{Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxKind {
    Root,
    Block,
    Statement,
    TokenIdent,
    TokenNumber,
    TokenString,
    TokenEquals,
    TokenNewline,
    TokenIndent,
    TokenDedent,
    TokenCommentLine,
    TokenCommentBlock,
    TokenDocComment,
    TokenOther,
}

#[derive(Debug, Clone)]
pub struct SyntaxToken {
    pub kind: SyntaxKind,
    pub text: String,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub enum SyntaxElement {
    Node(SyntaxNode),
    Token(SyntaxToken),
}

#[derive(Debug, Clone)]
pub struct SyntaxNode {
    pub kind: SyntaxKind,
    pub children: Vec<SyntaxElement>,
}

impl SyntaxNode {
    pub fn new(kind: SyntaxKind) -> Self {
        SyntaxNode {
            kind,
            children: Vec::new(),
        }
    }
}

fn map_token_kind(t: &Token) -> SyntaxKind {
    match t.kind {
        TokenKind::Ident => SyntaxKind::TokenIdent,
        TokenKind::Number => SyntaxKind::TokenNumber,
        TokenKind::StringLiteral => SyntaxKind::TokenString,
        TokenKind::Equals => SyntaxKind::TokenEquals,
        TokenKind::Newline => SyntaxKind::TokenNewline,
        TokenKind::Indent => SyntaxKind::TokenIndent,
        TokenKind::Dedent => SyntaxKind::TokenDedent,
        TokenKind::LineComment => SyntaxKind::TokenCommentLine,
        TokenKind::BlockComment => SyntaxKind::TokenCommentBlock,
        TokenKind::DocComment => SyntaxKind::TokenDocComment,
        _ => SyntaxKind::TokenOther,
    }
}

fn token_to_element(t: &Token) -> SyntaxElement {
    SyntaxElement::Token(SyntaxToken {
        kind: map_token_kind(t),
        text: t.text.clone(),
        line: t.line,
        col: t.col,
    })
}

// Build a simple lossless CST by grouping tokens into statements and blocks.
pub fn build_cst(tokens: &[Token]) -> SyntaxNode {
    fn build_nodes(tokens: &[Token], out: &mut Vec<SyntaxElement>) {
        let mut i = 0usize;
        while i < tokens.len() {
            match tokens[i].kind {
                TokenKind::Indent => {
                    // find matching Dedent using depth
                    let mut depth = 1usize;
                    let mut j = i + 1;
                    while j < tokens.len() && depth > 0 {
                        match tokens[j].kind {
                            TokenKind::Indent => depth += 1,
                            TokenKind::Dedent => depth -= 1,
                            _ => {}
                        }
                        j += 1;
                    }
                    // j is one past the matching Dedent (or end)
                    let end = if j > 0 { j - 1 } else { j };
                    let mut node = SyntaxNode::new(SyntaxKind::Block);
                    // build children from i+1 .. end
                    build_nodes(&tokens[i + 1..end], &mut node.children);
                    // include the indent token as leading token
                    node.children.insert(0, token_to_element(&tokens[i]));
                    // include the dedent token as trailing token if present
                    if end < tokens.len() {
                        node.children.push(token_to_element(&tokens[end]));
                    }
                    out.push(SyntaxElement::Node(node));
                    i = j;
                }
                TokenKind::Dedent => {
                    // stray dedent; emit as token
                    out.push(token_to_element(&tokens[i]));
                    i += 1;
                }
                TokenKind::Newline => {
                    out.push(token_to_element(&tokens[i]));
                    i += 1;
                }
                _ => {
                    // collect a statement until newline, indent, or dedent
                    let mut stmt = SyntaxNode::new(SyntaxKind::Statement);
                    while i < tokens.len() {
                        match tokens[i].kind {
                            TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent => break,
                            _ => {
                                stmt.children.push(token_to_element(&tokens[i]));
                                i += 1;
                            }
                        }
                    }
                    out.push(SyntaxElement::Node(stmt));
                }
            }
        }
    }

    let mut root = SyntaxNode::new(SyntaxKind::Root);
    build_nodes(tokens, &mut root.children);
    root
}

// Pretty-print a CST for quick CLI inspection.
pub fn format_cst(node: &SyntaxNode, indent: usize) -> String {
    let mut out = String::new();
    let pad = " ".repeat(indent);
    out.push_str(&format!("{}{:?}\n", pad, node.kind));
    for child in &node.children {
        match child {
            SyntaxElement::Node(n) => out.push_str(&format_cst(n, indent + 2)),
            SyntaxElement::Token(t) => out.push_str(&format!(
                "{}  TOKEN {:?} '{}' ({}:{})\n",
                pad, t.kind, t.text, t.line, t.col
            )),
        }
    }
    out
}
