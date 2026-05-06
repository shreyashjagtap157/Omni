use std::hint::black_box;

fn fuzz_parse(data: &[u8]) {
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    if let Ok(tokens) = omni_compiler::complete_lexer::tokenize_complete(input) {
        if let Ok(program) = {
            let mut parser = omni_compiler::parser::Parser::new(tokens);
            parser.parse_program()
        } {
            let _ = format!("{:?}", program);
        }
    }
}

fn fuzz_roundtrip(data: &[u8]) {
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    if let Ok(tokens) = omni_compiler::complete_lexer::tokenize_complete(input) {
        if let Ok(program) = {
            let mut parser = omni_compiler::parser::Parser::new(tokens);
            parser.parse_program()
        } {
            let formatted = omni_compiler::formatter::format_program(&program);
            let _ = black_box(formatted);
        }
    }
}

fn fuzz_cst_roundtrip(data: &[u8]) {
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    if let Ok(tokens) = omni_compiler::complete_lexer::tokenize_complete(input) {
        let cst = omni_compiler::cst::build_cst(&tokens);
        let text = omni_compiler::cst::format_cst(&cst, 0);
        let _ = black_box(text);
    }
}

#[no_mangle]
pub extern "C" fn LLVMFuzzerTestOneInput(data: *const u8, size: usize) -> bool {
    let data = unsafe { std::slice::from_raw_parts(data, size) };
    fuzz_parse(data);
    fuzz_roundtrip(data);
    fuzz_cst_roundtrip(data);
    true
}
