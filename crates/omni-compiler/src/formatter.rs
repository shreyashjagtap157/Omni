use crate::ast::{Expr, InterpolatedFragment, Program, Stmt};
use crate::complete_lexer::TokenKind;
use crate::cst::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

#[derive(Debug, Clone, Default)]
pub struct FormatterConfig {
    pub strict_mode: bool,
}

impl FormatterConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }
}

pub fn check_format(source: &str) -> Result<(), String> {
    let tokens = crate::complete_lexer::tokenize_complete(source)
        .map_err(|e| format!("Lexer error: {}", e))?;
    let cst = crate::cst::build_cst(&tokens);
    let formatted = format_cst_source(&cst);
    if source == formatted {
        Ok(())
    } else {
        let mut diff = String::new();
        diff.push_str("Formatting would change the file:\n\n");
        diff.push_str("--- original\n");
        diff.push_str("+++ formatted\n");
        let original_lines: Vec<&str> = source.lines().collect();
        let formatted_lines: Vec<&str> = formatted.lines().collect();
        let max_len = original_lines.len().max(formatted_lines.len());
        for i in 0..max_len {
            let orig = original_lines.get(i).unwrap_or(&"");
            let fmt = formatted_lines.get(i).unwrap_or(&"");
            if orig != fmt {
                if i < original_lines.len() {
                    diff.push_str(&format!("- {}\n", orig));
                }
                if i < formatted_lines.len() {
                    diff.push_str(&format!("+ {}\n", fmt));
                }
            }
        }
        Err(diff)
    }
}

fn escape_string(s: &str) -> String {
    s.chars()
        .map(|ch| match ch {
            '\0' => "\\0".to_string(),
            '\t' => "\\t".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '"' => "\\\"".to_string(),
            '\\' => "\\\\".to_string(),
            other => other.to_string(),
        })
        .collect()
}

fn escape_char(ch: char) -> String {
    match ch {
        '\0' => "\\0".to_string(),
        '\t' => "\\t".to_string(),
        '\n' => "\\n".to_string(),
        '\r' => "\\r".to_string(),
        '\'' => "\\'".to_string(),
        '\\' => "\\\\".to_string(),
        other => other.to_string(),
    }
}

fn format_ident(name: &str) -> String {
    if crate::complete_lexer::CompleteLexer::is_keyword(name) {
        format!("r#{}", name)
    } else {
        name.to_string()
    }
}

fn format_visibility(visibility: &crate::ast::Visibility) -> String {
    match visibility {
        crate::ast::Visibility::Private => String::new(),
        crate::ast::Visibility::Pub => "pub ".to_string(),
        crate::ast::Visibility::PubMod => "pub(mod) ".to_string(),
        crate::ast::Visibility::PubPkg => "pub(pkg) ".to_string(),
        crate::ast::Visibility::PubCap(name) => format!("pub(cap: {}) ", name),
        crate::ast::Visibility::PubFriend(module) => format!("pub(friend: {}) ", module),
    }
}

fn format_contract_inline(contract: &Stmt) -> Option<String> {
    match contract {
        Stmt::ContractRequires {
            condition, message, ..
        } => {
            let message = if message.is_empty() {
                String::new()
            } else {
                format!(", \"{}\"", escape_string(message))
            };
            Some(format!("@requires({}{})", format_expr(condition), message))
        }
        Stmt::ContractEnsures {
            condition, message, ..
        } => {
            let message = if message.is_empty() {
                String::new()
            } else {
                format!(", \"{}\"", escape_string(message))
            };
            Some(format!("@ensures({}{})", format_expr(condition), message))
        }
        Stmt::ContractInvariant {
            condition, message, ..
        } => {
            let message = if message.is_empty() {
                String::new()
            } else {
                format!(", \"{}\"", escape_string(message))
            };
            Some(format!("@invariant({}{})", format_expr(condition), message))
        }
        Stmt::ComptimeLimit { max_ops, .. } => Some(format!("@comptime_limit(ops: {})", max_ops)),
        _ => None,
    }
}

fn format_type_params_list(type_params: &[(String, Vec<String>)]) -> String {
    type_params
        .iter()
        .map(|(name, bounds)| {
            if bounds.is_empty() {
                format_ident(name)
            } else {
                format!("{}: {}", format_ident(name), bounds.join(" + "))
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_expr(e: &Expr) -> String {
    match e {
        Expr::StringLit(s, _) => format!("\"{}\"", escape_string(s)),
        Expr::ByteString(bytes, _) => {
            let mut out = String::from("b\"");
            for byte in bytes {
                match *byte {
                    0 => out.push_str("\\0"),
                    b'\t' => out.push_str("\\t"),
                    b'\n' => out.push_str("\\n"),
                    b'\r' => out.push_str("\\r"),
                    b'\"' => out.push_str("\\\""),
                    b'\\' => out.push_str("\\\\"),
                    0x20..=0x7e => out.push(char::from(*byte)),
                    value => out.push_str(&format!("\\x{value:02X}")),
                }
            }
            out.push('\"');
            out
        }
        Expr::Byte(value, _) => match *value {
            0 => "b'\\0'".to_string(),
            b'\t' => "b'\\t'".to_string(),
            b'\n' => "b'\\n'".to_string(),
            b'\r' => "b'\\r'".to_string(),
            b'\'' => "b'\\\''".to_string(),
            b'\\' => "b'\\\\'".to_string(),
            0x20..=0x7e => format!("b'{}'", char::from(*value)),
            value => format!("b'\\x{value:02X}'"),
        },
        Expr::Number(n, _) => format!("{}", n),
        Expr::Float(n, _) => format!("{}", n),
        Expr::Char(c, _) => format!("'{}'", escape_char(*c)),
        Expr::Bool(b, _) => format!("{}", b),
        Expr::Var(name, _) => format_ident(name),
        Expr::Call(name, args, _) => {
            let inner: Vec<String> = args.iter().map(format_expr).collect();
            format!("{}({})", format_ident(name), inner.join(", "))
        }
        Expr::BinaryOp {
            op, left, right, ..
        } => {
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
        Expr::Borrow { mutable, inner, .. } => {
            if *mutable {
                format!("&mut {}", format_expr(inner))
            } else {
                format!("&{}", format_expr(inner))
            }
        }
        Expr::Deref { inner, .. } => format!("*{}", format_expr(inner)),
        Expr::UnaryOp { op, inner, .. } => {
            let op_str = match op {
                TokenKind::Minus => "-",
                TokenKind::Bang => "!",
                _ => "???",
            };
            format!("{}{}", op_str, format_expr(inner))
        }
        Expr::FieldAccess { base, field, .. } => {
            format!("{}.{}", format_expr(base), format_ident(field))
        }
        Expr::IfExpr {
            cond, then, else_, ..
        } => {
            format!(
                "if {} {} else {}",
                format_expr(cond),
                format_expr(then),
                format_expr(else_)
            )
        }
        Expr::Interpolated(frags, _) => {
            let mut out = String::new();
            for frag in frags.iter() {
                match frag {
                    InterpolatedFragment::Literal(s, _) => {
                        let escaped = escape_string(s).replace("${", "\\${");
                        out.push_str(&escaped);
                    }
                    InterpolatedFragment::Expr(e) => {
                        out.push_str(&format!("${{{}}}", format_expr(e)))
                    }
                }
            }
            format!("f\"{}\"", out)
        }
        Expr::Block(stmts, _) => {
            let inner: Vec<String> = stmts.iter().map(|s| format_stmt(s, 1)).collect();
            format!("{{ {} }}", inner.join(" "))
        }
        Expr::Tuple(exprs, _) => {
            let inner: Vec<String> = exprs.iter().map(format_expr).collect();
            format!("({})", inner.join(", "))
        }
        Expr::Array(exprs, _) => {
            let inner: Vec<String> = exprs.iter().map(format_expr).collect();
            format!("[{}]", inner.join(", "))
        }
        Expr::Index(base, index, _) => {
            format!("{}[{}]", format_expr(base), format_expr(index))
        }
        Expr::Match { expr, arms, .. } => {
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
        Expr::Range {
            start,
            end,
            inclusive,
            ..
        } => {
            let sep = if *inclusive { "..." } else { ".." };
            format!("{}{}{}", format_expr(start), sep, format_expr(end))
        }
        Expr::Lambda { params, body, .. } => {
            let params_str: Vec<String> = params
                .iter()
                .map(|(name, ty)| match ty {
                    Some(t) => format!("{}: {}", format_ident(name), t),
                    None => format_ident(name),
                })
                .collect();
            format!("fn({}) {}", params_str.join(", "), format_expr(body))
        }
        Expr::Await(inner, _) => format!("await {}", format_expr(inner)),
        Expr::Try(inner, _) => format!("{}?", format_expr(inner)),
        Expr::StructLit { name, fields, .. } => {
            let fields_str: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("{}: {}", format_ident(k), format_expr(v)))
                .collect();
            format!("{} {{ {} }}", format_ident(name), fields_str.join(", "))
        }
    }
}

fn format_pattern(pattern: &crate::ast::Pattern) -> String {
    match pattern {
        crate::ast::Pattern::Wildcard => "_".to_string(),
        crate::ast::Pattern::Literal(value) => value.to_string(),
        crate::ast::Pattern::Var(name) => format_ident(name),
        crate::ast::Pattern::Struct(name, fields) => {
            if fields.is_empty() {
                return format!("{}[]", format_ident(name));
            }
            let inner: Vec<String> = fields
                .iter()
                .map(|(field_name, field_pattern)| {
                    format!(
                        "{}: {}",
                        format_ident(field_name),
                        format_pattern(field_pattern)
                    )
                })
                .collect();
            format!("{}[{}]", format_ident(name), inner.join(", "))
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
        Stmt::Annotation(annot, _) => format!("{}@{}\n", pad, format_ident(annot)),
        Stmt::Mod(name, _) => format!("{}mod {};\n", pad, format_ident(name)),
        Stmt::ModBlock(name, body, _) => {
            let mut out = format!("{}mod {} {{\n", pad, format_ident(name));
            for stmt in body {
                out.push_str(&format_stmt(stmt, indent + 1));
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Stmt::Print(expr, _) => format!("{}print {};\n", pad, format_expr(expr)),
        Stmt::Let(name, type_ann, expr, _) => {
            let annotation = type_ann
                .as_ref()
                .map(|ty| format!(": {}", ty))
                .unwrap_or_default();
            format!(
                "{}let {}{} = {};\n",
                pad,
                format_ident(name),
                annotation,
                format_expr(expr)
            )
        }
        Stmt::LetMut(name, type_ann, expr, _) => {
            let annotation = type_ann
                .as_ref()
                .map(|ty| format!(": {}", ty))
                .unwrap_or_default();
            format!(
                "{}let mut {}{} = {};\n",
                pad,
                format_ident(name),
                annotation,
                format_expr(expr)
            )
        }
        Stmt::Fn {
            name,
            visibility,
            is_async,
            type_params,
            params,
            ret_type,
            effects,
            contracts,
            body,
            ..
        } => {
            let generic_suffix = if type_params.is_empty() {
                String::new()
            } else {
                format!("[{}]", format_type_params_list(type_params))
            };
            let params_str = params
                .iter()
                .map(|(name, type_annotation)| {
                    if let Some(ty) = type_annotation {
                        format!("{}: {}", format_ident(name), ty)
                    } else {
                        format_ident(name)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            let async_prefix = if *is_async { "async " } else { "" };
            let mut out = format!(
                "{}{}{}fn {}{}({})",
                pad,
                format_visibility(visibility),
                async_prefix,
                format_ident(name),
                generic_suffix,
                params_str
            );
            if let Some(ret_type) = ret_type {
                out.push_str(&format!(" -> {}", ret_type));
            }
            let visible_effects = effects
                .iter()
                .filter(|effect| !(*is_async && effect.as_str() == "async"))
                .cloned()
                .collect::<Vec<_>>();
            if !visible_effects.is_empty() {
                out.push_str(&format!(" / {}", visible_effects.join(" + ")));
            }
            for contract in contracts.iter().filter_map(format_contract_inline) {
                out.push(' ');
                out.push_str(&contract);
            }
            out.push_str(" {\n");
            for stmt in body {
                out.push_str(&format_stmt(stmt, indent + 1));
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Stmt::ExprStmt(expr, _) => format!("{}{};\n", pad, format_expr(expr)),
        Stmt::Block(inner, _) => {
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
            ..
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
        Stmt::Loop { body, .. } => {
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
            ..
        } => {
            let mut out = format!(
                "{}for {} in {} {{\n",
                pad,
                format_ident(var_name),
                format_expr(iterable)
            );
            for stmt in body {
                out.push_str(&format_stmt(stmt, indent + 1));
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Stmt::While { cond, body, .. } => {
            let mut out = format!("{}while {} {{\n", pad, format_expr(cond));
            for stmt in body {
                out.push_str(&format_stmt(stmt, indent + 1));
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Stmt::Return(expr, _) => format!("{}return {};\n", pad, format_expr(expr)),
        Stmt::Break(_) => format!("{}break;\n", pad),
        Stmt::Continue(_) => format!("{}continue;\n", pad),
        Stmt::Assign(name, expr, _) => {
            format!("{}{} = {};\n", pad, format_ident(name), format_expr(expr))
        }
        Stmt::ExprFieldAssign(base, field, expr, _) => {
            format!(
                "{}{}.{} = {};\n",
                pad,
                format_expr(base),
                field,
                format_expr(expr)
            )
        }
        Stmt::DerefAssign(reference, expr, _) => {
            format!(
                "{}*{} = {};\n",
                pad,
                format_expr(reference),
                format_expr(expr)
            )
        }
        Stmt::WhileIn {
            var_name,
            iterable,
            body,
            ..
        } => {
            let mut out = format!(
                "{}while {} in {} {{\n",
                pad,
                format_ident(var_name),
                format_expr(iterable)
            );
            for stmt in body {
                out.push_str(&format_stmt(stmt, indent + 1));
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Stmt::Unsafe { body, .. } => {
            let mut out = format!("{}unsafe {{\n", pad);
            for stmt in body {
                out.push_str(&format_stmt(stmt, indent + 1));
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Stmt::LetLinear(name, type_ann, expr, _) => {
            let annotation = type_ann
                .as_ref()
                .map(|ty| format!(": {}", ty))
                .unwrap_or_default();
            format!(
                "{}linear {}{} = {};\n",
                pad,
                format_ident(name),
                annotation,
                format_expr(expr)
            )
        }
        Stmt::Struct {
            name,
            visibility,
            fields,
            is_linear,
            ..
        } => {
            let mut out = format!(
                "{}{}struct {}",
                pad,
                format_visibility(visibility),
                format_ident(name)
            );
            if *is_linear {
                out.push_str(" linear");
            }
            out.push_str(" {\n");
            for (field_name, field_type) in fields {
                out.push_str(&format!(
                    "{}    {}: {}\n",
                    pad,
                    format_ident(field_name),
                    field_type
                ));
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Stmt::ErrorSet {
            name,
            visibility,
            variants,
            ..
        } => {
            let mut out = format!(
                "{}{}error set {} {{\n",
                pad,
                format_visibility(visibility),
                format_ident(name)
            );
            for variant in variants {
                out.push_str(&format!(
                    "{}    variant {}\n",
                    pad,
                    format_ident(&variant.name)
                ));
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Stmt::Impl {
            target,
            visibility,
            type_params,
            for_type,
            methods,
            ..
        } => {
            let mut out = format!(
                "{}{}impl {}",
                pad,
                format_visibility(visibility),
                format_ident(target)
            );
            if !type_params.is_empty() {
                out.push('[');
                out.push_str(&format_type_params_list(type_params));
                out.push(']');
            }
            if let Some(for_ty) = for_type {
                out.push_str(&format!(" for {}", format_ident(for_ty)));
            }
            out.push_str(" {\n");
            for method in methods {
                out.push_str(&format_stmt(method, indent + 1));
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Stmt::Trait {
            name,
            visibility,
            type_params,
            methods,
            diagnostic_attrs,
            ..
        } => {
            let mut out = format!(
                "{}{}trait {}",
                pad,
                format_visibility(visibility),
                format_ident(name)
            );
            if !type_params.is_empty() {
                out.push('[');
                out.push_str(&format_type_params_list(type_params));
                out.push(']');
            }
            out.push_str(" {\n");
            for attr in diagnostic_attrs {
                out.push_str(&format!(
                    "{}    @diagnostic::on_unimplemented(message = \"{}\"",
                    pad,
                    escape_string(&attr.message)
                ));
                if let Some(label) = &attr.label {
                    out.push_str(&format!(", label = \"{}\"", escape_string(label)));
                }
                out.push_str(")\n");
            }
            for method in methods {
                out.push_str(&format_stmt(method, indent + 1));
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Stmt::TypeAlias {
            name,
            visibility,
            type_params,
            target,
            ..
        } => {
            let mut out = format!(
                "{}{}type {}",
                pad,
                format_visibility(visibility),
                format_ident(name)
            );
            if !type_params.is_empty() {
                out.push('[');
                out.push_str(&format_type_params_list(type_params));
                out.push(']');
            }
            out.push_str(" = ");
            out.push_str(target);
            out.push_str(";\n");
            out
        }
        Stmt::Use { path, alias, .. } => {
            let mut out = format!("{}use {}", pad, path);
            if let Some(a) = alias {
                out.push_str(" as ");
                out.push_str(a);
            }
            out.push_str(";\n");
            out
        }
        Stmt::GcMode { mode, .. } => {
            format!("{}@gc_mode({})\n", pad, mode)
        }
        Stmt::CancelToken { .. } => {
            format!("{}@cancel_token\n", pad)
        }
        Stmt::EffectHandler {
            effect, handler, ..
        } => {
            let mut out = format!("{}handle {} in {{\n", pad, effect);
            match handler.as_ref() {
                Expr::Block(body, _) => {
                    for stmt in body {
                        out.push_str(&format_stmt(stmt, indent + 1));
                    }
                }
                expression => {
                    out.push_str(&format!("{}    {}\n", pad, format_expr(expression)));
                }
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Stmt::Spawn { task, .. } => {
            format!("{}spawn {}\n", pad, format_expr(task))
        }
        Stmt::Channel {
            elem_type,
            capacity,
            ..
        } => {
            let cap_str = capacity.map(|c| format!("[{}]", c)).unwrap_or_default();
            format!("{}channel {}{}\n", pad, elem_type, cap_str)
        }
        Stmt::Actor { name, state, .. } => {
            format!("{}actor {} {}\n", pad, format_ident(name), state)
        }
        Stmt::WorkStealingExecutor {
            num_threads,
            queue_type,
            ..
        } => {
            format!("{}executor[{}] {}\n", pad, num_threads, queue_type)
        }
        Stmt::DeterministicRuntime { max_tasks, .. } => {
            format!("{}deterministic[{}]\n", pad, max_tasks)
        }
        Stmt::Tensor { shape, dtype, .. } => {
            let shape_str = shape
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join("x");
            format!("{}tensor[{}] {}\n", pad, shape_str, dtype)
        }
        Stmt::Simd {
            width, elem_type, ..
        } => {
            format!("{}simd[{}] {}\n", pad, width, elem_type)
        }
        Stmt::DocComment {
            target, content, ..
        } => {
            format!("{}doc {} \"{}\"\n", pad, target, content)
        }
        Stmt::DebugSession { port, .. } => {
            format!("{}debug[{}]\n", pad, port)
        }
        Stmt::Capability {
            name, permissions, ..
        } => {
            format!(
                "{}capability {} [{}]\n",
                pad,
                format_ident(name),
                permissions.join(", ")
            )
        }
        Stmt::FfiSandbox { allow_list, .. } => {
            format!("{}sandbox [{}]\n", pad, allow_list.join(", "))
        }
        Stmt::Enum {
            name,
            visibility,
            variants,
            is_sealed,
            ..
        } => {
            let mut out = format!(
                "{}{}enum {}",
                pad,
                format_visibility(visibility),
                format_ident(name)
            );
            if *is_sealed {
                out.push_str(" sealed");
            }
            out.push_str(" {\n");
            for variant in variants {
                out.push_str(&format!(
                    "{}    variant {}",
                    pad,
                    format_ident(&variant.name)
                ));
                if !variant.fields.is_empty() {
                    let fields: Vec<String> = variant
                        .fields
                        .iter()
                        .map(|(field_name, field_type)| {
                            format!("{}: {}", format_ident(field_name), field_type)
                        })
                        .collect();
                    out.push_str(&format!(" [{}]", fields.join(", ")));
                }
                out.push('\n');
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Stmt::UseScoped {
            path,
            aliases,
            body,
            ..
        } => {
            let mut out = format!("{}use {} in", pad, path);
            if !aliases.is_empty() {
                let alias_str = aliases
                    .iter()
                    .map(|(name, alias)| {
                        if let Some(a) = alias {
                            format!("{} as {}", name, a)
                        } else {
                            name.clone()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!(" [{}]", alias_str));
            }
            out.push_str(":\n");
            for stmt in body {
                out.push_str(&format_stmt(stmt, indent + 1));
            }
            out
        }
        contract @ (Stmt::ContractRequires { .. }
        | Stmt::ContractEnsures { .. }
        | Stmt::ContractInvariant { .. }
        | Stmt::ComptimeLimit { .. }) => {
            let rendered = format_contract_inline(contract)
                .expect("contract variants always have a canonical formatter");
            format!("{}{}\n", pad, rendered)
        }
        &Stmt::Defer { .. } | &Stmt::AsyncDefer { .. } => format!("{}defer {{ ... }}\n", pad),
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
            out.push_str("//");
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
            out.push_str("/*");
            out.push_str(&t.text);
            out.push_str("*/");
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

pub fn format_program_with_config(prog: &Program, config: &FormatterConfig) -> String {
    if config.strict_mode {
        format_program_strict(prog)
    } else {
        format_program(prog)
    }
}

fn format_program_strict(prog: &Program) -> String {
    let mut use_stmts: Vec<&Stmt> = Vec::new();
    let mut other_stmts: Vec<&Stmt> = Vec::new();
    let mut doc_comments: Vec<&Stmt> = Vec::new();

    for stmt in &prog.stmts {
        match stmt {
            Stmt::Use { .. } | Stmt::UseScoped { .. } => use_stmts.push(stmt),
            Stmt::DocComment { .. } => doc_comments.push(stmt),
            _ => other_stmts.push(stmt),
        }
    }

    use_stmts.sort_by(|a, b| {
        let path_a = match a {
            Stmt::Use { path, .. } => path.clone(),
            Stmt::UseScoped { path, .. } => path.clone(),
            _ => String::new(),
        };
        let path_b = match b {
            Stmt::Use { path, .. } => path.clone(),
            Stmt::UseScoped { path, .. } => path.clone(),
            _ => String::new(),
        };
        path_a.cmp(&path_b)
    });

    let mut out = String::new();

    for stmt in &doc_comments {
        out.push_str(&format_stmt(stmt, 0));
    }

    if !doc_comments.is_empty() && !use_stmts.is_empty() {
        out.push('\n');
    }

    for stmt in &use_stmts {
        out.push_str(&format_stmt(stmt, 0));
    }

    if !use_stmts.is_empty() && !other_stmts.is_empty() {
        out.push('\n');
    }

    for stmt in &other_stmts {
        out.push_str(&format_stmt(stmt, 0));
    }

    out
}

pub fn format_cst_source_with_config(node: &SyntaxNode, config: &FormatterConfig) -> String {
    let base = format_cst_source(node);
    if config.strict_mode {
        base.lines()
            .map(|line| line.trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    } else {
        base
    }
}
