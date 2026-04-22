use omni_compiler::lsp::CompilationDatabase;
use omni_compiler::lsp::HoverContent;
use omni_compiler::lsp_incr_db::SimpleLspDb;

#[test]
fn incr_db_updates_and_caches_analysis() {
    let mut db = SimpleLspDb::new();
    let path = "file2.omni";
    db.set_file_text(path, "let x = 1");
    let a1 = db.analysis_text(path);
    assert!(a1.starts_with("ok:"));

    // second call should hit cache (result unchanged)
    let a1b = db.analysis_text(path);
    assert_eq!(a1, a1b);

    // update file and ensure analysis changes
    db.set_file_text(path, "let x =");
    let a2 = db.analysis_text(path);
    assert!(a2.starts_with("parse_error:") || a2.starts_with("lex_error:"));
}

#[test]
fn cross_file_definition_lookup_uses_symbol_names() {
    let mut db = CompilationDatabase::new();
    db.add_source("lib.omni".to_string(), "let foo = 1\n".to_string(), 1);
    db.add_source("main.omni".to_string(), "print foo\n".to_string(), 1);

    let definition = db
        .goto_definition("main.omni", 1, 6)
        .expect("expected cross-file definition");

    assert_eq!(definition.0, "lib.omni");
}

#[test]
fn cross_file_hover_shows_inferred_type() {
    let mut db = CompilationDatabase::new();
    db.add_source("lib.omni".to_string(), "let foo = 1\n".to_string(), 1);
    db.add_source("main.omni".to_string(), "print foo\n".to_string(), 1);

    let hover = db.hover_at("main.omni", 1, 6).expect("expected hover");
    // hover.text contains short type string when available
    assert!(
        hover.text.contains("int")
            || hover
                .contents
                .iter()
                .any(|c| matches!(c, HoverContent::Type(t) if t.contains("int")))
    );
}

#[test]
fn rename_symbol_across_workspace_updates_sources_and_analysis() {
    let mut db = CompilationDatabase::new();
    db.add_source("lib.omni".to_string(), "let foo = 1\n".to_string(), 1);
    db.add_source("main.omni".to_string(), "print foo\n".to_string(), 1);

    db.rename_symbol_across_workspace("foo", "bar")
        .expect("rename failed");

    // sources should now contain the new name
    let main_text = db.get_source_text("main.omni").expect("main text");
    assert!(main_text.contains("bar"));

    let lib_text = db.get_source_text("lib.omni").expect("lib text");
    assert!(lib_text.contains("bar"));

    // goto_definition on the renamed use should still resolve to lib.omni
    let def = db
        .goto_definition("main.omni", 1, 6)
        .expect("expected definition");
    assert_eq!(def.0, "lib.omni");
}

#[test]
fn completion_lists_keywords_and_workspace_symbols() {
    let mut db = CompilationDatabase::new();
    db.add_source("lib.omni".to_string(), "let alpha = 1\n".to_string(), 1);
    db.add_source("main.omni".to_string(), "al\npri\n".to_string(), 1);

    let symbol_items = db.get_completions("main.omni", 1, 2);
    assert!(symbol_items.iter().any(|i| i.label == "alpha"));

    let keyword_items = db.get_completions("main.omni", 2, 3);
    assert!(keyword_items.iter().any(|i| i.label == "print"));
}

#[test]
fn completion_includes_struct_field_names() {
    let mut db = CompilationDatabase::new();
    db.add_source(
        "lib.omni".to_string(),
        "struct Point [x: int, y: int]\n".to_string(),
        1,
    );
    db.add_source(
        "main.omni".to_string(),
        "let p = Point{}\np.\n".to_string(),
        1,
    );

    // line 2 (1-based), column after 'p.' (0-based column 2)
    let completions = db.get_completions("main.omni", 2, 2);
    assert!(completions.iter().any(|i| i.label == "x"));
    assert!(completions.iter().any(|i| i.label == "y"));
}
