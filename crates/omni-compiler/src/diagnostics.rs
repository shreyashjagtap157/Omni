use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiagnosticCode(pub &'static str);

impl DiagnosticCode {
    pub fn code(&self) -> &str {
        self.0
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "E{:04}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Span {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub source_text: Option<String>,
}

impl Span {
    pub fn new(start_line: usize, start_col: usize, end_line: usize, end_col: usize) -> Self {
        Self {
            start_line,
            start_col,
            end_line,
            end_col,
            source_text: None,
        }
    }

    pub fn with_source(mut self, source: &str) -> Self {
        self.source_text = Some(source.to_string());
        self
    }

    pub fn from_token(line: usize, col: usize, text: &str) -> Self {
        Self::new(line, col, line, col + text.len())
    }
}

#[derive(Debug, Clone)]
pub struct Label {
    pub span: Span,
    pub message: Option<String>,
    pub style: LabelStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelStyle {
    Primary,
    Secondary,
}

impl Label {
    pub fn primary(span: Span) -> Self {
        Self {
            span,
            message: None,
            style: LabelStyle::Primary,
        }
    }

    pub fn secondary(span: Span) -> Self {
        Self {
            span,
            message: None,
            style: LabelStyle::Secondary,
        }
    }

    pub fn with_message(mut self, msg: &str) -> Self {
        self.message = Some(msg.to_string());
        self
    }
}

#[derive(Debug, Clone)]
pub struct Suggestion {
    pub label: String,
    pub span: Span,
    pub text: String,
    pub applicability: Applicability,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applicability {
    Always,
    Unspecified,
    Never,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub message: String,
    pub severity: Severity,
    pub spans: Vec<Span>,
    pub labels: Vec<Label>,
    pub suggestions: Vec<Suggestion>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

impl Diagnostic {
    pub fn error(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            severity: Severity::Error,
            spans: Vec::new(),
            labels: Vec::new(),
            suggestions: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn warning(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            severity: Severity::Warning,
            spans: Vec::new(),
            labels: Vec::new(),
            suggestions: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.spans.push(span);
        self
    }

    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    pub fn with_suggestion(mut self, suggestion: Suggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity_str = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
            Severity::Help => "help",
        };

        write!(f, "{} [{}]: {}", severity_str, self.code, self.message)?;

        for label in &self.labels {
            write!(
                f,
                "\n --> {}:{}",
                label.span.start_line, label.span.start_col
            )?;
            if let Some(msg) = &label.message {
                write!(f, ": {}", msg)?;
            }
        }

        for suggestion in &self.suggestions {
            write!(f, "\n  = {}", suggestion.label)?;
            write!(f, "\n   Replacement: \"{}\"", suggestion.text)?;
        }

        for note in &self.notes {
            write!(f, "\n  note: {}", note)?;
        }

        Ok(())
    }
}

pub mod error_codes {
    use super::DiagnosticCode;

    // Lexer error codes (E1xxx)
    pub const LEXER_UNTERMINATED_STRING: DiagnosticCode = DiagnosticCode("1001");
    pub const LEXER_INVALID_ESCAPE: DiagnosticCode = DiagnosticCode("1002");
    pub const LEXER_INVALID_NUMBER: DiagnosticCode = DiagnosticCode("1003");
    pub const LEXER_UNEXPECTED_CHAR: DiagnosticCode = DiagnosticCode("1004");
    pub const LEXER_INCONSISTENT_INDENT: DiagnosticCode = DiagnosticCode("1005");

    // Parser error codes (E2xxx)
    pub const PARSER_UNEXPECTED_TOKEN: DiagnosticCode = DiagnosticCode("2001");
    pub const PARSER_MISSING_TOKEN: DiagnosticCode = DiagnosticCode("2002");
    pub const PARSER_INVALID_EXPRESSION: DiagnosticCode = DiagnosticCode("2003");
    pub const PARSER_INVALID_STATEMENT: DiagnosticCode = DiagnosticCode("2004");
    pub const PARSER_UNEXPECTED_EOF: DiagnosticCode = DiagnosticCode("2005");
    pub const PARSER_INVALID_TYPE: DiagnosticCode = DiagnosticCode("2006");

    // Resolver error codes (E3xxx)
    pub const RESOLVER_UNDEFINED_NAME: DiagnosticCode = DiagnosticCode("3001");
    pub const RESOLVER_UNDEFINED_FUNCTION: DiagnosticCode = DiagnosticCode("3002");
    pub const RESOLVER_SHADOWED_NAME: DiagnosticCode = DiagnosticCode("3003");
    pub const RESOLVER_DUPLICATE_DEFINITION: DiagnosticCode = DiagnosticCode("3004");

    // Type checker error codes (E4xxx)
    pub const TYPE_MISMATCH: DiagnosticCode = DiagnosticCode("4001");
    pub const TYPE_INFERENCE_FAILED: DiagnosticCode = DiagnosticCode("4002");
    pub const TYPE_UNDEFINED_TYPE: DiagnosticCode = DiagnosticCode("4003");
    pub const TYPE_INVALID_OP: DiagnosticCode = DiagnosticCode("4004");
    pub const TYPE_MISSING_ARGUMENT: DiagnosticCode = DiagnosticCode("4005");
    pub const TYPE_EXTRA_ARGUMENT: DiagnosticCode = DiagnosticCode("4006");
    pub const TYPE_EFFECT_MISMATCH: DiagnosticCode = DiagnosticCode("4007");
    pub const TYPE_EFFECT_REQUIRED: DiagnosticCode = DiagnosticCode("4008");
    pub const TYPE_INVALID_FIELD_ACCESS: DiagnosticCode = DiagnosticCode("4009");
    pub const TYPE_NOT_STRUCT: DiagnosticCode = DiagnosticCode("4010");

    // Borrow checker error codes (E5xxx)
    pub const BORROW_USE_AFTER_MOVE: DiagnosticCode = DiagnosticCode("5001");
    pub const BORROW_DOUBLE_DROP: DiagnosticCode = DiagnosticCode("5002");
    pub const BORROW_MUTABLE_BORROW_WHILE_SHARED: DiagnosticCode = DiagnosticCode("5003");
    pub const BORROW_UNINITIALIZED_USE: DiagnosticCode = DiagnosticCode("5004");
    pub const BORROW_INVALID_BORROW: DiagnosticCode = DiagnosticCode("5005");
    pub const BORROW_LINEAR_NOT_USED: DiagnosticCode = DiagnosticCode("5006");

    // Runtime error codes (E6xxx)
    pub const RUNTIME_DIVIDE_BY_ZERO: DiagnosticCode = DiagnosticCode("6001");
    pub const RUNTIME_INDEX_OUT_OF_BOUNDS: DiagnosticCode = DiagnosticCode("6002");
    pub const RUNTIME_PANIC: DiagnosticCode = DiagnosticCode("6003");
    pub const RUNTIME_STACK_OVERFLOW: DiagnosticCode = DiagnosticCode("6004");

    // Codegen error codes (E7xxx)
    pub const CODEGEN_UNSUPPORTED_FEATURE: DiagnosticCode = DiagnosticCode("7001");
    pub const CODEGEN_COMPILATION_FAILED: DiagnosticCode = DiagnosticCode("7002");
}
