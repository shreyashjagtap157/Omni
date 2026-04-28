fn main() {
    use std::path::Path;
    println!("omni-stage0: minimal Stage0 CLI");
    let args: Vec<String> = std::env::args().collect();
    fn print_usage() {
        eprintln!(
            "Usage: omni-stage0 <command> <file>\nCommands: parse, parse-cst, fmt-cst, fmt, run, check, emit-mir, check-mir, run-mir, run-native, emit-wasm, export-types, bindgen, check-abi"
        );
    }

    fn parse_bindgen_format(
        value: Option<&str>,
    ) -> Result<omni_compiler::type_export::TypeExportFormat, String> {
        match value.map(|s| s.trim().to_ascii_lowercase()) {
            None => Ok(omni_compiler::type_export::TypeExportFormat::CHeader),
            Some(value) if value == "json" || value == "--json" => {
                Ok(omni_compiler::type_export::TypeExportFormat::Json)
            }
            Some(value)
                if value == "c" || value == "--c" || value == "header" || value == "cheader" =>
            {
                Ok(omni_compiler::type_export::TypeExportFormat::CHeader)
            }
            Some(value)
                if value == "python" || value == "--python" || value == "py" || value == "--py" =>
            {
                Ok(omni_compiler::type_export::TypeExportFormat::Python)
            }
            Some(other) => Err(format!("unknown bindgen format '{}'", other)),
        }
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
                let src = std::fs::read_to_string(path)
                    .map_err(|e| e.to_string())
                    .unwrap_or_else(|e| {
                        eprintln!("read error: {}", e);
                        std::process::exit(1);
                    });
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
                        println!("{}", omni_compiler::cst::format_cst(&cst, 0));
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
            "compile" => {
                eprintln!("Error: compile command not yet implemented");
                eprintln!("Available commands: parse-cst, fmt-cst, fmt, run, emit-mir, emit-lir, export-types, bindgen, check-abi, check");
                std::process::exit(1);
            }
            "emit-wasm" => {
                if args.len() < 3 {
                    eprintln!("Usage: omni-stage0 emit-wasm <file> [output.wasm]");
                    std::process::exit(2);
                }
                let path = Path::new(&args[2]);
                let output_path = if let Some(explicit) = args.get(3) {
                    std::path::PathBuf::from(explicit)
                } else {
                    let mut derived = path.to_path_buf();
                    derived.set_extension("wasm");
                    derived
                };
                match omni_compiler::emit_wasm_file(path) {
                    Ok(bytes) => {
                        if let Err(e) = std::fs::write(&output_path, &bytes) {
                            eprintln!("emit-wasm failed to write {}: {}", output_path.display(), e);
                            std::process::exit(1);
                        }
                        println!("Wrote {} ({} bytes)", output_path.display(), bytes.len());
                    }
                    Err(e) => {
                        eprintln!("emit-wasm failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "emit-lir" => {
                if args.len() < 3 {
                    eprintln!("Usage: omni-stage0 emit-lir <file>");
                    std::process::exit(2);
                }
                let path = Path::new(&args[2]);
                match omni_compiler::emit_lir_file(path) {
                    Ok(lir) => println!("{}", lir),
                    Err(e) => {
                        eprintln!("emit-lir failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "export-types" | "bindgen" => {
                if args.len() < 3 {
                    if args[1] == "bindgen" {
                        eprintln!("Usage: omni-stage0 bindgen <file> [--c|--json|--python]");
                    } else {
                        eprintln!("Usage: omni-stage0 export-types <file> [json|c|python]");
                    }
                    std::process::exit(2);
                }
                let path = Path::new(&args[2]);
                let format = if args[1] == "bindgen" {
                    match parse_bindgen_format(args.get(3).map(|s| s.as_str())) {
                        Ok(format) => format,
                        Err(e) => {
                            eprintln!("bindgen failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    let format = args.get(3).map(|s| s.as_str()).unwrap_or("json");
                    match omni_compiler::type_export::TypeExportFormat::parse(format) {
                        Ok(format) => format,
                        Err(e) => {
                            eprintln!("export-types failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                };
                match omni_compiler::export_types_file(path, format) {
                    Ok(output) => println!("{}", output),
                    Err(e) => {
                        eprintln!("{} failed: {}", args[1], e);
                        std::process::exit(1);
                    }
                }
            }
            "check-abi" => {
                if args.len() < 4 {
                    eprintln!("Usage: omni-stage0 check-abi <old-file> <new-file>");
                    std::process::exit(2);
                }
                let old_path = Path::new(&args[2]);
                let new_path = Path::new(&args[3]);
                match omni_compiler::check_abi_files(old_path, new_path) {
                    Ok(diffs) => {
                        if diffs.is_empty() {
                            println!("ABI compatible");
                        } else {
                            for diff in diffs {
                                println!("{}", diff);
                            }
                            std::process::exit(1);
                        }
                    }
                    Err(e) => {
                        eprintln!("check-abi failed: {}", e);
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
