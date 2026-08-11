fn main() {
    use std::path::Path;
    let mut args: Vec<String> = std::env::args().collect();

    // Check for global flags
    if let Some(idx) = args.iter().position(|a| a == "--types=minimal") {
        args.remove(idx);
    } else if let Some(idx) = args.iter().position(|a| a == "--types=verbose") {
        args.remove(idx);
    }

    fn print_usage() {
        eprintln!(
            "Usage: omni <command> [args]\nCommands: new, parse, lex, parse-cst, fmt-cst, fmt, fmt-check, check, run, run-native, run-jit, test(unqualified), emit-mir, check-mir, run-mir, build, compile, emit-wasm, emit-lir, export-types, bindgen, check-abi, doctor, --version"
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

    fn compile_source(path: &Path) -> Result<omni_compiler::driver::CompilationResult, String> {
        use omni_compiler::driver::{Backend, Compiler};
        prepare_omni_project(path)?;
        let source_path = resolve_omni_entry(path)?;
        let source =
            std::fs::read_to_string(&source_path).map_err(|e| format!("read error: {}", e))?;
        let compiler = Compiler::with_path(&source, Backend::Native, source_path);
        let result = compiler.compile();
        let has_errors = result
            .diagnostics
            .iter()
            .any(|d| matches!(d.severity, omni_compiler::diagnostics::Severity::Error));
        if has_errors {
            let errs: Vec<String> = result
                .diagnostics
                .iter()
                .filter(|d| matches!(d.severity, omni_compiler::diagnostics::Severity::Error))
                .map(|d| d.to_string())
                .collect();
            return Err(errs.join("\n"));
        }
        Ok(result)
    }

    if args.len() > 1 {
        match args[1].as_str() {
            "--version" | "-V" | "version" => {
                println!("{}", omni_compiler::version::version_banner());
                std::process::exit(0);
            }
            "help" | "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            "doctor" => {
                println!(
                    "Omni {} toolchain doctor",
                    omni_compiler::version::PROJECT_VERSION
                );
                println!("host: {}-{}", std::env::consts::ARCH, std::env::consts::OS);
                let native = std::env::consts::ARCH == "x86_64" && std::env::consts::OS == "linux";
                println!(
                    "owned native AOT (x86-64 Linux ELF): {}",
                    if native {
                        "available"
                    } else {
                        "not available on this host"
                    }
                );
                println!(
                    "Cranelift JIT execution: unqualified in v0.1.4.1 (historical source archived)"
                );
                println!(
                    "Wasm backend: {}",
                    if cfg!(feature = "wasm-backend") {
                        "enabled"
                    } else {
                        "disabled (opt in with --features wasm-backend)"
                    }
                );
                println!("LLVM execution: unqualified in v0.1.4.1; no LLVM SDK is required");
                println!("canonical execution: native AOT; JIT/interpreters are development oracles only");
                if !native {
                    println!("note: cross-target emission beyond x86-64 Linux is scheduled for later pre-1.0 milestones");
                }
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
                match omni_compiler::complete_lexer::tokenize_complete(&src) {
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
                    eprintln!("Usage: omni run <file>");
                    std::process::exit(2);
                }
                run_native_file(Path::new(&args[2]));
            }
            "fmt" => {
                if args.len() < 3 {
                    eprintln!("Usage: omni fmt <file> [--check] [--strict]");
                    std::process::exit(2);
                }
                let path = Path::new(&args[2]);
                let mut check_mode = false;
                let mut strict_mode = false;
                for option in &args[3..] {
                    match option.as_str() {
                        "--check" => check_mode = true,
                        "--strict" => strict_mode = true,
                        other => {
                            eprintln!("Unknown fmt option: {other}");
                            eprintln!("Usage: omni fmt <file> [--check] [--strict]");
                            std::process::exit(2);
                        }
                    }
                }

                let formatted = if strict_mode {
                    match omni_compiler::parser_utils::parse_file(path) {
                        Ok(program) => {
                            let config = omni_compiler::formatter::FormatterConfig::new()
                                .with_strict_mode(true);
                            Ok(omni_compiler::formatter::format_program_with_config(
                                &program, &config,
                            ))
                        }
                        Err(error) => Err(error.to_string()),
                    }
                } else {
                    omni_compiler::parse_cst_file(path)
                        .map(|cst| omni_compiler::formatter::format_cst_source(&cst))
                };

                match formatted {
                    Ok(out) => {
                        if check_mode {
                            let current = std::fs::read_to_string(path).unwrap_or_else(|error| {
                                eprintln!(
                                    "Format check could not read {}: {}",
                                    path.display(),
                                    error
                                );
                                std::process::exit(1);
                            });
                            if current != out {
                                eprintln!("File {} is not formatted correctly", path.display());
                                std::process::exit(1);
                            } else {
                                println!("File {} is formatted correctly", path.display());
                            }
                        } else {
                            std::fs::write(path, &out)
                                .map_err(|e| e.to_string())
                                .unwrap_or_else(|e| {
                                    eprintln!("Format write failed: {}", e);
                                    std::process::exit(1);
                                });
                            println!("Formatted {}", path.display());
                        }
                    }
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
                match compile_source(path) {
                    Ok(_) => println!("Type check OK"),
                    Err(e) => {
                        eprintln!("Type check failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "fix" => {
                eprintln!("omni fix is not available in the qualified v0.1.4 toolchain; no source file was modified");
                std::process::exit(2);
            }
            "doc" => {
                if args.len() < 3 {
                    eprintln!("Usage: omni-stage0 doc <file> [output.html]");
                    std::process::exit(2);
                }
                let path = Path::new(&args[2]);
                let out_path = args.get(3).map(|s| s.as_str()).unwrap_or("doc.html");
                match omni_compiler::parser_utils::parse_file(path) {
                    Ok(ast) => {
                        let items = omni_compiler::doc_gen::generate_docs(&ast.stmts);
                        let html = omni_compiler::doc_gen::generate_html(&items);
                        if let Err(e) = std::fs::write(out_path, html) {
                            eprintln!("Failed to write docs: {}", e);
                            std::process::exit(1);
                        }
                        println!("Generated docs to {}", out_path);
                    }
                    Err(e) => {
                        eprintln!("Doc generation failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "repl" => {
                eprintln!("Omni REPL execution is not qualified in v0.1.4; use `omni run` for canonical native AOT execution");
                std::process::exit(2);
            }
            "run-native" => {
                if args.len() < 3 {
                    eprintln!("Usage: omni run-native <file>");
                    std::process::exit(2);
                }
                run_native_file(Path::new(&args[2]));
            }
            "run-jit" => {
                eprintln!("Cranelift JIT execution is not qualified in Omni v0.1.4; use `omni run`/`omni run-native` for canonical native AOT execution");
                std::process::exit(2);
            }
            "compile" | "build" => {
                if args.len() < 3 {
                    eprintln!("Usage: omni build <file> [-o output] [--aot]");
                    std::process::exit(2);
                }
                let path = Path::new(&args[2]);

                // --aot remains accepted for compatibility; native AOT is now the default.
                let _legacy_aot = args.iter().any(|a| a == "--aot");
                let mut output_path: Option<std::path::PathBuf> = None;
                let mut idx = 3usize;
                while idx < args.len() {
                    match args[idx].as_str() {
                        "--aot" => idx += 1,
                        "-o" | "--output" => {
                            let Some(value) = args.get(idx + 1) else {
                                eprintln!("{} requires an output path", args[idx]);
                                std::process::exit(2);
                            };
                            output_path = Some(std::path::PathBuf::from(value));
                            idx += 2;
                        }
                        value if !value.starts_with('-') && output_path.is_none() => {
                            output_path = Some(std::path::PathBuf::from(value));
                            idx += 1;
                        }
                        other => {
                            eprintln!("unknown build option: {}", other);
                            std::process::exit(2);
                        }
                    }
                }

                let source_path = match resolve_omni_entry(path) {
                    Ok(path) => path,
                    Err(e) => {
                        eprintln!("{}", e);
                        std::process::exit(1);
                    }
                };

                if let Err(e) = prepare_omni_project(path) {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }

                let source = std::fs::read_to_string(&source_path).unwrap_or_else(|e| {
                    eprintln!("read error: {}", e);
                    std::process::exit(1);
                });
                let compiler = omni_compiler::driver::Compiler::with_path(
                    &source,
                    omni_compiler::driver::Backend::Native,
                    source_path.clone(),
                );
                let result = compiler.compile();
                let has_errors = result
                    .diagnostics
                    .iter()
                    .any(|d| matches!(d.severity, omni_compiler::diagnostics::Severity::Error));
                if has_errors {
                    let errs: Vec<String> = result
                        .diagnostics
                        .iter()
                        .filter(|d| {
                            matches!(d.severity, omni_compiler::diagnostics::Severity::Error)
                        })
                        .map(|d| d.to_string())
                        .collect();
                    eprintln!("Compilation failed:\n{}", errs.join("\n"));
                    std::process::exit(1);
                }

                let mir = result.mir.as_ref().unwrap_or_else(|| {
                    eprintln!("Compilation failed: no MIR produced");
                    std::process::exit(1);
                });
                let lir = omni_compiler::codegen_lir::lower_mir_to_lir(mir).unwrap_or_else(|e| {
                    eprintln!("LIR lowering failed: {}", e);
                    std::process::exit(1);
                });
                let out_path =
                    output_path.unwrap_or_else(|| default_native_output(path, &source_path));
                match omni_compiler::codegen::compile_to_aot(&lir, &out_path) {
                    Ok(actual_path) => {
                        println!("AOT compilation succeeded: {}", actual_path.display())
                    }
                    Err(e) => {
                        eprintln!("AOT compilation failed: {}", e);
                        std::process::exit(1);
                    }
                }
                std::process::exit(0);
            }
            "emit-wasm" => {
                if !cfg!(feature = "wasm-backend") {
                    eprintln!(
                        "emit-wasm is optional; rebuild/install with --features wasm-backend"
                    );
                    std::process::exit(2);
                }
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
                let source = std::fs::read_to_string(path).unwrap_or_else(|e| {
                    eprintln!("read error: {}", e);
                    std::process::exit(1);
                });
                let compiler = omni_compiler::driver::Compiler::new(
                    &source,
                    omni_compiler::driver::Backend::Wasm,
                );
                let result = compiler.compile();
                let has_errors = result
                    .diagnostics
                    .iter()
                    .any(|d| matches!(d.severity, omni_compiler::diagnostics::Severity::Error));
                if has_errors {
                    let errs: Vec<String> = result
                        .diagnostics
                        .iter()
                        .filter(|d| {
                            matches!(d.severity, omni_compiler::diagnostics::Severity::Error)
                        })
                        .map(|d| d.to_string())
                        .collect();
                    eprintln!("emit-wasm failed with errors:\n{}", errs.join("\n"));
                    std::process::exit(1);
                }
                if let Some(bytes) = result.wasm_output {
                    if let Err(e) = std::fs::write(&output_path, &bytes) {
                        eprintln!("emit-wasm failed to write {}: {}", output_path.display(), e);
                        std::process::exit(1);
                    }
                    println!("Wrote {} ({} bytes)", output_path.display(), bytes.len());
                } else {
                    eprintln!("emit-wasm failed: compilation produced no WASM output.");
                    std::process::exit(1);
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
            "new" => {
                if args.len() < 3 {
                    eprintln!("Usage: omni-stage0 new <name>");
                    std::process::exit(2);
                }
                let project_dir = std::path::PathBuf::from(&args[2]);
                let package_name = package_name_for_new_project(&project_dir).unwrap_or_else(|e| {
                    eprintln!("invalid Omni project name: {}", e);
                    std::process::exit(2);
                });
                if project_dir.exists() {
                    eprintln!("Directory '{}' already exists", project_dir.display());
                    std::process::exit(1);
                }
                std::fs::create_dir_all(&project_dir).unwrap_or_else(|e| {
                    eprintln!("Failed to create project directory: {}", e);
                    std::process::exit(1);
                });
                std::fs::create_dir_all(project_dir.join("src")).unwrap_or_else(|e| {
                    eprintln!("Failed to create src directory: {}", e);
                    std::process::exit(1);
                });
                let omni_toml = format!(
                    r#"[package]
name = "{}"
version = "0.1.0"
edition = "2026"

[dependencies]
"#,
                    package_name
                );
                std::fs::write(project_dir.join("omni.toml"), omni_toml).unwrap_or_else(|e| {
                    eprintln!("Failed to write omni.toml: {}", e);
                    std::process::exit(1);
                });
                let main_content = r#"fn main() -> i64 {
    print "Hello, world!";
    return 0;
}
"#;
                std::fs::write(project_dir.join("src").join("main.omni"), main_content)
                    .unwrap_or_else(|e| {
                        eprintln!("Failed to write src/main.omni: {}", e);
                        std::process::exit(1);
                    });
                println!(
                    "Created Omni project '{}' in {}",
                    package_name,
                    project_dir.display()
                );
            }
            "fmt-check" => {
                if args.len() < 3 {
                    eprintln!("Usage: omni-stage0 fmt-check <file>");
                    std::process::exit(2);
                }
                let path = Path::new(&args[2]);
                let source = std::fs::read_to_string(path).unwrap_or_else(|e| {
                    eprintln!("read error: {}", e);
                    std::process::exit(1);
                });
                match omni_compiler::formatter::check_format(&source) {
                    Ok(()) => println!("Formatted correctly"),
                    Err(diff) => {
                        eprintln!("Formatting check failed:\n{}", diff);
                        std::process::exit(1);
                    }
                }
            }
            "test" => {
                eprintln!(
                    "omni test is not qualified in v0.1.4-r1: the historical MIR-VM runner is a development oracle, not canonical Omni execution"
                );
                eprintln!(
                    "use scripts/native-conformance.py for the qualified native test wedge; a native @test runner is scheduled for a later milestone"
                );
                std::process::exit(2);
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

fn nearest_project_root(path: &std::path::Path) -> Option<std::path::PathBuf> {
    if path.is_dir() {
        return path.join("omni.toml").is_file().then(|| path.to_path_buf());
    }
    let start = path.parent()?;
    for dir in start.ancestors() {
        if dir.join("omni.toml").is_file() {
            return Some(dir.to_path_buf());
        }
    }
    None
}

fn prepare_omni_project(path: &std::path::Path) -> Result<(), String> {
    let Some(project_root) = nearest_project_root(path) else {
        // A source file outside a manifest-backed project is the supported
        // Stage-0 single-file mode. Directories must always be projects.
        if path.is_dir() {
            return Err(format!(
                "Omni project '{}' is missing omni.toml",
                path.display()
            ));
        }
        return Ok(());
    };
    let manifest_path = project_root.join("omni.toml");
    let manifest = omni_compiler::package::omni_toml_parser::load_manifest(&manifest_path)
        .map_err(|e| format!("Failed to load {}: {}", manifest_path.display(), e))?;
    let build_cfg =
        omni_compiler::package::build_script::BuildConfig::new(project_root.clone(), &manifest);
    omni_compiler::package::build_script::run_build_script(&build_cfg)
        .map_err(|e| format!("Failed to run build.omni: {}", e))?;
    let lockfile_path = project_root.join("omni.lock");
    omni_compiler::package::resolve_and_write_lockfile(&manifest, &lockfile_path)
        .map_err(|e| format!("Failed to generate lockfile: {}", e))?;
    Ok(())
}

fn resolve_omni_entry(path: &std::path::Path) -> Result<std::path::PathBuf, String> {
    if path.is_file() {
        return Ok(path.to_path_buf());
    }
    if path.is_dir() {
        let manifest = path.join("omni.toml");
        if !manifest.is_file() {
            return Err(format!(
                "Omni project '{}' is missing omni.toml",
                path.display()
            ));
        }
        let entry = path.join("src").join("main.omni");
        if !entry.is_file() {
            return Err(format!(
                "Omni project '{}' is missing src/main.omni",
                path.display()
            ));
        }
        return Ok(entry);
    }
    Err(format!("Omni input '{}' does not exist", path.display()))
}

fn default_native_output(input: &std::path::Path, source: &std::path::Path) -> std::path::PathBuf {
    if input.is_dir() {
        let name =
            omni_compiler::package::omni_toml_parser::load_manifest(&input.join("omni.toml"))
                .ok()
                .map(|m| m.name)
                .filter(|n| !n.trim().is_empty())
                .or_else(|| input.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "omni-app".to_string());
        return input.join("target").join("omni").join(name);
    }
    let mut out = source.to_path_buf();
    out.set_extension(std::env::consts::EXE_EXTENSION);
    if out == source {
        out.set_file_name(format!(
            "{}{}",
            source.file_name().unwrap_or_default().to_string_lossy(),
            if std::env::consts::EXE_EXTENSION.is_empty() {
                ".out"
            } else {
                ""
            }
        ));
    }
    out
}

fn run_native_file(path: &std::path::Path) -> ! {
    use omni_compiler::driver::{Backend, Compiler};
    prepare_omni_project(path).unwrap_or_else(|e| {
        eprintln!("{}", e);
        std::process::exit(1);
    });
    let source_path = resolve_omni_entry(path).unwrap_or_else(|e| {
        eprintln!("{}", e);
        std::process::exit(1);
    });
    let source = std::fs::read_to_string(&source_path).unwrap_or_else(|e| {
        eprintln!("read error: {}", e);
        std::process::exit(1);
    });
    let result = Compiler::with_path(&source, Backend::Native, source_path).compile();
    if result
        .diagnostics
        .iter()
        .any(|d| matches!(d.severity, omni_compiler::diagnostics::Severity::Error))
    {
        for d in &result.diagnostics {
            eprintln!("{}", d);
        }
        std::process::exit(1);
    }
    let mir = result.mir.as_ref().unwrap_or_else(|| {
        eprintln!("native execution failed: no MIR produced");
        std::process::exit(1);
    });
    let lir = omni_compiler::codegen_lir::lower_mir_to_lir(mir).unwrap_or_else(|e| {
        eprintln!("native execution failed during LIR lowering: {}", e);
        std::process::exit(1);
    });
    match omni_compiler::codegen::compile_and_run_aot(&lir) {
        Ok(native) => {
            use std::io::Write;
            if let Err(e) = std::io::stdout().write_all(&native.stdout) {
                eprintln!("failed to write native stdout: {}", e);
                std::process::exit(1);
            }
            if let Err(e) = std::io::stderr().write_all(&native.stderr) {
                eprintln!("failed to write native stderr: {}", e);
                std::process::exit(1);
            }
            std::process::exit(native.status.unwrap_or(1));
        }
        Err(e) => {
            eprintln!("native execution failed: {}", e);
            std::process::exit(1);
        }
    }
}

fn package_name_for_new_project(path: &std::path::Path) -> Result<String, String> {
    let name = path
        .file_name()
        .and_then(|part| part.to_str())
        .ok_or_else(|| "project path must end in a valid UTF-8 package name".to_string())?;
    let mut chars = name.chars();
    let first = chars
        .next()
        .ok_or_else(|| "package name cannot be empty".to_string())?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(format!(
            "'{}' must begin with an ASCII letter or underscore",
            name
        ));
    }
    if !chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-')) {
        return Err(format!(
            "'{}' may contain only ASCII letters, digits, '_' and '-'",
            name
        ));
    }
    Ok(name.to_string())
}
