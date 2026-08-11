use omni_compiler::cst::format_cst;
use omni_compiler::parse_cst_file;

#[test]
fn cst_preserves_comments() {
    let src = "print \"test\"\n// a line comment\nlet x = 1\n";
    use std::io::Write;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();
    let cst = parse_cst_file(path).expect("parse_cst failed");
    let out = format_cst(&cst, 0);
    // Must contain line comment token
    assert!(
        out.contains("TOKEN TokenCommentLine"),
        "expected line comment in CST: {}",
        out
    );
}

#[test]
fn cst_recovers_exact_source() {
    let src =
        "// lead\nfn main() -> i64 {\n\tlet x: i64 = 1;  /* keep spacing */\n    return x;\n}\n";
    use std::io::Write;
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    tmp.flush().unwrap();
    let cst = parse_cst_file(tmp.path()).expect("parse_cst failed");
    assert_eq!(omni_compiler::cst::recover_source(&cst), Some(src));
}
