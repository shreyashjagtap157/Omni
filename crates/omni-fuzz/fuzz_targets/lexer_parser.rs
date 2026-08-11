#![no_main]

use libfuzzer_sys::fuzz_target;
use omni_compiler::complete_lexer;
use omni_compiler::parser::Parser;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        if let Ok(tokens) = complete_lexer::tokenize_complete(s) {
            let mut parser = Parser::new(tokens);
            let _ = parser.parse_program();
        }
    }
});
