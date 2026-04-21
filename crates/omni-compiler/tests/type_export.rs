use omni_compiler::type_export::TypeExportFormat;
use omni_compiler::{check_abi_files, export_types_file};
use std::fs;
use tempfile::NamedTempFile;

fn write_source(source: &str) -> NamedTempFile {
    let file = NamedTempFile::new().expect("create temp file");
    fs::write(file.path(), source).expect("write temp source");
    file
}

#[test]
fn export_types_produces_json_for_functions_and_structs() {
    let file = write_source(
        "struct Point [x: int, y: int]\nfn add(a, b) -> int:\n    return 1\n",
    );

    let output = export_types_file(file.path(), TypeExportFormat::Json).expect("export failed");
    let json: serde_json::Value = serde_json::from_str(&output).expect("valid json");

    assert_eq!(json["schema_version"], "omni.type-export.v1");
    let items = json["items"].as_array().expect("items array");
    assert!(
        items
            .iter()
            .any(|item| item["kind"] == "struct" && item["name"] == "Point")
    );

    let function = items
        .iter()
        .find(|item| item["kind"] == "function" && item["name"] == "add")
        .expect("function export");
    assert_eq!(function["return_type"], "int");
    assert_eq!(function["params"].as_array().expect("params array").len(), 2);
}

#[test]
fn export_types_can_render_c_header() {
    let file = write_source(
        "struct Point [x: int, y: int]\nfn add(a, b) -> int:\n    return 1\n",
    );

    let output = export_types_file(file.path(), TypeExportFormat::CHeader).expect("export failed");
    assert!(output.contains("typedef struct Point"));
    assert!(output.contains("extern int64_t add("));
}

#[test]
fn export_types_can_render_python_bindings() {
    let file = write_source(
        "struct Point [x: int, y: int]\nfn add(a, b) -> int:\n    return 1\n",
    );

    let output = export_types_file(file.path(), TypeExportFormat::Python).expect("export failed");
    assert!(output.contains("class Point(_ctypes.Structure):"));
    assert!(output.contains("def configure_add(lib):"));
}

#[test]
fn abi_check_detects_breaking_function_changes() {
    let old_file = write_source("fn add(a, b) -> int:\n    return 1\n");
    let new_file = write_source("fn add(a) -> int:\n    return 1\n");

    let diffs = check_abi_files(old_file.path(), new_file.path()).expect("abi check failed");
    assert!(!diffs.is_empty());
    assert!(diffs.iter().any(|diff| diff.contains("parameter count changed")));
}