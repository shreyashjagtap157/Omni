use crate::ast::Stmt;
use std::collections::HashMap;

pub struct DocItem {
    pub name: String,
    pub kind: String,
    pub docs: String,
    pub signature: String,
    pub effects: Vec<String>,
}

/// Generate the minimal bootstrap API documentation model.
///
/// Documentation comments are associated by their parsed target name. Missing
/// documentation stays empty; the generator never fabricates placeholder text.
pub fn generate_docs(ast: &[Stmt]) -> Vec<DocItem> {
    let mut docs_by_target: HashMap<String, String> = HashMap::new();
    for stmt in ast {
        if let Stmt::DocComment {
            target, content, ..
        } = stmt
        {
            docs_by_target
                .entry(target.clone())
                .and_modify(|existing| {
                    if !existing.is_empty() && !content.is_empty() {
                        existing.push('\n');
                    }
                    existing.push_str(content.trim());
                })
                .or_insert_with(|| content.trim().to_string());
        }
    }

    let mut items = Vec::new();
    for stmt in ast {
        match stmt {
            Stmt::Fn {
                name,
                effects,
                params,
                ret_type,
                ..
            } => {
                let params_str = params
                    .iter()
                    .map(|(n, t)| match t {
                        Some(ty) => format!("{}: {}", n, ty),
                        None => n.clone(),
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let ret_str = ret_type
                    .as_ref()
                    .map(|r| format!(" -> {}", r))
                    .unwrap_or_default();
                items.push(DocItem {
                    name: name.clone(),
                    kind: "Function".to_string(),
                    docs: docs_by_target.get(name).cloned().unwrap_or_default(),
                    signature: format!("fn {}({}){}", name, params_str, ret_str),
                    effects: effects.clone(),
                });
            }
            Stmt::Struct {
                name, is_linear, ..
            } => {
                items.push(DocItem {
                    name: name.clone(),
                    kind: if *is_linear {
                        "Linear Struct".to_string()
                    } else {
                        "Struct".to_string()
                    },
                    docs: docs_by_target.get(name).cloned().unwrap_or_default(),
                    signature: format!("struct {}", name),
                    effects: vec![],
                });
            }
            _ => {}
        }
    }
    items
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn generate_html(items: &[DocItem]) -> String {
    let mut html = String::from(
        "<html><head><title>Omni Documentation</title><style>\n\
         body { font-family: sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }\n\
         .item { border: 1px solid #ddd; padding: 15px; margin-bottom: 20px; border-radius: 5px; }\n\
         .signature { font-family: monospace; font-size: 1.1em; background: #f4f4f4; padding: 5px; }\n\
         .effect { display: inline-block; background: #e0e0ff; color: #333; padding: 2px 5px; margin: 2px; border-radius: 3px; font-size: 0.8em; }\n\
         </style></head><body>",
    );
    html.push_str("<h1>Omni API Documentation</h1>");

    for item in items {
        html.push_str(&format!(
            "<div class=\"item\"><h2>{} <span style=\"font-size: 0.6em; color: #666;\">({})</span></h2>",
            escape_html(&item.name),
            escape_html(&item.kind)
        ));
        html.push_str(&format!(
            "<div class=\"signature\">{}</div>",
            escape_html(&item.signature)
        ));
        if !item.effects.is_empty() {
            html.push_str("<div style=\"margin-top: 10px;\"><strong>Effects:</strong>");
            for eff in &item.effects {
                html.push_str(&format!(
                    "<span class=\"effect\">{}</span>",
                    escape_html(eff)
                ));
            }
            html.push_str("</div>");
        }
        if !item.docs.is_empty() {
            html.push_str(&format!("<p>{}</p>", escape_html(&item.docs)));
        }
        html.push_str("</div>");
    }

    html.push_str("</body></html>");
    html
}
