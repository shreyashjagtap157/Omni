use omni_compiler::package::build_graph::BuildGraph;
use std::fs;
use std::time::{Duration, SystemTime};
use tempfile::tempdir;

#[test]
fn test_build_graph_stale() {
    let dir = tempdir().unwrap();
    let src1 = dir.path().join("a.omni");
    let src2 = dir.path().join("b.omni");
    let out = dir.path().join("out.o");

    let baseline = SystemTime::now() - Duration::from_secs(10);
    fs::write(&src1, "file A").unwrap();
    fs::write(&src2, "file B").unwrap();

    filetime_set(&src1, baseline);
    filetime_set(&src2, baseline);

    // Simulate compilation
    fs::write(&out, "compiled output").unwrap();
    filetime_set(&out, baseline + Duration::from_secs(5));

    let mut graph = BuildGraph::new();
    graph.add_node(out.clone(), vec![src1.clone(), src2.clone()]);

    // Should not be stale
    assert!(!graph.is_stale(&out));

    // Update dependency mtime to a time clearly after `out`.
    filetime_set(&src1, baseline + Duration::from_secs(20));

    // Now it should be stale
    assert!(graph.is_stale(&out));
}

fn filetime_set(path: &std::path::Path, t: SystemTime) {
    let f = fs::OpenOptions::new().write(true).open(path).unwrap();
    f.set_modified(t).unwrap();
}
