#![no_main]

use libfuzzer_sys::fuzz_target;
use omni_compiler::complete_lexer;
use omni_compiler::cst;
use omni_compiler::formatter;
use omni_compiler::parser::Parser;

fuzz_target!(|data: &[u8]| {
    let input = match std::str::from_utf8(data) {
        Ok(text) => text,
        Err(_) => return,
    };

    let tokens = match complete_lexer::tokenize_complete(input) {
        Ok(tokens) => tokens,
        Err(_) => return,
    };

    let cst = cst::build_cst(&tokens);
    let cst_text = cst::format_cst(&cst, 0);
    let _ = std::hint::black_box(cst_text);

    let mut parser = Parser::new(tokens.clone());
    let program = match parser.parse_program() {
        Ok(program) => program,
        Err(_) => return,
    };

    let formatted = formatter::format_program(&program);
    let _ = std::hint::black_box(formatted.clone());

    if let Ok(tokens2) = complete_lexer::tokenize_complete(&formatted) {
        let mut parser2 = Parser::new(tokens2);
        let _ = parser2.parse_program();
    }
});