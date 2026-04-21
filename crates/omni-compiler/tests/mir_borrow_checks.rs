use omni_compiler::mir;
use omni_compiler::parse_file;
use omni_compiler::polonius;
use std::io::Write;

#[test]
fn use_after_move_is_reported() {
    let src = "let a = 1\nlet b = a\nprint a\n";
    let mut tmp = tempfile::NamedTempFile::new().expect("tmpfile");
    write!(tmp, "{}", src).unwrap();
    let path = tmp.path();
    let prog = parse_file(path).expect("parse failed");
    let module = mir::lower_program_to_mir(&prog);
    // Ensure tests use the in-repo mock solver regardless of external env.
    std::env::remove_var("OMNI_USE_POLONIUS");
    let res = polonius::check_mir(&module);
    assert!(
        res.is_err(),
        "expected polonius check to report use-after-move"
    );
}
