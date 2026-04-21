use crate::lexer::TokenKind;

#[derive(Debug, Default)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Expr {
    StringLit(String),
    Interpolated(Vec<InterpolatedFragment>),
    Number(i64),
    Var(String),
    Bool(bool),
    Call(String, Vec<Expr>),
    BinaryOp {
        op: TokenKind,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    UnaryOp {
        op: TokenKind,
        inner: Box<Expr>,
    },
    FieldAccess {
        base: Box<Expr>,
        field: String,
    },
    IfExpr {
        cond: Box<Expr>,
        then: Box<Expr>,
        else_: Box<Expr>,
    },
    Block(Vec<Stmt>),
    Tuple(Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
    },
}

#[derive(Debug, Clone)]
pub enum InterpolatedFragment {
    Literal(String),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Box<Expr>>,
    pub body: Box<Expr>,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard,
    Literal(i64),
    Var(String),
    Struct(String, Vec<(String, Pattern)>),
    Or(Vec<Pattern>),
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Print(Expr),
    Let(String, Expr),
    LetLinear(String, Expr),
    ExprStmt(Expr),
    Block(Vec<Stmt>),
    Fn {
        name: String,
        type_params: Vec<String>,
        params: Vec<String>,
        ret_type: Option<String>,
        effects: Vec<String>,
        body: Vec<Stmt>,
    },
    Struct {
        name: String,
        fields: Vec<(String, String)>,
        is_linear: bool,
    },
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
        is_sealed: bool,
    },
    If {
        cond: Box<Expr>,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
    Loop {
        body: Vec<Stmt>,
    },
    For {
        var_name: String,
        iterable: Box<Expr>,
        body: Vec<Stmt>,
    },
    While {
        cond: Box<Expr>,
        body: Vec<Stmt>,
    },
    Return(Expr),
    Break,
    Continue,
    Assign(String, Expr),
    ExprFieldAssign(Box<Expr>, String, Expr),
    WhileIn {
        var_name: String,
        iterable: Box<Expr>,
        body: Vec<Stmt>,
    },
    Unsafe {
        body: Vec<Stmt>,
    },
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<(String, String)>,
}

impl Program {
    pub fn new() -> Self {
        Program { stmts: Vec::new() }
    }
}
