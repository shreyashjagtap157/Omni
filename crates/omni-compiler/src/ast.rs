use crate::complete_lexer::TokenKind;
use crate::diagnostics::Span;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Visibility {
    #[default]
    Private,
    PubMod,
    PubPkg,
    Pub,
    PubCap(String),
    PubFriend(String),
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    StringLit(String, Span),
    ByteString(Vec<u8>, Span),
    Byte(u8, Span),
    Interpolated(Vec<InterpolatedFragment>, Span),
    Number(i64, Span),
    Float(f64, Span),
    Char(char, Span),
    Var(String, Span),
    Bool(bool, Span),
    Call(String, Vec<Expr>, Span),
    BinaryOp {
        op: TokenKind,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    UnaryOp {
        op: TokenKind,
        inner: Box<Expr>,
        span: Span,
    },
    Borrow {
        mutable: bool,
        inner: Box<Expr>,
        span: Span,
    },
    Deref {
        inner: Box<Expr>,
        span: Span,
    },
    FieldAccess {
        base: Box<Expr>,
        field: String,
        span: Span,
    },
    IfExpr {
        cond: Box<Expr>,
        then: Box<Expr>,
        else_: Box<Expr>,
        span: Span,
    },
    Block(Vec<Stmt>, Span),
    Tuple(Vec<Expr>, Span),
    Array(Vec<Expr>, Span),
    Index(Box<Expr>, Box<Expr>, Span),
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        inclusive: bool,
        span: Span,
    },
    Lambda {
        params: Vec<(String, Option<String>)>,
        body: Box<Expr>,
        span: Span,
    },
    Await(Box<Expr>, Span),
    Try(Box<Expr>, Span),
    StructLit {
        name: String,
        fields: Vec<(String, Expr)>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::StringLit(_, s) => s.clone(),
            Expr::ByteString(_, s) => s.clone(),
            Expr::Byte(_, s) => s.clone(),
            Expr::Interpolated(_, s) => s.clone(),
            Expr::Number(_, s) => s.clone(),
            Expr::Float(_, s) => s.clone(),
            Expr::Char(_, s) => s.clone(),
            Expr::Var(_, s) => s.clone(),
            Expr::Bool(_, s) => s.clone(),
            Expr::Call(_, _, s) => s.clone(),
            Expr::BinaryOp { span, .. } => span.clone(),
            Expr::UnaryOp { span, .. } => span.clone(),
            Expr::Borrow { span, .. } => span.clone(),
            Expr::Deref { span, .. } => span.clone(),
            Expr::FieldAccess { span, .. } => span.clone(),
            Expr::IfExpr { span, .. } => span.clone(),
            Expr::Block(_, s) => s.clone(),
            Expr::Tuple(_, s) => s.clone(),
            Expr::Array(_, s) => s.clone(),
            Expr::Index(_, _, s) => s.clone(),
            Expr::Match { span, .. } => span.clone(),
            Expr::Range { span, .. } => span.clone(),
            Expr::Lambda { span, .. } => span.clone(),
            Expr::Await(_, s) => s.clone(),
            Expr::Try(_, s) => s.clone(),
            Expr::StructLit { span, .. } => span.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InterpolatedFragment {
    Literal(String, Span),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Box<Expr>>,
    pub body: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Wildcard,
    Literal(i64),
    Var(String),
    Struct(String, Vec<(String, Pattern)>),
    Or(Vec<Pattern>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Annotation(String, Span),
    Print(Expr, Span),
    Let(String, Option<String>, Expr, Span),
    LetMut(String, Option<String>, Expr, Span),
    LetLinear(String, Option<String>, Expr, Span),
    ExprStmt(Expr, Span),
    Block(Vec<Stmt>, Span),
    Mod(String, Span),
    ModBlock(String, Vec<Stmt>, Span),
    Fn {
        name: String,
        visibility: Visibility,
        is_async: bool,
        type_params: Vec<(String, Vec<String>)>, // (param_name, trait_bounds)
        params: Vec<(String, Option<String>)>,
        ret_type: Option<String>,
        effects: Vec<String>,
        contracts: Vec<Stmt>,
        body: Vec<Stmt>,
        span: Span,
    },
    Struct {
        name: String,
        visibility: Visibility,
        fields: Vec<(String, String)>,
        is_linear: bool,
        span: Span,
    },
    Enum {
        name: String,
        visibility: Visibility,
        variants: Vec<EnumVariant>,
        is_sealed: bool,
        span: Span,
    },
    ErrorSet {
        name: String,
        visibility: Visibility,
        variants: Vec<EnumVariant>,
        span: Span,
    },
    If {
        cond: Box<Expr>,
        bindings: Vec<(String, Expr)>,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
        span: Span,
    },
    Loop {
        body: Vec<Stmt>,
        span: Span,
    },
    For {
        var_name: String,
        iterable: Box<Expr>,
        body: Vec<Stmt>,
        span: Span,
    },
    While {
        cond: Box<Expr>,
        body: Vec<Stmt>,
        span: Span,
    },
    Return(Expr, Span),
    Break(Span),
    Continue(Span),
    Defer {
        cleanup: Box<Stmt>,
        span: Span,
    },
    AsyncDefer {
        cleanup: Box<Stmt>,
        span: Span,
    },
    Assign(String, Expr, Span),
    ExprFieldAssign(Box<Expr>, String, Expr, Span),
    DerefAssign(Box<Expr>, Expr, Span),
    WhileIn {
        var_name: String,
        iterable: Box<Expr>,
        body: Vec<Stmt>,
        span: Span,
    },
    Unsafe {
        body: Vec<Stmt>,
        span: Span,
    },
    Impl {
        target: String,
        visibility: Visibility,
        type_params: Vec<(String, Vec<String>)>, // (param_name, trait_bounds)
        for_type: Option<String>, // The type after `for`, e.g. `impl Trait for Type` => Some("Type"); inherent impl => None
        methods: Vec<Stmt>,
        span: Span,
    },
    Trait {
        name: String,
        visibility: Visibility,
        type_params: Vec<(String, Vec<String>)>, // (param_name, trait_bounds)
        methods: Vec<Stmt>,
        diagnostic_attrs: Vec<DiagnosticAttribute>,
        span: Span,
    },
    TypeAlias {
        name: String,
        visibility: Visibility,
        type_params: Vec<(String, Vec<String>)>, // (param_name, trait_bounds)
        target: String,
        span: Span,
    },
    Use {
        path: String,
        alias: Option<String>,
        span: Span,
    },
    UseScoped {
        path: String,
        aliases: Vec<(String, Option<String>)>,
        body: Vec<Stmt>,
        span: Span,
    },
    GcMode {
        mode: String,
        span: Span,
    },
    CancelToken {
        inner: Option<Box<Stmt>>,
        span: Span,
    },
    EffectHandler {
        effect: String,
        handler: Box<Expr>,
        span: Span,
    },
    Spawn {
        task: Box<Expr>,
        span: Span,
    },
    Channel {
        elem_type: String,
        capacity: Option<u32>,
        span: Span,
    },
    Actor {
        name: String,
        state: String,
        handlers: Vec<Stmt>,
        span: Span,
    },
    WorkStealingExecutor {
        num_threads: u32,
        queue_type: String,
        span: Span,
    },
    DeterministicRuntime {
        max_tasks: u32,
        span: Span,
    },
    Tensor {
        shape: Vec<u32>,
        dtype: String,
        span: Span,
    },
    Simd {
        width: u32,
        elem_type: String,
        span: Span,
    },
    DocComment {
        target: String,
        content: String,
        span: Span,
    },
    DebugSession {
        port: u32,
        breakpoints: Vec<String>,
        span: Span,
    },
    Capability {
        name: String,
        permissions: Vec<String>,
        span: Span,
    },
    FfiSandbox {
        allow_list: Vec<String>,
        span: Span,
    },
    ContractRequires {
        condition: Expr,
        message: String,
        span: Span,
    },
    ContractEnsures {
        condition: Expr,
        message: String,
        span: Span,
    },
    ContractInvariant {
        condition: Expr,
        message: String,
        span: Span,
    },
    ComptimeLimit {
        max_ops: u64,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticAttribute {
    pub message: String,
    pub label: Option<String>,
}

impl Program {
    pub fn new() -> Self {
        Program { stmts: Vec::new() }
    }
}
