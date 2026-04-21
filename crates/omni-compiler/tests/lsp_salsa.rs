#[cfg(feature = "use_salsa_lsp")]
use omni_compiler::lsp_salsa_db::LspDb;

#[test]
#[cfg(feature = "use_salsa_lsp")]
fn salsa_incremental_analysis_updates_on_change() {
    let mut db = LspDb::new();
    let path = "file1.omni".to_string();
    db.set_file_text(path.clone(), "let x = 1".to_string());
    let a1 = db.analysis_text(path.clone());
    assert!(a1.starts_with("ok:"));

    db.set_file_text(path.clone(), "let x =".to_string());
    let a2 = db.analysis_text(path.clone());
    assert!(a2.starts_with("parse_error:") || a2.starts_with("lex_error:"));
}
