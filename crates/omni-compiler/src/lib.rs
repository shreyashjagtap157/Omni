//! Omni Compiler Library

pub fn version() -> &'static str {
    "0.1.0"
}

pub mod ast;
pub mod async_effects;
pub mod codegen;
pub mod codegen_lir;
pub mod codegen_rust;
pub mod comptime;
pub mod cst;
pub mod diagnostics;
pub mod formatter;
pub mod interpreter;
pub mod lexer;
pub mod lsp;
#[cfg(feature = "use_salsa_lsp")]
pub mod lsp_salsa_db;
pub mod lsp_incr_db;
pub mod macros;
pub mod mir;
pub mod mir_optimize;
pub mod parser;
pub mod polonius;
pub mod resolver;
pub mod traits;
pub mod type_checker;
pub mod vm;

use std::path::Path;

fn is_stdlib_file(path: &Path) -> bool {
    path.ends_with(Path::new("omni").join("stdlib").join("core.omni"))
        || path.ends_with(Path::new("omni").join("stdlib").join("collections.omni"))
}

fn read_source_with_stdlib(path: &Path) -> Result<String, String> {
    let mut source = String::new();
    if !is_stdlib_file(path) {
        for stdlib_path in [
            Path::new("omni/stdlib/core.omni"),
            Path::new("omni/stdlib/collections.omni"),
        ] {
            if stdlib_path.exists() {
                let stdlib_src = std::fs::read_to_string(stdlib_path).map_err(|e| e.to_string())?;
                source.push_str(&stdlib_src);
                source.push('\n');
            }
        }
    }
    let file_src = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    source.push_str(&file_src);
    Ok(source)
}

pub fn parse_file(path: &Path) -> Result<ast::Program, String> {
    let src = read_source_with_stdlib(path)?;
    let mut lexer = lexer::Lexer::new(&src);
    let tokens = lexer.tokenize()?;
    let mut parser = parser::Parser::new(tokens);
    parser.parse_program()
}

pub fn parse_cst_file(path: &Path) -> Result<cst::SyntaxNode, String> {
    let src = read_source_with_stdlib(path)?;
    let mut lexer = lexer::Lexer::new(&src);
    let tokens = lexer.tokenize()?;
    Ok(cst::build_cst(&tokens))
}

pub fn run_file(path: &Path) -> Result<(), String> {
    let program = parse_file(path)?;
    type_checker::type_check_program(&program)?;
    interpreter::run_program(&program)
}

pub fn format_file(path: &Path) -> Result<(), String> {
    let cst = parse_cst_file(path)?;
    let formatted = formatter::format_cst_source(&cst);
    std::fs::write(path, formatted).map_err(|e| e.to_string())
}

pub fn check_file(path: &Path) -> Result<(), String> {
    let program = parse_file(path)?;
    type_checker::type_check_program(&program)
}

pub fn emit_mir_file(path: &Path) -> Result<String, String> {
    let program = parse_file(path)?;
    let module = mir::lower_program_to_mir(&program);
    Ok(mir::format_mir(&module))
}

pub fn emit_lir_file(path: &Path) -> Result<String, String> {
    let program = parse_file(path)?;
    let mut module = mir::lower_program_to_mir(&program);
    mir_optimize::run_mir_optimizations(&mut module);
    let lir = codegen_lir::lower_mir_to_lir(&module);
    Ok(codegen_lir::compile_lir_module_text(&lir))
}

pub fn compile_lir_file(path: &Path) -> Result<String, String> {
    emit_lir_file(path)
}

pub fn check_mir_file(path: &Path) -> Result<(), String> {
    let program = parse_file(path)?;
    let module = mir::lower_program_to_mir(&program);
    polonius::check_mir(&module)
}

pub fn run_mir_vm_file(path: &Path) -> Result<(), String> {
    let program = parse_file(path)?;
    let module = mir::lower_program_to_mir(&program);
    vm::run_mir_module(&module)
}

pub fn run_native_file(path: &Path) -> Result<(), String> {
    let program = parse_file(path)?;
    let mut module = mir::lower_program_to_mir(&program);
    mir_optimize::run_mir_optimizations(&mut module);

    if requires_rust_emitter(&module) {
        return codegen_rust::compile_and_run(&module);
    }

    let lir = codegen_lir::lower_mir_to_lir(&module);
    let _ = codegen::compile_and_run(&lir)?;
    Ok(())
}

fn requires_rust_emitter(module: &mir::MirModule) -> bool {
    module.functions.iter().any(|function| {
        function.blocks.iter().any(|block| {
            block.instrs.iter().any(|instr| {
                matches!(
                    instr,
                    mir::Instruction::ConstStr { .. }
                        | mir::Instruction::FieldAccess { .. }
                        | mir::Instruction::StructAccess { .. }
                        | mir::Instruction::IndexAccess { .. }
                        | mir::Instruction::StructDef { .. }
                        | mir::Instruction::EnumDef { .. }
                )
            })
        })
    })
}
