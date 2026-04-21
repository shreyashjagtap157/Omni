#[test]
fn inproc_polonius_lib_runs() {
    // Ensure adapter's library-backed path runs when OMNI_USE_POLONIUS=1
    std::env::set_var("OMNI_USE_POLONIUS", "1");
    // Minimal facts for a trivial function: def then use at points 0 and 1
    let facts = r#"
function tiny
 block 0
  0: const_int a 1
  1: print a

point tiny 0 0
def tiny 0 0 a
point tiny 0 1
use tiny 0 1 a
"#;
    let res = polonius_engine_adapter::check_facts(facts);
    std::env::remove_var("OMNI_USE_POLONIUS");
    assert!(res.is_ok(), "library-backed polonius path should run without error");
}
