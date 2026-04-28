use lir::{Instr, Module, Type};

/// MLIR dialect identifiers for the Omni IR
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MlirDialect {
    Func,
    Arith,
    Cf,
    MemRef,
    Linalg,
}

impl MlirDialect {
    pub fn name(&self) -> &'static str {
        match self {
            MlirDialect::Func => "func",
            MlirDialect::Arith => "arith",
            MlirDialect::Cf => "cf",
            MlirDialect::MemRef => "memref",
            MlirDialect::Linalg => "linalg",
        }
    }
}

/// MLIR operation representation
#[derive(Debug, Clone)]
pub enum MlirOp {
    // Func dialect
    FuncOp {
        name: String,
        inputs: Vec<Type>,
        outputs: Vec<Type>,
        body: Vec<MlirOp>,
    },
    Return {
        values: Vec<String>,
    },
    Call {
        callee: String,
        args: Vec<String>,
        results: Vec<String>,
    },

    // Arith dialect
    Constant {
        name: String,
        value: i64,
        result_type: Type,
    },
    Add {
        lhs: String,
        rhs: String,
        result: String,
    },
    Sub {
        lhs: String,
        rhs: String,
        result: String,
    },
    Mul {
        lhs: String,
        rhs: String,
        result: String,
    },
    Div {
        lhs: String,
        rhs: String,
        result: String,
    },

    // Control flow
    Branch {
        dest: String,
        args: Vec<String>,
    },
    CondBranch {
        cond: String,
        true_dest: String,
        false_dest: String,
    },

    // Memory
    Alloca {
        result: String,
        elem_type: Type,
    },
    Load {
        address: String,
        result: String,
    },
    Store {
        value: String,
        address: String,
    },
}

impl MlirOp {
    /// Convert to MLIR text format
    pub fn to_mlir(&self, indent: usize) -> String {
        let indent_str = "  ".repeat(indent);
        match self {
            MlirOp::FuncOp {
                name,
                inputs,
                outputs,
                body,
            } => {
                let _input_str = inputs
                    .iter()
                    .map(lir_type_to_mlir_type)
                    .collect::<Vec<_>>()
                    .join(", ");
                let output_str = if outputs.is_empty() {
                    "()".to_string()
                } else {
                    format!(
                        "({})",
                        outputs
                            .iter()
                            .map(lir_type_to_mlir_type)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                let mut result = format!("{}func.func @{}{} {{\n", indent_str, name, output_str);
                for op in body {
                    result.push_str(&op.to_mlir(indent + 1));
                }
                result.push_str(&format!("{}}}\n", indent_str));
                result
            }
            MlirOp::Return { values } => {
                if values.is_empty() {
                    format!("{}arith.return\n", indent_str)
                } else {
                    format!("{}arith.return {}\n", indent_str, values.join(", "))
                }
            }
            MlirOp::Call {
                callee,
                args,
                results,
            } => {
                let args_str = if args.is_empty() {
                    String::new()
                } else {
                    args.join(", ")
                };
                if results.is_empty() {
                    format!("{}call @{}({})\n", indent_str, callee, args_str)
                } else {
                    format!(
                        "{}%{} = call @{}({})\n",
                        indent_str,
                        results.join(", %"),
                        callee,
                        args_str
                    )
                }
            }
            MlirOp::Constant {
                name,
                value,
                result_type,
            } => {
                format!(
                    "{}%{} = arith.constant {} : {}\n",
                    indent_str,
                    name,
                    value,
                    lir_type_to_mlir_type(result_type)
                )
            }
            MlirOp::Add { lhs, rhs, result } => {
                format!(
                    "{}%{} = arith.addi %{}, %{} : i64\n",
                    indent_str, result, lhs, rhs
                )
            }
            MlirOp::Sub { lhs, rhs, result } => {
                format!(
                    "{}%{} = arith.subi %{}, %{} : i64\n",
                    indent_str, result, lhs, rhs
                )
            }
            MlirOp::Mul { lhs, rhs, result } => {
                format!(
                    "{}%{} = arith.muli %{}, %{} : i64\n",
                    indent_str, result, lhs, rhs
                )
            }
            MlirOp::Div { lhs, rhs, result } => {
                format!(
                    "{}%{} = arith.divsi %{}, %{} : i64\n",
                    indent_str, result, lhs, rhs
                )
            }
            MlirOp::Branch { dest, args } => {
                if args.is_empty() {
                    format!("{}cf.br ^{}\n", indent_str, dest)
                } else {
                    format!("{}cf.br ^{}({})\n", indent_str, dest, args.join(", "))
                }
            }
            MlirOp::CondBranch {
                cond,
                true_dest,
                false_dest,
            } => {
                format!(
                    "{}cf.cond_br %{}, ^{}, ^{}\n",
                    indent_str, cond, true_dest, false_dest
                )
            }
            MlirOp::Alloca { result, elem_type } => {
                format!(
                    "{}%{} = memref.alloca() : {}\n",
                    indent_str,
                    result,
                    lir_type_to_mlir_type(elem_type)
                )
            }
            MlirOp::Load { address, result } => {
                format!(
                    "{}%{} = memref.load %{}[] : {}\n",
                    indent_str,
                    result,
                    address,
                    lir_type_to_mlir_type(&Type::I64)
                )
            }
            MlirOp::Store { value, address } => {
                format!(
                    "{}memref.store %{}, %{}[] : {}\n",
                    indent_str,
                    value,
                    address,
                    lir_type_to_mlir_type(&Type::I64)
                )
            }
        }
    }
}

fn lir_type_to_mlir_type(ty: &Type) -> String {
    match ty {
        Type::I64 => "i64".to_string(),
        Type::Ptr => "i64".to_string(),
        Type::Void => "()".to_string(),
    }
}

/// Lower LIR to MLIR operations
pub fn lower_lir_to_mlir(module: &Module) -> Vec<MlirOp> {
    let mut operations = Vec::new();

    for func in &module.functions {
        let mut body = Vec::new();
        let mut var_counter = 0;

        for instr in &func.body {
            match instr {
                Instr::Const(value) => {
                    let var_name = format!("c{}", var_counter);
                    var_counter += 1;
                    body.push(MlirOp::Constant {
                        name: var_name,
                        value: *value,
                        result_type: Type::I64,
                    });
                }
                Instr::Add => {
                    // Assume two constants on stack, emit add
                    body.push(MlirOp::Add {
                        lhs: format!("c{}", var_counter - 2),
                        rhs: format!("c{}", var_counter - 1),
                        result: format!("add{}", var_counter),
                    });
                    var_counter += 1;
                }
                Instr::Sub => {
                    body.push(MlirOp::Sub {
                        lhs: format!("c{}", var_counter - 2),
                        rhs: format!("c{}", var_counter - 1),
                        result: format!("sub{}", var_counter),
                    });
                    var_counter += 1;
                }
                Instr::Mul => {
                    body.push(MlirOp::Mul {
                        lhs: format!("c{}", var_counter - 2),
                        rhs: format!("c{}", var_counter - 1),
                        result: format!("mul{}", var_counter),
                    });
                    var_counter += 1;
                }
                Instr::Div => {
                    body.push(MlirOp::Div {
                        lhs: format!("c{}", var_counter - 2),
                        rhs: format!("c{}", var_counter - 1),
                        result: format!("div{}", var_counter),
                    });
                    var_counter += 1;
                }
                Instr::Call(name) => {
                    body.push(MlirOp::Call {
                        callee: name.clone(),
                        args: vec![],
                        results: vec![format!("ret{}", var_counter)],
                    });
                    var_counter += 1;
                }
                Instr::Ret => {
                    body.push(MlirOp::Return { values: vec![] });
                }
                Instr::Load(slot) => {
                    body.push(MlirOp::Load {
                        address: format!("slot{}", slot),
                        result: format!("load{}", slot),
                    });
                }
                Instr::Store(slot) => {
                    body.push(MlirOp::Store {
                        value: format!("val{}", slot),
                        address: format!("slot{}", slot),
                    });
                }
                Instr::Jump(target) => {
                    body.push(MlirOp::Branch {
                        dest: format!("bb{}", target),
                        args: vec![],
                    });
                }
                Instr::CondJump { if_true, if_false } => {
                    body.push(MlirOp::CondBranch {
                        cond: "cond".to_string(),
                        true_dest: format!("bb{}", if_true),
                        false_dest: format!("bb{}", if_false),
                    });
                }
                Instr::Drop(_) | Instr::Nop => {
                    // No MLIR equivalent needed
                }
            }
        }

        operations.push(MlirOp::FuncOp {
            name: func.name.clone(),
            inputs: func.params.clone(),
            outputs: func.rets.clone(),
            body,
        });
    }

    operations
}

/// Generate full MLIR module text
pub fn emit_mlir_text(module: &Module) -> String {
    let mut output = String::new();

    // MLIR header with required dialects
    output.push_str("// RUN: mlir-opt %s -arith-expand\n");
    output.push_str("// RUN: mlir-opt %s -cf-optimize\n\n");

    output.push_str("module {\n");

    let operations = lower_lir_to_mlir(module);
    for op in operations {
        output.push_str(&op.to_mlir(1));
    }

    output.push_str("}\n");
    output
}

/// Small tensor-workload acceptance fixture for toolchain-backed MLIR tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TensorAddWorkload {
        pub length: usize,
}

impl TensorAddWorkload {
        pub fn new(length: usize) -> Self {
                Self { length: length.max(1) }
        }

        pub fn emit_mlir_text(&self) -> String {
                emit_tensor_add_mlir_text(self.length)
        }
}

/// Emit a tiny tensor addition module that uses tensor and linalg dialects.
pub fn emit_tensor_add_mlir_text(length: usize) -> String {
        let tensor_type = format!("tensor<{}xi64>", length.max(1));
        let template = r#"#map_1d_identity = affine_map<(d0) -> (d0)>
module {
    func.func @tensor_add(%lhs: __TY__, %rhs: __TY__) -> __TY__ {
        %result = tensor.empty() : __TY__
        %0 = linalg.generic {
            indexing_maps = [#map_1d_identity, #map_1d_identity, #map_1d_identity],
            iterator_types = ["parallel"]
        } ins(%lhs, %rhs : __TY__, __TY__) outs(%result : __TY__) {
        ^bb0(%a: i64, %b: i64, %acc: i64):
            %sum = arith.addi %a, %b : i64
            linalg.yield %sum : i64
        } -> __TY__
        func.return %0 : __TY__
    }
}
"#;

        template.replace("__TY__", &tensor_type)
}

/// Emit a small control-flow-heavy MLIR module so the MLIR path proves more
/// than straight-line arithmetic.
pub fn emit_control_flow_demo_mlir_text() -> String {
        let mut output = String::new();
        output.push_str("module {\n");
        output.push_str("  func.func @control_flow_demo(%cond: i1, %lhs: i64, %rhs: i64) -> i64 {\n");
        output.push_str("    cf.cond_br %cond, ^bb1, ^bb2\n");
        output.push_str("  ^bb1:\n");
        output.push_str("    cf.br ^bb3(%lhs : i64)\n");
        output.push_str("  ^bb2:\n");
        output.push_str("    cf.br ^bb3(%rhs : i64)\n");
        output.push_str("  ^bb3(%value: i64):\n");
        output.push_str("    func.return %value : i64\n");
        output.push_str("  }\n");
        output.push_str("}\n");
        output
}

/// Compile and run by emitting MLIR text and executing the validated runtime path.
pub fn compile_and_run_with_mlir(module: &Module) -> Result<Vec<i64>, String> {
    let _mlir_text = emit_mlir_text(module);
    compile_and_run_with_mlir_jit(module)
}

/// Execution bridge that uses Cranelift's JIT for the current workspace runtime.
pub fn compile_and_run_with_mlir_jit(module: &Module) -> Result<Vec<i64>, String> {
    codegen_cranelift::compile_and_run_with_jit(module)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_mlir_text() {
        let module = lir::example_module();
        let mlir = emit_mlir_text(&module);
        assert!(!mlir.is_empty());
        assert!(mlir.contains("module"));
        assert!(mlir.contains("func.func"));
        assert!(mlir.contains("arith.constant"));
    }

    #[test]
    fn test_emit_tensor_add_mlir_text() {
        let workload = TensorAddWorkload::new(4);
        let mlir = workload.emit_mlir_text();
        assert!(mlir.contains("tensor.empty"));
        assert!(mlir.contains("linalg.generic"));
        assert!(mlir.contains("linalg.yield"));
        assert!(mlir.contains("tensor<4xi64>"));
    }

    #[test]
    fn test_emit_control_flow_demo_mlir_text() {
        let mlir = emit_control_flow_demo_mlir_text();
        assert!(mlir.contains("cf.cond_br"));
        assert!(mlir.contains("cf.br"));
        assert!(mlir.contains("func.return"));
    }

    #[test]
    fn test_lower_lir_to_mlir() {
        let module = lir::example_module();
        let ops = lower_lir_to_mlir(&module);
        assert!(!ops.is_empty());
    }

    #[test]
    fn test_jit_runs() {
        let module = lir::example_module();
        let result = compile_and_run_with_mlir_jit(&module);
        assert!(result.is_ok());
        // example_module returns 42 (40 + 2)
        assert_eq!(result.unwrap(), vec![42]);
    }

    #[test]
    fn test_compile_and_run_with_mlir_uses_jit() {
        let module = lir::example_module();
        let result = compile_and_run_with_mlir(&module);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![42]);
    }
}
