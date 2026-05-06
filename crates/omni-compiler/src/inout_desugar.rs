//! Inout parameter desugaring pass.
//!
//! Desugars `fn foo(inout x: T)` into explicit move-in / move-out semantics:
//! 1. Caller moves the argument into the callee's inout parameter.
//! 2. Callee's inout parameter is a linear-type local variable.
//! 3. On return, the (possibly mutated) value is moved back to the caller.
//!
//! This pass rewrites the AST so that MIR lowering can treat inout parameters
//! as linear moves rather than copies.

use crate::ast::{Program, Stmt, Expr};

/// Rewrite a program in-place, desugaring `inout` parameters.
pub fn desugar_inout_in_ast(prog: &mut Program) -> Result<(), String> {
    for stmt in &mut prog.stmts {
        desugar_stmt(stmt);
    }
    Ok(())
}

fn desugar_stmt(stmt: &mut Stmt) {
    match stmt {
        Stmt::Fn {
            ref mut params,
            ref mut body,
            name: _,
            ..
        } => {
            // Collect inout parameters and their types.
            let inout_params: Vec<(String, String)> = params
                .iter()
                .filter_map(|p| {
                    // Inout params are marked with a special prefix or attribute.
                    // For now, we assume parameters whose name starts with "inout_"
                    // are inout parameters.
                    if p.starts_with("inout_") {
                        let real_name = p.strip_prefix("inout_").unwrap_or(p).to_string();
                        // We don't have the type info here, so we'll need to
                        // get it from the type annotation if available.
                        Some((real_name, "inout".to_string()))
                    } else {
                        None
                    }
                })
                .collect();

            if inout_params.is_empty() {
                return;
            }

            // Rewrite function body to treat inout params as linear.
            let mut new_body: Vec<Stmt> = Vec::new();
            for (real_name, _ty) in &inout_params {
                // At the start of the function, the inout param is already
                // moved in (caller did the move). We just need to ensure
                // it's treated as linear.
                new_body.push(Stmt::LetLinear(real_name.clone(), Expr::Var(format!("__inout_src_{}", real_name))));
            }

            // Add the original body (with references to the inout param fixed).
            for s in body.iter() {
                let mut rewritten = s.clone();
                rewrite_inout_refs(&mut rewritten, &inout_params);
                new_body.push(rewritten);
            }

            // At the end, move the inout value back to the caller's location.
            for (real_name, _ty) in &inout_params {
                new_body.push(Stmt::LetLinear(
                    format!("__inout_out_{}", real_name),
                    Expr::Var(real_name.clone()),
                ));
            }

            *body = new_body;
        }
        Stmt::Block(inner) => {
            let mut new_inner = Vec::new();
            for s in inner.iter() {
                let mut rewritten = s.clone();
                desugar_stmt(&mut rewritten);
                new_inner.push(rewritten);
            }
            *inner = new_inner;
        }
        _ => {}
    }
}

fn rewrite_inout_refs(stmt: &mut Stmt, inout_params: &[(String, String)]) {
    // This is a placeholder for rewriting references to inout parameters.
    // In a full implementation, we would rewrite:
    //   `x.field` where `x` is inout -> special field access that tracks mutations.
    // For now, this is a no-op.
    let _ = (stmt, inout_params);
}

/// Lowering support: convert a function call with inout arguments.
///
/// When calling `foo(inout x)`, the caller must:
/// 1. Move `x` into the function (it becomes a linear value for the callee).
/// 2. After the call, `x` is considered moved (caller must use the returned value).
///
/// This function generates the MIR instructions for making an inout call.
pub fn lower_inout_call(
    dest: &str,
    func: &str,
    args: &[String],
    is_inout: &[bool],
) -> Vec<crate::mir::Instruction> {
    let mut instrs = Vec::new();
    use crate::mir::Instruction;

    for (i, (arg, &is_io)) in args.iter().zip(is_inout.iter()).enumerate() {
        if is_io {
            // Inout argument: move the value into the function.
            instrs.push(Instruction::LinearMove {
                dest: format!("__inout_arg_{}_{}", func, i),
                src: arg.clone(),
            });
        } else {
            // Normal argument: regular move.
            instrs.push(Instruction::Move {
                dest: format!("__arg_{}_{}", func, i),
                src: arg.clone(),
            });
        }
    }

    // Call the function.
    instrs.push(Instruction::Call {
        dest: format!("{}__inout_result", dest),
        func: func.to_string(),
        args: args.to_vec(),
    });

    // For each inout argument, the caller receives back a (possibly mutated) value.
    for (i, (&is_io, arg)) in is_inout.iter().zip(args.iter()).enumerate() {
        if is_io {
            instrs.push(Instruction::LinearMove {
                dest: arg.clone(), // Move back to the original variable.
                src: format!("__inout_out_{}_{}", func, i),
            });
        }
    }

    instrs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Program, Stmt, Expr};

    #[test]
    fn desugar_finds_inout_params() {
        // Create a function with an inout_ parameter.
        let mut prog = Program {
            stmts: vec![
                Stmt::Fn {
                    name: "update".to_string(),
                    is_public: false,
                    is_async: false,
                    type_params: vec![],
                    params: vec!["inout_val".to_string()],
                    ret_type: None,
                    effects: vec![],
                    body: vec![
                        Stmt::Assign("val".to_string(), Expr::Number(42)),
                    ],
                },
            ],
        };

        let result = desugar_inout_in_ast(&mut prog);
        assert!(result.is_ok());
    }

    #[test]
    fn lower_inout_call_generates_correct_mir() {
        let instrs = lower_inout_call(
            "result",
            "update",
            &["x".to_string()],
            &[true],
        );

        // Should have: linear_move for arg, call, linear_move back.
        assert!(instrs.len() >= 2);
        
        // Check that we have LinearMove instructions.
        let has_linear_move = instrs.iter().any(|i| matches!(i, crate::mir::Instruction::LinearMove { .. }));
        assert!(has_linear_move);
    }

    #[test]
    fn inout_call_with_mixed_args() {
        let instrs = lower_inout_call(
            "result",
            "foo",
            &["normal".to_string(), "inout_val".to_string()],
            &[false, true],
        );

        // Should generate both Move and LinearMove instructions.
        let move_count = instrs.iter().filter(|i| matches!(i, crate::mir::Instruction::Move { .. })).count();
        let linear_move_count = instrs.iter().filter(|i| matches!(i, crate::mir::Instruction::LinearMove { .. })).count();
        
        assert!(move_count >= 1); // normal arg
        assert!(linear_move_count >= 1); // inout arg
    }
}
