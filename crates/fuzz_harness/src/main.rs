use std::env;
use std::fs;

use omni_compiler::lexer::Lexer;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: fuzz_harness <file>");
        std::process::exit(2);
    }
    let path = &args[1];
    let text = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read {}: {}", path, e);
            std::process::exit(1);
        }
    };

    let mut lexer = Lexer::new(&text);
    match lexer.tokenize() {
        Ok(tokens) => println!("OK: {} tokens", tokens.len()),
        Err(e) => println!("LEX_ERROR: {}", e),
    }
}
