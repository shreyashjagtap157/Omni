use crate::mir::{Instruction, MirModule};
use std::fs::File;
use std::io::Write;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn sanitize_ident(name: &str) -> String {
    let mut out = String::new();
    for (i, ch) in name.chars().enumerate() {
        if i == 0 {
            if ch.is_ascii_alphabetic() || ch == '_' {
                out.push(ch);
                continue;
            } else if ch.is_ascii_digit() {
                out.push('_');
                out.push(ch);
                continue;
            }
        }
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "v".to_string()
    } else {
        out
    }
}

fn escape_str(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

pub fn compile_and_run(module: &MirModule) -> Result<(), String> {
    let mut src = String::new();
    src.push_str(
        "#![allow(unused_variables, unused_assignments, dead_code, unused_mut, unused_imports)]\n",
    );
    src.push_str("fn main() {\n");

    let mut vars = Vec::new();
    for f in &module.functions {
        for b in &f.blocks {
            for instr in &b.instrs {
                match instr {
                    Instruction::ConstInt { dest, .. } => vars.push(dest.clone()),
                    Instruction::ConstStr { dest, .. } => vars.push(dest.clone()),
                    Instruction::ConstBool { dest, .. } => vars.push(dest.clone()),
                    Instruction::Move { dest, src } => {
                        vars.push(dest.clone());
                        vars.push(src.clone());
                    }
                    Instruction::Print { src } => vars.push(src.clone()),
                    Instruction::Drop { var } => vars.push(var.clone()),
                    Instruction::Jump { .. }
                    | Instruction::JumpIf { .. }
                    | Instruction::Label { .. } => {}
                    Instruction::BinaryOp { dest, .. } => vars.push(dest.clone()),
                    Instruction::UnaryOp { dest, .. } => vars.push(dest.clone()),
                    Instruction::Return { .. } => {}
                    Instruction::Assign { dest, src } => {
                        vars.push(dest.clone());
                        vars.push(src.clone());
                    }
                    Instruction::Call { dest, .. } => vars.push(dest.clone()),
                    Instruction::FieldAccess { dest, .. } => vars.push(dest.clone()),
                    Instruction::StructAccess { dest, .. } => vars.push(dest.clone()),
                    Instruction::IndexAccess { dest, .. } => vars.push(dest.clone()),
                    Instruction::LinearMove { dest, src } => {
                        vars.push(dest.clone());
                        vars.push(src.clone());
                    }
                    Instruction::DropLinear { var } => vars.push(var.clone()),
                    Instruction::StructDef { .. } => {}
                    Instruction::EnumDef { .. } => {}
                }
            }
        }
    }

    vars.sort();
    vars.dedup();

    for v in &vars {
        let id = sanitize_ident(v);
        src.push_str(&format!("    let mut {}: Option<&str> = None;\n", id));
    }

    for f in &module.functions {
        for b in &f.blocks {
            for instr in &b.instrs {
                match instr {
                    Instruction::ConstInt { dest, value } => {
                        let id = sanitize_ident(dest);
                        src.push_str(&format!(
                            "    {} = Some(Box::leak(Box::new(format!(\"{}\"))));\n",
                            id, value
                        ));
                    }
                    Instruction::ConstStr { dest, value } => {
                        let id = sanitize_ident(dest);
                        let esc = escape_str(value);
                        src.push_str(&format!("    {} = Some(\"{}\");\n", id, esc));
                    }
                    Instruction::ConstBool { dest, value } => {
                        let id = sanitize_ident(dest);
                        src.push_str(&format!("    {} = Some(\"{}\");\n", id, value));
                    }
                    Instruction::Move {
                        dest,
                        src: src_name,
                    } => {
                        let idd = sanitize_ident(dest);
                        let ids = sanitize_ident(src_name);
                        src.push_str(&format!("    {} = {}.clone();\n", idd, ids));
                    }
                    Instruction::Drop { var } => {
                        let id = sanitize_ident(var);
                        src.push_str(&format!("    {} = None;\n", id));
                    }
                    Instruction::Print { src: src_name } => {
                        let ids = sanitize_ident(src_name);
                        src.push_str(&format!(
                            "    if let Some(v) = {} {{ println!(\"{{}}\", v); }}\n",
                            ids
                        ));
                    }
                    Instruction::BinaryOp { dest, .. } | Instruction::UnaryOp { dest, .. } => {
                        let id = sanitize_ident(dest);
                        src.push_str(&format!("    {} = Some(\"0\");\n", id));
                    }
                    Instruction::Assign { dest, .. }
                    | Instruction::Call { dest, .. }
                    | Instruction::FieldAccess { dest, .. }
                    | Instruction::StructAccess { dest, .. }
                    | Instruction::IndexAccess { dest, .. } => {
                        let id = sanitize_ident(dest);
                        src.push_str(&format!("    {} = Some(\"0\");\n", id));
                    }
                    Instruction::Jump { .. }
                    | Instruction::JumpIf { .. }
                    | Instruction::Label { .. }
                    | Instruction::Return { .. } => {}
                    Instruction::LinearMove { dest, src: src_var } => {
                        let idd = sanitize_ident(dest);
                        let ids = sanitize_ident(src_var);
                        src.push_str(&format!("    {} = {}.clone();\n", idd, ids));
                    }
                    Instruction::DropLinear { var } => {
                        let id = sanitize_ident(var);
                        src.push_str(&format!("    {} = None;\n", id));
                    }
                    Instruction::StructDef { .. } => {}
                    Instruction::EnumDef { .. } => {}
                }
            }
        }
    }

    src.push_str("}\n");

    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let pid = std::process::id();
    let thread_id = format!("{:?}", std::thread::current().id())
        .replace("ThreadId(", "")
        .replace(")", "");
    path.push(format!("omni_codegen_{}_{}_{}.rs", pid, thread_id, nanos));
    let src_path = path.clone();
    let bin_path = path.with_extension(std::env::consts::EXE_EXTENSION);

    if bin_path.exists() {
        let _ = std::fs::remove_file(&bin_path);
    }

    let mut f = File::create(&src_path).map_err(|e| e.to_string())?;
    f.write_all(src.as_bytes()).map_err(|e| e.to_string())?;

    let status = Command::new("rustc")
        .arg(&src_path)
        .arg("-O")
        .arg("-o")
        .arg(&bin_path)
        .status()
        .map_err(|e| format!("failed to spawn rustc: {}", e))?;

    if !status.success() {
        return Err("rustc failed".to_string());
    }

    let run = Command::new(&bin_path)
        .status()
        .map_err(|e| format!("failed to run generated binary: {}", e))?;

    if !run.success() {
        return Err("generated program failed".to_string());
    }

    Ok(())
}
