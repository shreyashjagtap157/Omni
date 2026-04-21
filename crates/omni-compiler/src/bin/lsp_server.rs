use std::io::{BufRead, BufReader, Read, Write};

use serde_json::{json, Value};

#[cfg(feature = "use_salsa_lsp")]
use omni_compiler::lsp_salsa_db::LspDb as ServerDb;
#[cfg(not(feature = "use_salsa_lsp"))]
use omni_compiler::lsp::CompilationDatabase as ServerDb;

fn uri_to_path(uri: &str) -> String {
    if uri.starts_with("file://") {
        let mut s = uri.trim_start_matches("file://").to_string();
        // On Windows the URL may be like file:///C:/path; strip leading slash
        if s.starts_with('/') && s.chars().nth(2) == Some(':') {
            s = s.trim_start_matches('/').to_string();
        }
        s
    } else {
        uri.to_string()
    }
}

fn read_message<R: Read>(reader: &mut BufReader<R>) -> Option<Value> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut header = String::new();
        match reader.read_line(&mut header) {
            Ok(0) => return None,
            Ok(_) => {
                let line = header.trim_end();
                if line.is_empty() {
                    break;
                }
                if let Some(colon) = line.find(':') {
                    let key = &line[..colon].to_ascii_lowercase();
                    let val = line[colon + 1..].trim();
                    if key == "content-length" {
                        if let Ok(n) = val.parse::<usize>() {
                            content_length = Some(n);
                        }
                    }
                }
            }
            Err(_) => return None,
        }
    }

    let len = content_length?;
    let mut buf = vec![0u8; len];
    if reader.read_exact(&mut buf).is_err() {
        return None;
    }
    serde_json::from_slice(&buf).ok()
}

fn send_response<W: Write>(out: &mut W, id: &Value, result: Value) -> std::io::Result<()> {
    let body = json!({"jsonrpc": "2.0", "id": id, "result": result});
    let s = body.to_string();
    write!(out, "Content-Length: {}\r\n\r\n{}", s.len(), s)?;
    out.flush()
}

fn send_error<W: Write>(out: &mut W, id: &Value, code: i64, message: &str) -> std::io::Result<()> {
    let body = json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}});
    let s = body.to_string();
    write!(out, "Content-Length: {}\r\n\r\n{}", s.len(), s)?;
    out.flush()
}

fn completion_kind_to_lsp_kind(kind: omni_compiler::lsp::CompletionKind) -> i64 {
    match kind {
        omni_compiler::lsp::CompletionKind::Keyword => 14,
        omni_compiler::lsp::CompletionKind::Function => 3,
        omni_compiler::lsp::CompletionKind::Variable => 6,
        omni_compiler::lsp::CompletionKind::Type => 7,
        omni_compiler::lsp::CompletionKind::Field => 5,
    }
}

fn main() {
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    let mut db = ServerDb::new();

    while let Some(msg) = read_message(&mut reader) {
        if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
            match method {
                "initialize" => {
                    let id = msg.get("id").cloned().unwrap_or(Value::Null);
                    let caps = json!({
                        "capabilities": {
                            "hoverProvider": true,
                            "definitionProvider": true,
                            "completionProvider": {
                                "resolveProvider": false,
                                "triggerCharacters": ["."]
                            },
                            "inlayHintProvider": true,
                            "textDocumentSync": 1
                        }
                    });
                    let _ = send_response(&mut out, &id, caps);
                }
                "initialized" => {
                    // no-op
                }
                "textDocument/didOpen" => {
                    if let Some(params) = msg.get("params") {
                        if let Some(doc) = params.get("textDocument") {
                            let uri = doc.get("uri").and_then(|v| v.as_str()).unwrap_or("");
                            let text = doc.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let version = doc.get("version").and_then(|v| v.as_i64()).unwrap_or(1) as usize;
                            let path = uri_to_path(uri);
                            db.add_source(path, text, version);
                        }
                    }
                }
                "textDocument/hover" => {
                    let id = msg.get("id").cloned().unwrap_or(Value::Null);
                    if let Some(params) = msg.get("params") {
                        let uri = params
                            .get("textDocument")
                            .and_then(|d| d.get("uri"))
                            .and_then(|u| u.as_str())
                            .unwrap_or("");
                        let pos = params.get("position").unwrap_or(&Value::Null);
                        let line = pos.get("line").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
                        let col = pos.get("character").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
                        let path = uri_to_path(uri);
                        // lsp.rs expects 1-based line numbers
                        if let Some(q) = db.hover_at(&path, line + 1, col) {
                            let contents = q.text;
                            let result = json!({"contents": {"kind": "plaintext", "value": contents}});
                            let _ = send_response(&mut out, &id, result);
                        } else {
                            let result = Value::Null;
                            let _ = send_response(&mut out, &id, result);
                        }
                    } else {
                        let id = msg.get("id").cloned().unwrap_or(Value::Null);
                        let _ = send_error(&mut out, &id, -32602, "Invalid params");
                    }
                }
                "textDocument/definition" => {
                    let id = msg.get("id").cloned().unwrap_or(Value::Null);
                    if let Some(params) = msg.get("params") {
                        let uri = params
                            .get("textDocument")
                            .and_then(|d| d.get("uri"))
                            .and_then(|u| u.as_str())
                            .unwrap_or("");
                        let pos = params.get("position").unwrap_or(&Value::Null);
                        let line = pos.get("line").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
                        let col = pos.get("character").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
                        let path = uri_to_path(uri);
                        if let Some((def_path, span)) = db.goto_definition(&path, line + 1, col) {
                            // Convert to LSP Location
                            let uri = if def_path.starts_with('/') || def_path.chars().nth(1) == Some(':') {
                                format!("file://{}", def_path)
                            } else {
                                def_path.clone()
                            };
                            let loc = json!({
                                "uri": uri,
                                "range": {
                                    "start": { "line": span.start_line.saturating_sub(1), "character": span.start_col },
                                    "end": { "line": span.end_line.saturating_sub(1), "character": span.end_col }
                                }
                            });
                            let _ = send_response(&mut out, &id, loc);
                        } else {
                            let _ = send_response(&mut out, &id, Value::Null);
                        }
                    }
                }
                "textDocument/completion" => {
                    let id = msg.get("id").cloned().unwrap_or(Value::Null);
                    if let Some(params) = msg.get("params") {
                        let uri = params
                            .get("textDocument")
                            .and_then(|d| d.get("uri"))
                            .and_then(|u| u.as_str())
                            .unwrap_or("");
                        let pos = params.get("position").unwrap_or(&Value::Null);
                        let line = pos.get("line").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
                        let col = pos.get("character").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
                        let path = uri_to_path(uri);
                        let items = db.get_completions(&path, line + 1, col);
                        let result: Vec<_> = items
                            .into_iter()
                            .map(|it| {
                                json!({
                                    "label": it.label,
                                    "kind": completion_kind_to_lsp_kind(it.kind),
                                    "detail": it.detail
                                })
                            })
                            .collect();
                        let _ = send_response(&mut out, &id, json!(result));
                    } else {
                        let _ = send_error(&mut out, &id, -32602, "Invalid params");
                    }
                }
                "textDocument/inlayHint" | "textDocument/inlayHints" => {
                    let id = msg.get("id").cloned().unwrap_or(Value::Null);
                    if let Some(params) = msg.get("params") {
                        let uri = params
                            .get("textDocument")
                            .and_then(|d| d.get("uri"))
                            .and_then(|u| u.as_str())
                            .unwrap_or("");
                        let path = uri_to_path(uri);
                        let hints = db.get_inlay_hints(&path);
                        // Map to simple JSON objects
                        let jh: Vec<_> = hints
                            .into_iter()
                            .map(|h| json!({"text": h.text, "line": h.span.start_line.saturating_sub(1), "start": h.span.start_col, "end": h.span.end_col }))
                            .collect();
                        let _ = send_response(&mut out, &id, json!(jh));
                    }
                }
                "omni/borrowVisualization" => {
                    let id = msg.get("id").cloned().unwrap_or(Value::Null);
                    if let Some(params) = msg.get("params") {
                        let uri = params
                            .get("textDocument")
                            .and_then(|d| d.get("uri"))
                            .and_then(|u| u.as_str())
                            .unwrap_or("");
                        let path = uri_to_path(uri);
                        let viz = db.get_borrow_visualization(&path);
                        // Serialize minimal borrow info
                        let borrows: Vec<_> = viz
                            .borrows
                            .into_iter()
                            .map(|b| json!({"variable": b.variable, "start": b.borrow_span.start_line.saturating_sub(1), "end": b.borrow_span.end_line, "kind": format!("{:?}", b.kind), "is_valid": b.is_valid}))
                            .collect();
                        let issues: Vec<_> = viz
                            .issues
                            .into_iter()
                            .map(|i| json!({"severity": format!("{:?}", i.severity), "message": i.message, "spans": i.spans.into_iter().map(|s| json!({"line": s.start_line.saturating_sub(1), "col": s.start_col})).collect::<Vec<_>>() }))
                            .collect();
                        let _ = send_response(&mut out, &id, json!({"borrows": borrows, "issues": issues}));
                    }
                }
                "shutdown" => {
                    let id = msg.get("id").cloned().unwrap_or(Value::Null);
                    let _ = send_response(&mut out, &id, Value::Null);
                }
                _ => {
                    // Unhandled method - ignore or send empty reply for requests
                }
            }
        }
    }
}
