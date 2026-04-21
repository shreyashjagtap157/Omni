use crate::lexer::Lexer;
use crate::parser::Parser;

// If Salsa is NOT enabled, re-export the simple incremental DB implementation
// from `lsp_incr_db.rs` so the crate builds without Salsa.
#[cfg(not(feature = "use_salsa_lsp"))]
pub use crate::lsp_incr_db::SimpleLspDb as LspDb;

// --- Salsa-backed implementation ---
#[cfg(feature = "use_salsa_lsp")]
mod salsa_impl {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use crate::lsp::CompilationDatabase;

    // Input struct representing a file and its text. The `input` macro
    // generates the ID type `File` and associated accessors/setters.
    #[salsa::input]
    pub struct File {
        pub path: String,
        #[returns(ref)]
        pub text: String,
    }

    // Declare the DB view trait used by the tracked queries. This trait is
    // extended by the `salsa::db` macro with internal plumbing.
    #[salsa::db]
    pub trait Lsp: salsa::Database {
        fn file(&self, path: String) -> File;
    }

    use salsa::Setter;

    // Tracked query that computes a tiny analysis string for a file. The
    // query takes a `File` id (an input entity) rather than a bare `String`.
    // This matches Salsa's expectations for query arguments.
    #[salsa::tracked]
    pub fn analysis_text(db: &dyn Lsp, file: File) -> String {
        let path = file.path(db);
        let text = file.text(db);

        let mut lexer = Lexer::new(&text);
        match lexer.tokenize() {
            Ok(tokens) => {
                let mut parser = Parser::new(tokens);
                match parser.parse_program() {
                    Ok(_) => format!("ok:{}", path),
                    Err(e) => format!("parse_error:{}", e),
                }
            }
            Err(e) => format!("lex_error:{}", e),
        }
    }

    // The concrete DB backing used by the LSP. It stores a small cache of
    // File IDs so we can return the same `File` id for a given path.
    #[salsa::db]
    #[derive(Default)]
    pub struct LspDb {
        storage: salsa::Storage<LspDb>,
        files: RefCell<HashMap<String, File>>,
        shadow: RefCell<CompilationDatabase>,
    }

    // Hook up the salsa Database impl.
    #[salsa::db]
    impl salsa::Database for LspDb {}

    impl LspDb {
        pub fn new() -> Self {
            Default::default()
        }

        fn update_source(&mut self, path: String, text: String, version: usize) {
            let shadow_path = path.clone();
            let shadow_text = text.clone();

            // Avoid holding a borrow across calls that require `&mut self`.
            let maybe_file = {
                let files = self.files.borrow();
                files.get(&path).copied()
            };

            if let Some(file) = maybe_file {
                file.set_text(self).to(text);
            } else {
                let file = File::new(self, path.clone(), text.clone());
                self.files.borrow_mut().insert(path, file);
            }

            self.shadow
                .borrow_mut()
                .add_source(shadow_path, shadow_text, version);
        }

        // Set or update the text for `path`.
        pub fn set_file_text(&mut self, path: String, text: String) {
            self.update_source(path, text, 0);
        }

        pub fn add_source(&mut self, path: String, text: String, version: usize) {
            self.update_source(path, text, version);
        }

        // Convenience wrapper that calls the tracked query.
        pub fn analysis_text(&self, path: String) -> String {
            // Obtain the File id for `path` and call the tracked query.
            let file = self.file(path);
            crate::lsp_salsa_db::salsa_impl::analysis_text(self, file)
        }

        pub fn hover_at(&self, path: &str, line: usize, col: usize) -> Option<crate::lsp::QueryResult> {
            self.shadow.borrow().hover_at(path, line, col)
        }

        pub fn goto_definition(&self, path: &str, line: usize, col: usize) -> Option<(String, crate::diagnostics::Span)> {
            self.shadow.borrow().goto_definition(path, line, col)
        }

        pub fn get_inlay_hints(&self, path: &str) -> Vec<crate::lsp::InlayHint> {
            self.shadow.borrow().get_inlay_hints(path)
        }

        pub fn get_borrow_visualization(&self, path: &str) -> crate::lsp::BorrowVisualization {
            self.shadow.borrow().get_borrow_visualization(path)
        }

        pub fn get_completions(&self, path: &str, line: usize, col: usize) -> Vec<crate::lsp::CompletionItem> {
            self.shadow.borrow().get_completions(path, line, col)
        }

        pub fn get_diagnostics(&self, path: &str) -> Vec<crate::diagnostics::Diagnostic> {
            self.shadow.borrow().get_diagnostics(path)
        }
    }

    // Implement the view method that maps a path to a `File` id. We use
    // interior mutability so the method can be called with `&self`.
    #[salsa::db]
    impl Lsp for LspDb {
        fn file(&self, path: String) -> File {
            let mut files = self.files.borrow_mut();
            match files.get(&path) {
                Some(&f) => f,
                None => {
                    let f = File::new(self, path.clone(), String::new());
                    files.insert(path.clone(), f);
                    f
                }
            }
        }
    }

    // Re-export the concrete type at module root so callers can use
    // `lsp_salsa_db::LspDb` uniformly regardless of the feature.
    pub use LspDb as LspDbConcrete;
}

#[cfg(feature = "use_salsa_lsp")]
pub use salsa_impl::LspDbConcrete as LspDb;

