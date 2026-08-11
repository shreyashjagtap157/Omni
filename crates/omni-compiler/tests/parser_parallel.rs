use omni_compiler::parser_utils::parse_files_parallel;
use std::io::Write;

#[test]
fn independent_files_parse_in_parallel_with_deterministic_result_order() {
    let mut a = tempfile::NamedTempFile::new().expect("a");
    let mut b = tempfile::NamedTempFile::new().expect("b");
    writeln!(a, "fn a() -> i64 {{ return 1; }}").unwrap();
    writeln!(b, "fn b() -> i64 {{ return 2; }}").unwrap();
    a.flush().unwrap();
    b.flush().unwrap();

    let paths = vec![a.path().to_path_buf(), b.path().to_path_buf()];
    let results = parse_files_parallel(&paths);
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].0, paths[0]);
    assert_eq!(results[1].0, paths[1]);
    assert!(results[0].1.is_ok());
    assert!(results[1].1.is_ok());
}
