use lir::{Instr, Module, Type};
use std::collections::HashMap;
use wasm_encoder::{
    CodeSection, EntityType, ExportKind, ExportSection, Function, FunctionSection,
    ImportSection, Instruction as WasmInstruction, Module as WasmModule, TypeSection, ValType,
};

pub fn emit_wasm_bytes(module: &Module) -> Result<Vec<u8>, String> {
    if module.functions.iter().any(|function| function.rets.len() > 1) {
        return Err("multi-return wasm emission is not implemented yet".to_string());
    }

    let mut wasm_module = WasmModule::new();
    let mut type_section = TypeSection::new();
    let mut import_section = ImportSection::new();
    let mut function_section = FunctionSection::new();
    let mut export_section = ExportSection::new();
    let mut code_section = CodeSection::new();

    let host_print_type_index = 0u32;
    type_section.ty().function([ValType::I64], []);

    let mut function_type_indices = Vec::new();
    let mut next_type_index = 1u32;
    for function in &module.functions {
        let params = function
            .params
            .iter()
            .map(lir_type_to_val_type)
            .collect::<Result<Vec<_>, _>>()?;
        let results = function
            .rets
            .iter()
            .map(lir_type_to_val_type)
            .collect::<Result<Vec<_>, _>>()?;

        type_section.ty().function(params, results);
        function_type_indices.push(next_type_index);
        next_type_index += 1;
    }

    wasm_module.section(&type_section);
    import_section.import(
        "env",
        "host_print",
        EntityType::Function(host_print_type_index),
    );
    wasm_module.section(&import_section);

    let mut function_indices = HashMap::new();
    for (index, function) in module.functions.iter().enumerate() {
        function_section.function(function_type_indices[index]);
        let wasm_index = (index as u32) + 1;
        function_indices.insert(function.name.clone(), wasm_index);
        export_section.export(&function.name, ExportKind::Func, wasm_index);
    }

    wasm_module.section(&function_section);
    wasm_module.section(&export_section);

    for function in &module.functions {
        let max_slot = function
            .body
            .iter()
            .filter_map(|instr| match instr {
                Instr::Load(slot) | Instr::Store(slot) | Instr::Drop(slot) => Some(*slot),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        let locals = if max_slot == 0 && !function.body.iter().any(|instr| matches!(instr, Instr::Load(0) | Instr::Store(0) | Instr::Drop(0))) {
            Vec::new()
        } else {
            vec![(max_slot + 1, ValType::I64)]
        };

        let mut wasm_function = Function::new(locals);
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
                Instr::Call(name) => {
                    let call_index = if name == "print" && !function_indices.contains_key(name) {
                        0u32
                    } else {
                        *function_indices.get(name).ok_or_else(|| {
                            format!("wasm backend cannot resolve call target '{}'", name)
                        })?
                    };
                    wasm_function.instruction(&WasmInstruction::Call(call_index));
                }
                Instr::Load(slot) => {
                    wasm_function.instruction(&WasmInstruction::LocalGet(*slot));
                }
                Instr::Store(slot) => {
                    wasm_function.instruction(&WasmInstruction::LocalSet(*slot));
                }
                Instr::Ret => {
                    wasm_function.instruction(&WasmInstruction::Return);
                }
                Instr::Nop => {
                    wasm_function.instruction(&WasmInstruction::Nop);
                }
                Instr::Drop(_) => {
                    wasm_function.instruction(&WasmInstruction::Drop);
                }
                Instr::Jump(_) | Instr::CondJump { .. } => {
                    return Err(format!(
                        "wasm backend does not yet support control flow in function '{}'",
                        function.name
                    ));
                }
            }
        }
        code_section.function(&wasm_function);
    }

    wasm_module.section(&code_section);
    Ok(wasm_module.finish())
}

fn lir_type_to_val_type(ty: &Type) -> Result<ValType, String> {
    match ty {
        Type::I64 | Type::Ptr => Ok(ValType::I64),
        Type::Void => Err("void is not a valid wasm parameter or result type".to_string()),
    }
}
