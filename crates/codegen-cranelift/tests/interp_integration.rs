use codegen_cranelift::{render_lir_text, run_lir_interpreter};
use lir::example_module;

#[test]
fn interpreter_runs_example_module() {
    let m = example_module();
    let text = render_lir_text(&m);
    assert!(text.contains("fn main"));

    let res = run_lir_interpreter(&m).expect("interpreter failed");
    assert_eq!(res.return_values[0], 42);
}
