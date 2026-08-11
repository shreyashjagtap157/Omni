// The `Diagnostic` type is intentionally large (spans, labels, suggestions).
// Boxing it would add unnecessary indirection across the entire compiler.
#![allow(clippy::result_large_err)]

pub mod abi_check;
pub mod ast;
pub mod codegen;
pub mod codegen_lir;
pub mod codegen_rust;
pub mod complete_lexer;
pub mod comptime;
pub mod control_flow;
pub mod cst;
pub mod diagnostics;
pub mod doc_gen;
pub mod driver;
pub mod effect_resolver;
pub mod effect_system;
pub mod formatter;
pub mod generational_refs;
pub mod inout_desugar;
pub mod integration;
pub mod interpreter;
pub mod linear_types;
pub mod llvm_detect;
pub mod lsp;
pub mod lsp_incr_db;
pub mod lsp_salsa_db;
pub mod macros;
pub mod mir;
pub mod mir_optimize;
pub mod module_system;
pub mod monomorphizer;
pub mod omni_toml_parser;
pub mod package;
pub mod parser;
pub mod parser_utils;
pub mod polonius;
pub mod resolver;
pub mod security;
pub mod stdlib_bridge;
pub mod traits;
pub mod type_checker;
pub mod type_export;
pub mod types;
pub mod version;
pub mod vm;

// Re-exports for backward compatibility with existing tests
pub use parser_utils::parse_file;

pub fn emit_lir_file(path: &std::path::Path) -> Result<String, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let compiler = driver::Compiler::new(&text, driver::Backend::Native);
    let result = compiler.compile();
    let errors: Vec<String> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == diagnostics::Severity::Error)
        .map(ToString::to_string)
        .collect();
    if !errors.is_empty() {
        return Err(format!("Compilation failed: {}", errors.join("; ")));
    }
    if let Some(mir_module) = result.mir {
        let lir_module = codegen_lir::lower_mir_to_lir(&mir_module)?;
        Ok(format!("{:?}", lir_module))
    } else {
        Err("Failed to generate MIR".to_string())
    }
}

pub fn export_types_file(
    path: &std::path::Path,
    format: type_export::TypeExportFormat,
) -> Result<String, String> {
    let program = parser_utils::parse_file(path).map_err(|e| e.to_string())?;
    let doc = type_export::export_program(&program);
    match format {
        type_export::TypeExportFormat::Json => type_export::document_to_json(&doc),
        type_export::TypeExportFormat::CHeader => type_export::document_to_c_header(&doc),
        type_export::TypeExportFormat::Python => type_export::document_to_python_module(&doc),
    }
}

pub fn check_abi_files(
    old: &std::path::Path,
    new: &std::path::Path,
) -> Result<Vec<String>, String> {
    let old_prog = parser_utils::parse_file(old).map_err(|e| e.to_string())?;
    let new_prog = parser_utils::parse_file(new).map_err(|e| e.to_string())?;
    let old_doc = type_export::export_program(&old_prog);
    let new_doc = type_export::export_program(&new_prog);
    Ok(abi_check::compare_documents(&old_doc, &new_doc))
}

pub fn parse_cst_file(path: &std::path::Path) -> Result<cst::SyntaxNode, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    cst::build_cst_from_source(&text).map_err(|e| e.to_string())
}
