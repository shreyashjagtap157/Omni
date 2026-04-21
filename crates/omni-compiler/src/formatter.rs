use crate::ast::{Expr, InterpolatedFragment, Program, Stmt};
use crate::cst::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};
use crate::lexer::TokenKind;

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn format_expr(e: &Expr) -> String {
    match e {
        Expr::StringLit(s) => format!("\"{}\"", escape_string(s)),
        Expr::Number(n) => format!("{}", n),
        Expr::Bool(b) => format!("{}", b),
        Expr::Var(name) => name.clone(),
        Expr::Call(name, args) => {
            let inner: Vec<String> = args.iter().map(format_expr).collect();
            format!("{}({})", name, inner.join(", "))
        }
        Expr::BinaryOp { op, left, right } => {
            let op_str = match op {
                TokenKind::Plus => "+",
                TokenKind::Minus => "-",
                TokenKind::Star => "*",
                TokenKind::Slash => "/",
                TokenKind::Percent => "%",
                TokenKind::EqEq => "==",
                TokenKind::NotEq => "!=",
                TokenKind::Lt => "<",
                TokenKind::LtEq => "<=",
                TokenKind::Gt => ">",
                TokenKind::GtEq => ">=",
                TokenKind::AndAnd => "&&",
                TokenKind::OrOr => "||",
                _ => "???",
            };
            format!("{} {} {}", format_expr(left), op_str, format_expr(right))
        }
        Expr::UnaryOp { op, inner } => {
            let op_str = match op {
                TokenKind::Minus => "-",
                TokenKind::Bang => "!",
                _ => "???",
            };
            format!("{}{}", op_str, format_expr(inner))
        }
        Expr::FieldAccess { base, field } => {
            format!("{}.{}", format_expr(base), field)
        }
        Expr::IfExpr { cond, then, else_ } => {
            format!(
                "if {} {} else {}",
                format_expr(cond),
                format_expr(then),
                format_expr(else_)
            )
        }
        Expr::Interpolated(frags) => {
            let mut out = String::new();
            for frag in frags.iter() {
                match frag {
                    InterpolatedFragment::Literal(s) => out.push_str(&escape_string(s)),
                    InterpolatedFragment::Expr(e) => {
                        out.push_str(&format!("{{{}}}", format_expr(e)))
                    }
                }
            }
            format!("\"{}\"", out)
        }
        Expr::Block(stmts) => {
            let inner: Vec<String> = stmts.iter().map(|s| format_stmt(s, 1)).collect();
            format!("{{ {} }}", inner.join(" "))
        }
        Expr::Tuple(exprs) => {
            let inner: Vec<String> = exprs.iter().map(format_expr).collect();
            format!("({})", inner.join(", "))
        }
        Expr::Index(base, index) => {
            format!("{}[{}]", format_expr(base), format_expr(index))
        }
        Expr::Match { expr, arms } => {
            let mut out = format!("match {} {{\n", format_expr(expr));
            for arm in arms {
                out.push_str("  | ");
                out.push_str(&format_pattern(&arm.pattern));
                if let Some(guard) = &arm.guard {
                    out.push_str(&format!(" if {}", format_expr(guard)));
                }
                out.push_str(&format!(" => {}\n", format_expr(&arm.body)));
            }
            out.push_str("}\n");
            out
        }
    }
}

fn format_pattern(pattern: &crate::ast::Pattern) -> String {
    match pattern {
        crate::ast::Pattern::Wildcard => "_".to_string(),
        crate::ast::Pattern::Literal(value) => value.to_string(),
        crate::ast::Pattern::Var(name) => name.clone(),
        crate::ast::Pattern::Struct(name, fields) => {
            if fields.is_empty() {
                return name.to_string();
            }
            let inner: Vec<String> = fields
                .iter()
                .map(|(field_name, field_pattern)| {
                    format!("{}: {}", field_name, format_pattern(field_pattern))
                })
                .collect();
            format!("{}[{}]", name, inner.join(", "))
        }
        crate::ast::Pattern::Or(patterns) => patterns
            .iter()
            .map(format_pattern)
            .collect::<Vec<_>>()
            .join(" | "),
    }
}

fn format_stmt(s: &Stmt, indent: usize) -> String {
    let pad = " ".repeat(indent * 4);
    match s {
        Stmt::Print(expr) => format!("{}print {}\n", pad, format_expr(expr)),
        Stmt::Let(name, expr) => format!("{}let {} = {}\n", pad, name, format_expr(expr)),
        Stmt::Fn {
            name,
            type_params,
            params,
            ret_type,
            effects,
            body,
        } => {
            let generic_suffix = if type_params.is_empty() {
                String::new()
            } else {
                format!("<{}>", type_params.join(", "))
            };
            let mut out = format!(
                "{}fn {}{}({})",
                pad,
                name,
                generic_suffix,
                params.join(", ")
            );
            if let Some(ret_type) = ret_type {
                out.push_str(&format!(" -> {}", ret_type));
            }
            if !effects.is_empty() {
                out.push_str(&format!(" / {}", effects.join(" + ")));
            }
            out.push_str(" {\n");
            for stmt in body {
                out.push_str(&format_stmt(stmt, indent + 1));
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Stmt::ExprStmt(expr) => format!("{}{}\n", pad, format_expr(expr)),
        Stmt::Block(inner) => {
            let mut out = String::new();
            for stmt in inner {
                out.push_str(&format_stmt(stmt, indent));
            }
            out
        }
        Stmt::If {
            cond,
            then_body,
            else_body,
        } => {
            let mut out = format!("{}if {} {{\n", pad, format_expr(cond));
            for stmt in then_body {
                out.push_str(&format_stmt(stmt, indent + 1));
            }
            out.push_str(&format!("{}}} else {{\n", pad));
            for stmt in else_body {
                out.push_str(&format_stmt(stmt, indent + 1));
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Stmt::Loop { body } => {
            let mut out = format!("{}loop {{\n", pad);
            for stmt in body {
                out.push_str(&format_stmt(stmt, indent + 1));
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Stmt::For {
            var_name,
            iterable,
            body,
        } => {
            let mut out = format!("{}for {} in {} {{\n", pad, var_name, format_expr(iterable));
            for stmt in body {
                out.push_str(&format_stmt(stmt, indent + 1));
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Stmt::While { cond, body } => {
            let mut out = format!("{}while {} {{\n", pad, format_expr(cond));
            for stmt in body {
                out.push_str(&format_stmt(stmt, indent + 1));
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Stmt::Return(expr) => format!("{}return {}\n", pad, format_expr(expr)),
        Stmt::Break => format!("{}break\n", pad),
        Stmt::Continue => format!("{}continue\n", pad),
        Stmt::Assign(name, expr) => format!("{}let {} = {}\n", pad, name, format_expr(expr)),
        Stmt::ExprFieldAssign(base, field, expr) => {
            format!(
                "{}{}.{} = {}\n",
                pad,
                format_expr(base),
                field,
                format_expr(expr)
            )
        }
        Stmt::WhileIn {
            var_name,
            iterable,
            body,
        } => {
            let mut out = format!("{}for {} in {} {{\n", pad, var_name, format_expr(iterable));
            for stmt in body {
                out.push_str(&format_stmt(stmt, indent + 1));
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Stmt::Unsafe { body } => {
            let mut out = format!("{}unsafe {{\n", pad);
            for stmt in body {
                out.push_str(&format_stmt(stmt, indent + 1));
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Stmt::LetLinear(name, expr) => format!("{}linear {} = {}\n", pad, name, format_expr(expr)),
        Stmt::Struct {
            name,
            fields,
            is_linear,
        } => {
            let mut out = format!("{}struct {}", pad, name);
            if *is_linear {
                out.push_str(" linear");
            }
            out.push_str(" {\n");
            for (field_name, field_type) in fields {
                out.push_str(&format!("{}  {}: {}\n", pad, field_name, field_type));
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Stmt::Enum {
            name,
            variants,
            is_sealed,
        } => {
            let mut out = format!("{}enum {}", pad, name);
            if *is_sealed {
                out.push_str(" sealed");
            }
            out.push_str(" {\n");
            for variant in variants {
                out.push_str(&format!("{}  variant {}", pad, variant.name));
                if !variant.fields.is_empty() {
                    let fields: Vec<String> = variant
                        .fields
                        .iter()
                        .map(|(field_name, field_type)| format!("{}: {}", field_name, field_type))
                        .collect();
                    out.push_str(&format!(" [{}]", fields.join(", ")));
                }
                out.push('\n');
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
    }
}

pub fn format_program(prog: &Program) -> String {
    let mut out = String::new();
    for stmt in &prog.stmts {
        out.push_str(&format_stmt(stmt, 0));
    }
    out
}

fn ensure_space(out: &mut String) {
    if out.is_empty() {
        return;
    }
    let last = out.chars().next_back();
    if let Some(c) = last {
        if c == '\n' || c.is_whitespace() {
            return;
        }
    }
    out.push(' ');
}

fn write_token(t: &SyntaxToken, indent_level: usize, out: &mut String, on_newline: &mut bool) {
    if *on_newline {
        out.push_str(&" ".repeat(indent_level * 4));
        *on_newline = false;
    }

    match t.kind {
        SyntaxKind::TokenIdent
        | SyntaxKind::TokenNumber
        | SyntaxKind::TokenOther
        | SyntaxKind::TokenEquals => {
            ensure_space(out);
            out.push_str(&t.text);
        }
        SyntaxKind::TokenString => {
            ensure_space(out);
            out.push_str(&format!("\"{}\"", escape_string(&t.text)));
        }
        SyntaxKind::TokenCommentLine => {
            if *on_newline {
                out.push_str(&" ".repeat(indent_level * 4));
            }
            out.push_str("--");
            out.push_str(&t.text);
            out.push('\n');
            *on_newline = true;
        }
        SyntaxKind::TokenDocComment => {
            if *on_newline {
                out.push_str(&" ".repeat(indent_level * 4));
            }
            out.push_str("///");
            out.push_str(&t.text);
            out.push('\n');
            *on_newline = true;
        }
        SyntaxKind::TokenCommentBlock => {
            if *on_newline {
                out.push_str(&" ".repeat(indent_level * 4));
            }
            out.push_str("---");
            out.push_str(&t.text);
            out.push_str("---");
            out.push('\n');
            *on_newline = true;
        }
        SyntaxKind::TokenNewline => {
            out.push('\n');
            *on_newline = true;
        }
        SyntaxKind::TokenIndent | SyntaxKind::TokenDedent => {}
        _ => {
            ensure_space(out);
            out.push_str(&t.text);
        }
    }
}

fn format_element(
    elem: &SyntaxElement,
    indent_level: usize,
    out: &mut String,
    on_newline: &mut bool,
) {
    match elem {
        SyntaxElement::Token(t) => write_token(t, indent_level, out, on_newline),
        SyntaxElement::Node(n) => match n.kind {
            SyntaxKind::Root => {
                for c in &n.children {
                    format_element(c, indent_level, out, on_newline);
                }
            }
            SyntaxKind::Block => {
                for c in &n.children {
                    format_element(c, indent_level + 1, out, on_newline);
                }
            }
            SyntaxKind::Statement => {
                for c in &n.children {
                    format_element(c, indent_level, out, on_newline);
                }
            }
            _ => {
                for c in &n.children {
                    format_element(c, indent_level, out, on_newline);
                }
            }
        },
    }
}

pub fn format_cst_source(node: &SyntaxNode) -> String {
    let mut out = String::new();
    let mut on_newline = true;
    for child in &node.children {
        format_element(child, 0, &mut out, &mut on_newline);
    }
    out
}
