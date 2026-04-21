use omni_compiler::{lexer::Lexer, parser::Parser, formatter};

fn main() {
    let src = "print 1\nlet x = 42\nprint x\n";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().unwrap();
    let formatted = formatter::format_program(&prog);
    println!("Formatted: {:?}", formatted);
}
