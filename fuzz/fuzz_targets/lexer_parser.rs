#![no_main]

use libfuzzer_sys::fuzz_target;
use omni_compiler::lexer::Lexer;
use omni_compiler::parser::Parser;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let mut lexer = Lexer::new(s);
        if let Ok(tokens) = lexer.tokenize() {
            let mut parser = Parser::new(tokens);
            let _ = parser.parse_program();
        }
    }
});