use std::collections::HashMap;

use crate::lexer::Lexer;
use crate::parser::Parser;

/// A small, conservative incremental database used by the LSP server for
/// quick, in-memory analysis. This is intentionally lightweight and does not
/// require `salsa` — it caches per-file analysis and invalidates on updates.
#[derive(Default)]
pub struct SimpleLspDb {
    files: HashMap<String, String>,
    cache: HashMap<String, String>,
}

impl SimpleLspDb {
    pub fn new() -> Self {
        SimpleLspDb {
            files: HashMap::new(),
            cache: HashMap::new(),
        }
    }

    pub fn set_file_text(&mut self, path: &str, text: &str) {
        self.files.insert(path.to_string(), text.to_string());
        self.cache.remove(path);
    }

    pub fn analysis_text(&mut self, path: &str) -> String {
        if let Some(c) = self.cache.get(path) {
            return c.clone();
        }
        let text = match self.files.get(path) {
            Some(t) => t.clone(),
            None => "".to_string(),
        };

        let mut lexer = Lexer::new(&text);
        let result = match lexer.tokenize() {
            Ok(tokens) => {
                let mut parser = Parser::new(tokens);
                match parser.parse_program() {
                    Ok(_) => format!("ok:{}", path),
                    Err(e) => format!("parse_error:{}", e),
                }
            }
            Err(e) => format!("lex_error:{}", e),
        };

        self.cache.insert(path.to_string(), result.clone());
        result
    }
}
