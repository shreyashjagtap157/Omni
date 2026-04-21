use omni_compiler::parse_cst_file;
use omni_compiler::formatter::format_cst_source;

#[test]
fn mixed_tabs_and_spaces_roundtrip() {
    let src = "print \"start\"\n\tlet x = 1\n    print x\n";
    use std::io::Write;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    tmp.flush().unwrap();
    let path = tmp.path();
    let cst1 = parse_cst_file(path).expect("parse_cst failed");
    let out = format_cst_source(&cst1);
    // roundtrip
    let mut tmp2 = tempfile::NamedTempFile::new().expect("tmpfile2");
    write!(tmp2, "{}", out).unwrap();
    tmp2.flush().unwrap();
    let cst2 = parse_cst_file(tmp2.path()).expect("parse_cst failed on formatted output");
    assert_eq!(cst1.children.len(), cst2.children.len(), "CST child count should match after roundtrip");
}

#[test]
fn blank_lines_preserved() {
    let src = "print \"a\"\n\n\nprint \"b\"\n";
    use std::io::Write;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    tmp.flush().unwrap();
    let cst = parse_cst_file(tmp.path()).expect("parse_cst failed");
    // debug: print CST structure to help diagnose newline tokens
    eprintln!("CST debug:\n{}", omni_compiler::cst::format_cst(&cst, 0));
    // Inspect CST children for consecutive Newline tokens (blank line)
    use omni_compiler::cst::SyntaxElement;
    let mut max_run = 0usize;
    let mut run = 0usize;
    for child in &cst.children {
        match child {
            SyntaxElement::Token(t) => {
                if matches!(t.kind, omni_compiler::cst::SyntaxKind::TokenNewline) {
                    run += 1;
                    if run > max_run { max_run = run; }
                } else {
                    run = 0;
                }
            }
            _ => { run = 0; }
        }
    }
    assert!(max_run >= 2, "Expected at least one blank line (2 consecutive newlines) in CST, max_run={}", max_run);
}

#[test]
fn blank_lines_with_leading_spaces_preserved() {
    let src = "print \"a\"\n   \nprint \"b\"\n";
    use std::io::Write;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    tmp.flush().unwrap();
    let cst = parse_cst_file(tmp.path()).expect("parse_cst failed");
    let out = format_cst_source(&cst);
    assert!(out.contains("print \"a\""));
    assert!(out.contains("print \"b\""));
    assert!(out.contains("\n\n"), "Expected blank line to be preserved in formatted output, got: {}", out);
}

#[test]
fn block_comments_preserved() {
    let src = "print \"start\"\n---\nmulti\nline\n---\nprint \"end\"\n";
    use std::io::Write;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    tmp.flush().unwrap();
    let cst = parse_cst_file(tmp.path()).expect("parse_cst failed");
    let out = format_cst_source(&cst);
    assert!(out.contains("---\nmulti\nline\n---"), "Expected block comment preserved in output: {}", out);
}
