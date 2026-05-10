// Test debug
use omni_compiler::complete_lexer;

fn main() {
    let src = r#"print "Hello, World!""#;
    eprintln!("Input: {:?}", src);
    match complete_lexer::tokenize_complete(src) {
        Ok(tokens) => {
            eprintln!("Tokens:");
            for (i, t) in tokens.iter().enumerate() {
                eprintln!("{}: {:?} {:?} @ {}:{}", i, t.kind, t.text, t.line, t.col);
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
