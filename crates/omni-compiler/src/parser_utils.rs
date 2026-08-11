use crate::ast::Program;
use crate::diagnostics::{error_codes, Diagnostic};
use crate::parser::Parser;

pub fn parse_file(path: &std::path::Path) -> Result<Program, Diagnostic> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| Diagnostic::error(error_codes::PARSER_UNEXPECTED_TOKEN, e.to_string()))?;
    let mut lexer = crate::complete_lexer::CompleteLexer::new(&source);
    let tokens = lexer
        .tokenize()
        .map_err(|e| Diagnostic::error(error_codes::PARSER_UNEXPECTED_TOKEN, e.to_string()))?;
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

/// Parse independent source files concurrently while preserving input order.
///
/// Parsing is deliberately file-local: name resolution and module linking occur
/// in later compiler phases. Each result is returned beside its input path so a
/// caller can surface all file-local diagnostics without nondeterministic merge
/// order.
pub fn parse_files_parallel(
    paths: &[std::path::PathBuf],
) -> Vec<(std::path::PathBuf, Result<Program, Diagnostic>)> {
    std::thread::scope(|scope| {
        let handles = paths
            .iter()
            .map(|path| {
                let worker_path = path.clone();
                scope.spawn(move || parse_file(&worker_path))
            })
            .collect::<Vec<_>>();

        paths
            .iter()
            .cloned()
            .zip(handles)
            .map(|(path, handle)| {
                let result = match handle.join() {
                    Ok(result) => result,
                    Err(_) => Err(Diagnostic::error(
                        error_codes::PARSER_UNEXPECTED_TOKEN,
                        format!("parser worker failed while parsing {}", path.display()),
                    )),
                };
                (path, result)
            })
            .collect()
    })
}
