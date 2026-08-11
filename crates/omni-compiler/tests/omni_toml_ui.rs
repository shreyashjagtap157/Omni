use omni_compiler::package::omni_toml_parser::parse_manifest;

#[test]
fn test_parse_manifest() {
    let toml = r#"
        [package]
        name = "my_app"
        version = "0.1.0"
        edition = "2026"

        [dependencies]
        std = "1.0.0"

        [modules]
        module = "http"
        module = "utils"

        [capabilities]
        network = ["read", "write"]
        filesystem = ["read", "/tmp"]
        subprocess = false

        [features]
        default = ["network"]

        [build_targets]
        target = "wasm32-unknown-unknown"
    "#;

    let manifest = parse_manifest(toml).expect("Failed to parse manifest");

    assert_eq!(manifest.name, "my_app");
    assert_eq!(manifest.version, "0.1.0");
    assert_eq!(manifest.edition.unwrap(), "2026");

    assert_eq!(manifest.dependencies.get("std").unwrap(), "1.0.0");

    assert_eq!(manifest.modules, vec!["http", "utils"]);

    assert_eq!(
        manifest.capabilities.get("network").unwrap(),
        &vec!["read", "write"]
    );
    assert_eq!(
        manifest.capabilities.get("filesystem").unwrap(),
        &vec!["read", "/tmp"]
    );
    assert_eq!(manifest.capabilities.get("subprocess").unwrap().len(), 0);

    assert_eq!(manifest.features.get("default").unwrap(), &vec!["network"]);

    assert_eq!(manifest.build_targets, vec!["wasm32-unknown-unknown"]);
}
