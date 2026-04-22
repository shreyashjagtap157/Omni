use lir::{Instr, Module, Type};
use std::collections::HashMap;
use wasm_encoder::{
    CodeSection, EntityType, ExportKind, ExportSection, Function, FunctionSection, ImportSection,
    Instruction as WasmInstruction, Module as WasmModule, TypeSection, ValType,
};

pub fn emit_wasm_bytes(module: &Module) -> Result<Vec<u8>, String> {
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
        let locals = if max_slot == 0
            && !function
                .body
                .iter()
                .any(|instr| matches!(instr, Instr::Load(0) | Instr::Store(0) | Instr::Drop(0)))
        {
            Vec::new()
        } else {
            vec![(max_slot + 1, ValType::I64)]
        };

        let mut wasm_function = Function::new(locals);

        // Collect all jump targets to determine which need block labels
        let mut jump_targets: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for (_idx, instr) in function.body.iter().enumerate() {
            match instr {
                Instr::Jump(target) => {
                    jump_targets.insert(*target);
                }
                Instr::CondJump { if_true, if_false } => {
                    jump_targets.insert(*if_true);
                    jump_targets.insert(*if_false);
                }
                _ => {}
            }
        }

        // Create a mapping from instruction index to block depth
        // We use wasm blocks to implement control flow
        let mut block_depths: Vec<u32> = vec![0; function.body.len()];
        let mut current_depth: u32 = 0;

        for idx in 0..function.body.len() {
            if jump_targets.contains(&idx) {
                // This instruction is a jump target, start a new block
                current_depth += 1;
            }
            block_depths[idx] = current_depth;
        }

        // Emit instructions
        for (idx, instr) in function.body.iter().enumerate() {
            // If this is a jump target, start a block
            if jump_targets.contains(&idx) {
                wasm_function.instruction(&WasmInstruction::Block(wasm_encoder::BlockType::Empty));
            }

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
                Instr::Jump(target) => {
                    // Calculate relative branch depth
                    let current_depth = block_depths[idx];
                    let target_depth = if *target < function.body.len() {
                        block_depths[*target]
                    } else {
                        0
                    };

                    if *target > idx {
                        // Forward jump - br to a nested block
                        let depth = (target_depth - current_depth) as u32;
                        wasm_function.instruction(&WasmInstruction::Br(depth));
                    } else {
                        // Backward jump (loop) - br to outer block
                        let depth = (current_depth - target_depth) as u32;
                        wasm_function.instruction(&WasmInstruction::Br(depth));
                    }
                }
                Instr::CondJump { if_true, if_false } => {
                    // For conditional jumps, we need to implement if-else logic
                    // This is a simplified implementation that uses br_if

                    // The top of stack should be the condition (0 or non-zero)
                    // Convert i64 to i32 for br_if
                    wasm_function.instruction(&WasmInstruction::I32WrapI64);

                    let current_depth = block_depths[idx];
                    let true_depth = if *if_true < function.body.len() {
                        block_depths[*if_true]
                    } else {
                        0
                    };
                    let false_depth = if *if_false < function.body.len() {
                        block_depths[*if_false]
                    } else {
                        0
                    };

                    // br_if jumps when condition is true
                    if *if_true > idx {
                        // Forward jump to then block
                        let depth = (true_depth - current_depth) as u32;
                        wasm_function.instruction(&WasmInstruction::BrIf(depth));
                    } else {
                        // Backward jump
                        let depth = (current_depth - true_depth) as u32;
                        wasm_function.instruction(&WasmInstruction::BrIf(depth));
                    }

                    // If we didn't take the true branch, we need to handle the false branch
                    // This is a simplified version - in practice, we might need more complex block structure
                    if *if_false > idx {
                        // Forward jump to else block
                        let depth = (false_depth - current_depth) as u32;
                        wasm_function.instruction(&WasmInstruction::Br(depth));
                    }
                }
            }
        }

        // Close any open blocks
        for _ in 0..=*block_depths.last().unwrap_or(&0) {
            wasm_function.instruction(&WasmInstruction::End);
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

/// Compile and validate a WASM module from LIR
#[cfg(test)]
pub fn compile_and_validate(module: &Module) -> Result<Vec<u8>, String> {
    let wasm_bytes = emit_wasm_bytes(module)?;

    // Validate using wasmparser
    wasmparser::Validator::new()
        .validate_all(&wasm_bytes)
        .map_err(|e| format!("wasm validation failed: {}", e))?;

    Ok(wasm_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lir::Function;

    #[test]
    fn test_emit_simple_module() {
        let module = lir::example_module();
        let wasm = emit_wasm_bytes(&module).expect("should emit wasm");
        assert!(!wasm.is_empty());

        // Parse with wasmparser to validate
        wasmparser::Validator::new()
            .validate_all(&wasm)
            .expect("wasm should be valid");
    }

    #[test]
    fn test_compile_and_validate() {
        let module = lir::example_module();
        let wasm = compile_and_validate(&module).expect("should compile and validate");
        assert!(!wasm.is_empty());
    }

    #[test]
    fn test_local_slots() {
        let mut m = Module::new();
        let f = Function::new(
            "slot_test",
            vec![],
            Type::I64,
            vec![
                Instr::Const(42),
                Instr::Store(0),
                Instr::Load(0),
                Instr::Ret,
            ],
        );
        m.add_function(f);

        let wasm = emit_wasm_bytes(&m).expect("should emit");
        wasmparser::Validator::new()
            .validate_all(&wasm)
            .expect("should be valid");
    }

    #[test]
    fn test_function_call() {
        let mut m = Module::new();
        let callee = Function::new(
            "callee",
            vec![Type::I64],
            Type::I64,
            vec![Instr::Load(0), Instr::Ret],
        );
        let caller = Function::new(
            "caller",
            vec![],
            Type::I64,
            vec![
                Instr::Const(42),
                Instr::Call("callee".to_string()),
                Instr::Ret,
            ],
        );
        m.add_function(callee);
        m.add_function(caller);

        let wasm = emit_wasm_bytes(&m).expect("should emit");
        wasmparser::Validator::new()
            .validate_all(&wasm)
            .expect("should be valid");
    }

    #[test]
    fn test_multi_return_function() {
        let mut m = Module::new();
        let pair = Function::new_multi(
            "pair",
            vec![],
            vec![Type::I64, Type::I64],
            vec![Instr::Const(1), Instr::Const(2), Instr::Ret],
        );
        m.add_function(pair);

        let wasm = emit_wasm_bytes(&m).expect("should emit multi-return wasm");
        wasmparser::Validator::new()
            .validate_all(&wasm)
            .expect("multi-return wasm should be valid");
    }
}
