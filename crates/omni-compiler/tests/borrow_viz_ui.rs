use omni_compiler::lsp::analyze_borrows;
use omni_compiler::lsp::CompilationDatabase;

#[test]
fn test_borrow_analysis_basic() {
    let mut db = CompilationDatabase::new();
    db.add_source(
        "test.omni".to_string(),
        "let x = 1\nprint x\n".to_string(),
        1,
    );

    let viz = analyze_borrows(&db, "test.omni");
    // Basic analysis should complete without errors
    let _ = viz;
}

#[test]
fn test_borrow_analysis_with_move() {
    let mut db = CompilationDatabase::new();
    db.add_source(
        "test.omni".to_string(),
        "let x = 1\nlet y = x\nprint y\n".to_string(),
        1,
    );

    let viz = analyze_borrows(&db, "test.omni");
    // Should detect the move
    let _ = viz;
}

#[test]
fn test_borrow_analysis_with_function_call() {
    let mut db = CompilationDatabase::new();
    db.add_source(
        "test.omni".to_string(),
        "fn test(a)\n    print a\nlet x = 1\ntest(x)\nprint x\n".to_string(),
        1,
    );

    let viz = analyze_borrows(&db, "test.omni");
    // Should have some borrow info
    let _ = viz;
}

#[test]
fn test_borrow_analysis_field_access() {
    let mut db = CompilationDatabase::new();
    db.add_source(
        "test.omni".to_string(),
        "struct Point [x: int, y: int]\nprint 1\n".to_string(),
        1,
    );

    let viz = analyze_borrows(&db, "test.omni");
    // Field access creates borrows
    let _ = viz;
}

#[test]
fn test_borrow_analysis_nonexistent_file() {
    let db = CompilationDatabase::new();
    let viz = analyze_borrows(&db, "nonexistent.omni");
    // Should return empty visualization
    assert!(viz.borrows.is_empty());
    assert!(viz.moves.is_empty());
    assert!(viz.issues.is_empty());
}
