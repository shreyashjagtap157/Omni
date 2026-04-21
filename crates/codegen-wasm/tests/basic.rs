use codegen_wasm::emit_wasm_bytes;
use lir::example_module;
use wasmparser::Parser;

#[test]
fn emits_parseable_wasm_for_example_module() {
    let bytes = emit_wasm_bytes(&example_module()).expect("wasm emission failed");
    assert!(bytes.starts_with(b"\0asm"));

    for payload in Parser::new(0).parse_all(&bytes) {
        payload.expect("generated wasm should parse");
    }
}
