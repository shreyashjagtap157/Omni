use omni_compiler::driver::{Backend, Compiler};
use std::io::Write;

#[test]
fn test_public_symbol_access() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");

    // 1. Write omni.toml
    let toml_path = dir.path().join("omni.toml");
    let mut toml_file = std::fs::File::create(&toml_path).unwrap();
    write!(
        toml_file,
        r#"name = "test_pkg"
version = "0.1.0"
[modules]
module = "utils"
"#
    )
    .unwrap();

    // 2. Write utils.omni (public function)
    let utils_path = dir.path().join("utils.omni");
    let mut utils_file = std::fs::File::create(&utils_path).unwrap();
    write!(
        utils_file,
        "pub fn add(a: int, b: int) -> int:\n    return a + b\n"
    )
    .unwrap();

    // 3. Write main.omni
    let main_src = "use utils::add\nfn main() -> int:\n    return add(1, 2)\n";
    let main_path = dir.path().join("main.omni");
    let mut main_file = std::fs::File::create(&main_path).unwrap();
    write!(main_file, "{}", main_src).unwrap();

    // 4. Compile main.omni with the correct path
    let compiler = Compiler::with_path(main_src, Backend::Native, main_path);
    let result = compiler.compile();

    assert!(
        result.diagnostics.is_empty(),
        "Expected no compilation errors, but got: {:?}",
        result.diagnostics
    );
    assert!(result.program.is_some());
}

#[test]
fn test_private_symbol_access() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");

    // 1. Write omni.toml
    let toml_path = dir.path().join("omni.toml");
    let mut toml_file = std::fs::File::create(&toml_path).unwrap();
    write!(
        toml_file,
        r#"name = "test_pkg"
version = "0.1.0"
[modules]
module = "utils"
"#
    )
    .unwrap();

    // 2. Write utils.omni (private function by default)
    let utils_path = dir.path().join("utils.omni");
    let mut utils_file = std::fs::File::create(&utils_path).unwrap();
    write!(
        utils_file,
        "fn add(a: int, b: int) -> int:\n    return a + b\n"
    )
    .unwrap();

    // 3. Write main.omni
    let main_src = "use utils::add\nfn main() -> int:\n    return add(1, 2)\n";
    let main_path = dir.path().join("main.omni");
    let mut main_file = std::fs::File::create(&main_path).unwrap();
    write!(main_file, "{}", main_src).unwrap();

    // 4. Compile main.omni
    let compiler = Compiler::with_path(main_src, Backend::Native, main_path);
    let result = compiler.compile();

    assert!(
        !result.diagnostics.is_empty(),
        "Expected compilation error for private symbol access, but got none"
    );
    let has_privacy_error = result.diagnostics.iter().any(|d| d.code.code() == "4012");
    assert!(
        has_privacy_error,
        "Expected TYPE_VISIBILITY_PRIVATE error, got: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_capability_based_visibility_success() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");

    // 1. Write omni.toml with capability network = true
    let toml_path = dir.path().join("omni.toml");
    let mut toml_file = std::fs::File::create(&toml_path).unwrap();
    write!(
        toml_file,
        r#"name = "test_pkg"
version = "0.1.0"
[modules]
module = "utils"
[capabilities]
network = true
"#
    )
    .unwrap();

    // 2. Write utils.omni (capability-based function)
    let utils_path = dir.path().join("utils.omni");
    let mut utils_file = std::fs::File::create(&utils_path).unwrap();
    write!(
        utils_file,
        "pub(cap: network) fn fetch() -> int:\n    return 42\n"
    )
    .unwrap();

    // 3. Write main.omni
    let main_src = "use utils::fetch\nfn main() -> int:\n    return fetch()\n";
    let main_path = dir.path().join("main.omni");
    let mut main_file = std::fs::File::create(&main_path).unwrap();
    write!(main_file, "{}", main_src).unwrap();

    // 4. Compile main.omni
    let compiler = Compiler::with_path(main_src, Backend::Native, main_path);
    let result = compiler.compile();

    assert!(
        result.diagnostics.is_empty(),
        "Expected no compilation errors with enabled capability, but got: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_capability_based_visibility_failure() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");

    // 1. Write omni.toml without network capability (or set to false)
    let toml_path = dir.path().join("omni.toml");
    let mut toml_file = std::fs::File::create(&toml_path).unwrap();
    write!(
        toml_file,
        r#"name = "test_pkg"
version = "0.1.0"
[modules]
module = "utils"
[capabilities]
network = false
"#
    )
    .unwrap();

    // 2. Write utils.omni (capability-based function)
    let utils_path = dir.path().join("utils.omni");
    let mut utils_file = std::fs::File::create(&utils_path).unwrap();
    write!(
        utils_file,
        "pub(cap: network) fn fetch() -> int:\n    return 42\n"
    )
    .unwrap();

    // 3. Write main.omni
    let main_src = "use utils::fetch\nfn main() -> int:\n    return fetch()\n";
    let main_path = dir.path().join("main.omni");
    let mut main_file = std::fs::File::create(&main_path).unwrap();
    write!(main_file, "{}", main_src).unwrap();

    // 4. Compile main.omni
    let compiler = Compiler::with_path(main_src, Backend::Native, main_path);
    let result = compiler.compile();

    assert!(
        !result.diagnostics.is_empty(),
        "Expected compilation error for capability-based visibility mismatch, but got none"
    );
    let has_privacy_error = result.diagnostics.iter().any(|d| d.code.code() == "4012");
    assert!(
        has_privacy_error,
        "Expected TYPE_VISIBILITY_PRIVATE error, got: {:?}",
        result.diagnostics
    );
}
