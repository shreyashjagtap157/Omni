use lir::example_module;

#[test]
fn builds_example_module() {
    let m = example_module();
    assert_eq!(m.functions.len(), 1);
    assert_eq!(m.functions[0].name, "main");
}
