//! Experimental WebAssembly artifact backend.
//!
//! WebAssembly is not Omni's canonical CPU execution model. The experimental backend only
//! qualifies a deliberately narrow, parameterless, straight-line scalar LIR
//! subset. Any instruction outside that subset is rejected explicitly.

use lir::{Instr, Module, Type};
use std::collections::HashMap;
use wasm_encoder::{
    CodeSection, EntityType, ExportKind, ExportSection, Function, FunctionSection, ImportSection,
    Instruction as WasmInstruction, Module as WasmModule, TypeSection, ValType,
};

pub fn emit_wasm_bytes(module: &Module) -> Result<Vec<u8>, String> {
    validate_subset(module)?;

    let mut wasm_module = WasmModule::new();
    let mut type_section = TypeSection::new();
    let mut import_section = ImportSection::new();
    let mut function_section = FunctionSection::new();
    let mut export_section = ExportSection::new();
    let mut code_section = CodeSection::new();

    // Optional scalar print import: env.host_print(i64) -> ().
    type_section.ty().function([ValType::I64], []);
    import_section.import("env", "host_print", EntityType::Function(0));

    let mut function_indices = HashMap::new();
    for (index, function) in module.functions.iter().enumerate() {
        let results = function
            .rets
            .iter()
            .map(lir_type_to_val_type)
            .collect::<Result<Vec<_>, _>>()?;
        let type_index = (index as u32) + 1;
        type_section.ty().function([], results);
        function_section.function(type_index);
        let wasm_index = (index as u32) + 1;
        if function_indices
            .insert(function.name.clone(), wasm_index)
            .is_some()
        {
            return Err(format!("duplicate Wasm function '{}'", function.name));
        }
        export_section.export(&function.name, ExportKind::Func, wasm_index);
    }

    wasm_module.section(&type_section);
    wasm_module.section(&import_section);
    wasm_module.section(&function_section);
    wasm_module.section(&export_section);

    for function in &module.functions {
        let mut wasm_function = Function::new(Vec::new());
        for instr in &function.body {
            match instr {
                Instr::Const(value) => {
                    wasm_function.instruction(&WasmInstruction::I64Const(*value));
                }
                Instr::Add => {
                    wasm_function.instruction(&WasmInstruction::I64Add);
                }
                Instr::Sub => {
                    wasm_function.instruction(&WasmInstruction::I64Sub);
                }
                Instr::Mul => {
                    wasm_function.instruction(&WasmInstruction::I64Mul);
                }
                Instr::Div => {
                    wasm_function.instruction(&WasmInstruction::I64DivS);
                }
                Instr::Mod => {
                    wasm_function.instruction(&WasmInstruction::I64RemS);
                }
                Instr::Lt => {
                    wasm_function.instruction(&WasmInstruction::I64LtS);
                }
                Instr::Gt => {
                    wasm_function.instruction(&WasmInstruction::I64GtS);
                }
                Instr::Le => {
                    wasm_function.instruction(&WasmInstruction::I64LeS);
                }
                Instr::Ge => {
                    wasm_function.instruction(&WasmInstruction::I64GeS);
                }
                Instr::Eq => {
                    wasm_function.instruction(&WasmInstruction::I64Eq);
                }
                Instr::Ne => {
                    wasm_function.instruction(&WasmInstruction::I64Ne);
                }
                Instr::Not => {
                    wasm_function.instruction(&WasmInstruction::I64Eqz);
                }
                Instr::Call(name) if name == "print" => {
                    wasm_function.instruction(&WasmInstruction::Call(0));
                }
                Instr::Call(name) => {
                    let index = *function_indices
                        .get(name)
                        .ok_or_else(|| format!("Wasm call target '{name}' is not defined"))?;
                    wasm_function.instruction(&WasmInstruction::Call(index));
                }
                Instr::Ret => {
                    wasm_function.instruction(&WasmInstruction::Return);
                }
                Instr::Nop => {
                    wasm_function.instruction(&WasmInstruction::Nop);
                }
                unsupported => {
                    return Err(format!(
                        "Wasm scalar experiment rejected unsupported instruction {unsupported:?}"
                    ));
                }
            };
        }
        wasm_function.instruction(&WasmInstruction::End);
        code_section.function(&wasm_function);
    }

    wasm_module.section(&code_section);
    Ok(wasm_module.finish())
}

fn validate_subset(module: &Module) -> Result<(), String> {
    if module.functions.is_empty() {
        return Err("Wasm module contains no functions".to_string());
    }
    for function in &module.functions {
        if !function.params.is_empty() {
            return Err(format!(
                "Wasm scalar experiment does not yet lower parameters for function '{}'",
                function.name
            ));
        }
        if function.rets.len() > 1 || function.rets.iter().any(|ty| *ty != Type::I64) {
            return Err(format!(
                "Wasm scalar experiment supports only zero or one i64 return in '{}'",
                function.name
            ));
        }
        for instr in &function.body {
            match instr {
                Instr::Const(_)
                | Instr::Add
                | Instr::Sub
                | Instr::Mul
                | Instr::Div
                | Instr::Mod
                | Instr::Lt
                | Instr::Gt
                | Instr::Le
                | Instr::Ge
                | Instr::Eq
                | Instr::Ne
                | Instr::Not
                | Instr::Call(_)
                | Instr::Ret
                | Instr::Nop => {}
                other => {
                    return Err(format!(
                        "Wasm scalar experiment does not support {other:?} in '{}'",
                        function.name
                    ));
                }
            }
        }
    }
    Ok(())
}

fn lir_type_to_val_type(ty: &Type) -> Result<ValType, String> {
    match ty {
        Type::I64 => Ok(ValType::I64),
        Type::Ptr(_) => Err("Wasm pointer ABI is not qualified in Omni v0.1.4".to_string()),
        Type::Void => Err("void is not a Wasm value type".to_string()),
    }
}

#[cfg(test)]
pub fn compile_and_validate(module: &Module) -> Result<Vec<u8>, String> {
    let wasm_bytes = emit_wasm_bytes(module)?;
    wasmparser::Validator::new()
        .validate_all(&wasm_bytes)
        .map_err(|error| format!("wasm validation failed: {error}"))?;
    Ok(wasm_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lir::Function;

    #[test]
    fn emits_valid_straight_line_scalar_module() {
        let module = lir::example_module();
        let bytes = compile_and_validate(&module).expect("qualified subset");
        assert!(bytes.starts_with(b"\0asm"));
    }

    #[test]
    fn locals_fail_closed() {
        let mut module = Module::new();
        module.add_function(Function::new(
            "main",
            vec![],
            Type::I64,
            vec![
                Instr::Const(42),
                Instr::Store(0),
                Instr::Load(0),
                Instr::Ret,
            ],
            vec![],
        ));
        let error = emit_wasm_bytes(&module).expect_err("locals are not qualified");
        assert!(error.contains("does not support"));
    }
}
