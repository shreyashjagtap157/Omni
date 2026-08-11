#[cfg(test)]
mod tests {
    use omni_compiler::driver::{Backend, Compiler};

    fn compile(source: &str) -> Vec<String> {
        let compiler = Compiler::new(source, Backend::Native);
        let result = compiler.compile();
        result
            .diagnostics
            .into_iter()
            .map(|d| format!("{:?}", d))
            .collect()
    }

    #[test]
    fn test_valid_borrow() {
        let source = "let x = 10\nlet y = x\nprint(y)\n";
        let errs = compile(source);
        eprintln!("Diagnostics: {:?}", errs);
        assert!(errs.is_empty());
    }

    #[test]
    fn test_invalid_mutable_borrow() {
        let source = "linear x = 10\nlet y = x\nprint(x)\n";
        let errs = compile(source);
        // Ownership-sensitive source must fail closed until the v0.2 borrow checker
        // is qualified.
        assert!(
            !errs.is_empty(),
            "expected diagnostics for invalid borrow, got none"
        );
    }

    #[test]
    fn test_use_after_move() {
        let source = "linear x = 10\nlet y = x\nlet z = x\n";
        let errs = compile(source);
        assert!(
            !errs.is_empty(),
            "expected diagnostics for use-after-move, got none"
        );
    }
}
