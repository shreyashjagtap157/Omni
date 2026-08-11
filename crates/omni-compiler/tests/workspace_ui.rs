use omni_compiler::package::omni_workspace::Workspace;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_workspace_load() {
    let dir = tempdir().unwrap();
    let root_path = dir.path().to_path_buf();

    // Create root manifest
    let root_toml = r#"
        [package]
        name = "my_workspace"
        version = "1.0.0"

        [dependencies]
        shared_lib = "0.1.0"
    "#;
    fs::write(root_path.join("omni.toml"), root_toml).unwrap();

    // Create a sub-package
    let sub_dir = root_path.join("sub_pkg");
    fs::create_dir(&sub_dir).unwrap();

    let sub_toml = r#"
        [package]
        name = "sub_pkg"
        version = "0.2.0"

        [dependencies]
        log = "1.0.0"
    "#;
    fs::write(sub_dir.join("omni.toml"), sub_toml).unwrap();

    let mut workspace = Workspace::new(root_path);
    workspace.load().expect("Failed to load workspace");

    // Check members
    assert_eq!(workspace.members.len(), 2);
    assert!(workspace.members.contains_key("my_workspace"));
    assert!(workspace.members.contains_key("sub_pkg"));

    // Check resolution
    assert!(workspace.resolve_all_dependencies().is_ok());
}
