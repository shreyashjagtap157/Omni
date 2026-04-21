fn main() {
    use std::path::Path;
    println!("omni-stage0: minimal Stage0 CLI");
    let args: Vec<String> = std::env::args().collect();
    fn print_usage() {
        eprintln!(
            "Usage: omni-stage0 <command> <file>\nCommands: parse, parse-cst, fmt-cst, fmt, run, check, emit-mir, check-mir, run-mir, run-native"
        );
    }

    if args.len() > 1 {
        match args[1].as_str() {
            "help" | "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            "parse" => {
                if args.len() < 3 {
                    eprintln!("Usage: omni-stage0 parse <file>");
                    std::process::exit(2);
                }
                let path = Path::new(&args[2]);
                match omni_compiler::parse_file(path) {
                    Ok(program) => {
                        println!("Parsed program: {:#?}", program);
                    }
                    Err(e) => {
                        eprintln!("Error parsing: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "lex" => {
                if args.len() < 3 {
                    eprintln!("Usage: omni-stage0 lex <file>");
                    std::process::exit(2);
                }
                let path = Path::new(&args[2]);
                let src = std::fs::read_to_string(path).map_err(|e| e.to_string()).unwrap_or_else(|e| { eprintln!("read error: {}", e); std::process::exit(1); });
                let mut lexer = omni_compiler::lexer::Lexer::new(&src);
                match lexer.tokenize() {
                    Ok(toks) => {
                        for t in toks.iter() {
                            println!("{:?} {}:{} '{}'", t.kind, t.line, t.col, t.text);
                        }
                    }
                    Err(e) => {
                        eprintln!("Lexer error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "parse-cst" => {
                if args.len() < 3 {
                    eprintln!("Usage: omni-stage0 parse-cst <file>");
                    std::process::exit(2);
                }
                let path = Path::new(&args[2]);
                match omni_compiler::parse_cst_file(path) {
                    Ok(cst) => {
                        println!("CST:\n{}", omni_compiler::cst::format_cst(&cst, 0));
                    }
                    Err(e) => {
                        eprintln!("Error parsing CST: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "fmt-cst" => {
                if args.len() < 3 {
                    eprintln!("Usage: omni-stage0 fmt-cst <file>");
                    std::process::exit(2);
                }
                let path = Path::new(&args[2]);
                match omni_compiler::parse_cst_file(path) {
                    Ok(cst) => {
                        let out = omni_compiler::formatter::format_cst_source(&cst);
                        println!("{}", out);
                    }
                    Err(e) => {
                        eprintln!("Error parsing CST: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "run" => {
                if args.len() < 3 {
                    eprintln!("Usage: omni-stage0 run <file>");
                    std::process::exit(2);
                }
                let path = Path::new(&args[2]);
                match omni_compiler::run_file(path) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Run failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "fmt" => {
                if args.len() < 3 {
                    eprintln!("Usage: omni-stage0 fmt <file>");
                    std::process::exit(2);
                }
                let path = Path::new(&args[2]);
                match omni_compiler::format_file(path) {
                    Ok(_) => println!("Formatted {}", path.display()),
                    Err(e) => {
                        eprintln!("Format failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "check" => {
                if args.len() < 3 {
                    eprintln!("Usage: omni-stage0 check <file>");
                    std::process::exit(2);
                }
                let path = Path::new(&args[2]);
                match omni_compiler::check_file(path) {
                    Ok(_) => println!("Type check OK"),
                    Err(e) => {
                        eprintln!("Type check failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "emit-mir" => {
                if args.len() < 3 {
                    eprintln!("Usage: omni-stage0 emit-mir <file>");
                    std::process::exit(2);
                }
                let path = Path::new(&args[2]);
                match omni_compiler::emit_mir_file(path) {
                    Ok(s) => println!("{}", s),
                    Err(e) => {
                        eprintln!("Emit MIR failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "check-mir" => {
                if args.len() < 3 {
                    eprintln!("Usage: omni-stage0 check-mir <file>");
                    std::process::exit(2);
                }
                let path = Path::new(&args[2]);
                match omni_compiler::check_mir_file(path) {
                    Ok(_) => println!("MIR check OK"),
                    Err(e) => {
                        eprintln!("MIR check failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "run-mir" => {
                if args.len() < 3 {
                    eprintln!("Usage: omni-stage0 run-mir <file>");
                    std::process::exit(2);
                }
                let path = Path::new(&args[2]);
                match omni_compiler::run_mir_vm_file(path) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("run-mir failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "run-native" => {
                if args.len() < 3 {
                    eprintln!("Usage: omni-stage0 run-native <file>");
                    std::process::exit(2);
                }
                let path = Path::new(&args[2]);
                match omni_compiler::run_native_file(path) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("run-native failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            _ => {
                eprintln!("Unknown command: {}", args[1]);
                print_usage();
                std::process::exit(1);
            }
        }
    } else {
        print_usage();
        std::process::exit(1);
    }
}
