use crate::ast::{Expr, Program, Stmt};
use crate::complete_lexer::TokenKind;

#[derive(Debug, Default)]
pub struct MirModule {
    pub functions: Vec<MirFunction>,
}

#[derive(Debug)]
pub struct MirFunction {
    pub name: String,
    pub blocks: Vec<BasicBlock>,
}

#[derive(Debug)]
pub struct BasicBlock {
    pub id: usize,
    pub instrs: Vec<Instruction>,
}

#[derive(Debug)]
pub enum Instruction {
    ConstInt {
        dest: String,
        value: i64,
    },
    ConstStr {
        dest: String,
        value: String,
    },
    ConstBool {
        dest: String,
        value: bool,
    },
    Move {
        dest: String,
        src: String,
    },
    LinearMove {
        dest: String,
        src: String,
    },
    Print {
        src: String,
    },
    Drop {
        var: String,
    },
    DropLinear {
        var: String,
    },
    Jump {
        target: usize,
    },
    JumpIf {
        cond: String,
        target: usize,
    },
    Label {
        id: usize,
    },
    BinaryOp {
        dest: String,
        op: TokenKind,
        left: String,
        right: String,
    },
    UnaryOp {
        dest: String,
        op: TokenKind,
        operand: String,
    },
    Return {
        value: String,
    },
    Assign {
        dest: String,
        src: String,
    },
    Call {
        dest: String,
        func: String,
        args: Vec<String>,
    },
    FieldAccess {
        dest: String,
        base: String,
        field: String,
    },
    StructAccess {
        dest: String,
        base: String,
        field: String,
    },
    IndexAccess {
        dest: String,
        base: String,
        index: String,
    },
    StructDef {
        name: String,
        fields: Vec<(String, String)>,
        is_linear: bool,
    },
    EnumDef {
        name: String,
        variants: Vec<crate::ast::EnumVariant>,
    },
    MatchBranch {
        cond: String,
        then_block: usize,
        else_block: usize,
    },
}

impl MirModule {
    pub fn new() -> Self {
        MirModule {
            functions: Vec::new(),
        }
    }
}

impl MirFunction {
    pub fn new(name: &str) -> Self {
        MirFunction {
            name: name.to_string(),
            blocks: Vec::new(),
        }
    }
}

impl BasicBlock {
    pub fn new(id: usize) -> Self {
        BasicBlock {
            id,
            instrs: Vec::new(),
        }
    }
}

pub fn lower_program_to_mir(prog: &Program) -> MirModule {
    let mut module = MirModule::new();
    let mut func = MirFunction::new("main");
    let mut block = BasicBlock::new(0);
    let mut temp_id: usize = 0;
    let mut scopes: Vec<Vec<String>> = vec![Vec::new()];

    fn lower_stmt(
        stmt: &Stmt,
        block: &mut BasicBlock,
        temp_id: &mut usize,
        scopes: &mut Vec<Vec<String>>,
    ) {
        match stmt {
            Stmt::Let(name, expr) => {
                match expr {
                    Expr::Number(n) => {
                        block.instrs.push(Instruction::ConstInt {
                            dest: name.clone(),
                            value: *n,
                        });
                    }
                    Expr::StringLit(s) => {
                        block.instrs.push(Instruction::ConstStr {
                            dest: name.clone(),
                            value: s.clone(),
                        });
                    }
                    Expr::Bool(b) => {
                        block.instrs.push(Instruction::ConstBool {
                            dest: name.clone(),
                            value: *b,
                        });
                    }
                    Expr::Var(other) => {
                        block.instrs.push(Instruction::Move {
                            dest: name.clone(),
                            src: other.clone(),
                        });
                    }
                    Expr::BinaryOp { op, left, right } => {
                        let l = match left.as_ref() {
                            Expr::Var(v) => v.clone(),
                            Expr::Number(n) => {
                                let t = format!("__t{}", temp_id);
                                *temp_id += 1;
                                block.instrs.push(Instruction::ConstInt {
                                    dest: t.clone(),
                                    value: *n,
                                });
                                t
                            }
                            _ => {
                                let t = format!("__t{}", temp_id);
                                *temp_id += 1;
                                t
                            }
                        };
                        let r = match right.as_ref() {
                            Expr::Var(v) => v.clone(),
                            Expr::Number(n) => {
                                let t = format!("__t{}", temp_id);
                                *temp_id += 1;
                                block.instrs.push(Instruction::ConstInt {
                                    dest: t.clone(),
                                    value: *n,
                                });
                                t
                            }
                            _ => {
                                let t = format!("__t{}", temp_id);
                                *temp_id += 1;
                                t
                            }
                        };
                        block.instrs.push(Instruction::BinaryOp {
                            dest: name.clone(),
                            op: op.clone(),
                            left: l,
                            right: r,
                        });
                    }
                    Expr::UnaryOp { op, inner } => {
                        let src = match inner.as_ref() {
                            Expr::Var(v) => v.clone(),
                            _ => {
                                let t = format!("__t{}", temp_id);
                                *temp_id += 1;
                                t
                            }
                        };
                        block.instrs.push(Instruction::UnaryOp {
                            dest: name.clone(),
                            op: op.clone(),
                            operand: src,
                        });
                    }
                    Expr::Call(fname, args) => {
                        let mut arg_values: Vec<String> = Vec::new();
                        for arg in args {
                            let arg_temp = format!("__arg{}", *temp_id);
                            *temp_id += 1;
                            match arg {
                                Expr::Number(n) => {
                                    block.instrs.push(Instruction::ConstInt {
                                        dest: arg_temp.clone(),
                                        value: *n,
                                    });
                                }
                                Expr::StringLit(s) => {
                                    block.instrs.push(Instruction::ConstStr {
                                        dest: arg_temp.clone(),
                                        value: s.clone(),
                                    });
                                }
                                Expr::Bool(b) => {
                                    block.instrs.push(Instruction::ConstBool {
                                        dest: arg_temp.clone(),
                                        value: *b,
                                    });
                                }
                                Expr::Var(v) => {
                                    block.instrs.push(Instruction::Move {
                                        dest: arg_temp.clone(),
                                        src: v.clone(),
                                    });
                                }
                                _ => {
                                    block.instrs.push(Instruction::ConstInt {
                                        dest: arg_temp.clone(),
                                        value: 0,
                                    });
                                }
                            }
                            arg_values.push(arg_temp);
                        }
                        block.instrs.push(Instruction::Call {
                            dest: name.clone(),
                            func: fname.clone(),
                            args: arg_values,
                        });
                        let t = format!("__call_res{}", *temp_id);
                        *temp_id += 1;
                        block.instrs.push(Instruction::ConstInt {
                            dest: t.clone(),
                            value: 0,
                        });
                        block.instrs.push(Instruction::Move {
                            dest: name.clone(),
                            src: t,
                        });
                    }
                    Expr::FieldAccess { base, field } => {
                        let base_var = match base.as_ref() {
                            Expr::Var(v) => v.clone(),
                            _ => {
                                let t = format!("__t{}", *temp_id);
                                *temp_id += 1;
                                t
                            }
                        };
                        block.instrs.push(Instruction::FieldAccess {
                            dest: name.clone(),
                            base: base_var,
                            field: field.clone(),
                        });
                    }
                    Expr::Match { expr, arms } => {
                        let cond_var = match expr.as_ref() {
                            Expr::Var(v) => v.clone(),
                            _ => {
                                let t = format!("__t{}", *temp_id);
                                *temp_id += 1;
                                t
                            }
                        };
                        let match_dest = name.clone();
                        for (i, arm) in arms.iter().enumerate() {
                            let next_block_id = *temp_id;
                            *temp_id += 1;
                            let then_block_id = *temp_id;
                            *temp_id += 1;
                            block.instrs.push(Instruction::JumpIf {
                                cond: cond_var.clone(),
                                target: then_block_id,
                            });
                            block.instrs.push(Instruction::Jump {
                                target: next_block_id,
                            });
                            block.instrs.push(Instruction::Label { id: then_block_id });
                            if let Expr::Var(v) = arm.body.as_ref() {
                                block.instrs.push(Instruction::Move {
                                    dest: match_dest.clone(),
                                    src: v.clone(),
                                });
                            } else if let Expr::Number(n) = arm.body.as_ref() {
                                let t = format!("__t{}", *temp_id);
                                *temp_id += 1;
                                block.instrs.push(Instruction::ConstInt {
                                    dest: t.clone(),
                                    value: *n,
                                });
                                block.instrs.push(Instruction::Move {
                                    dest: match_dest.clone(),
                                    src: t,
                                });
                            }
                            if i < arms.len() - 1 {
                                block.instrs.push(Instruction::Jump {
                                    target: next_block_id,
                                });
                            }
                            block.instrs.push(Instruction::Label { id: next_block_id });
                        }
                    }
                    _ => {
                        let t = format!("__t{}", *temp_id);
                        *temp_id += 1;
                        block.instrs.push(Instruction::ConstInt {
                            dest: t.clone(),
                            value: 0,
                        });
                        block.instrs.push(Instruction::Move {
                            dest: name.clone(),
                            src: t,
                        });
                    }
                }
                if let Some(cur) = scopes.last_mut() {
                    cur.push(name.clone());
                }
            }
            Stmt::Print(expr) => match expr {
                Expr::Number(n) => {
                    let t = format!("__t{}", *temp_id);
                    *temp_id += 1;
                    block.instrs.push(Instruction::ConstInt {
                        dest: t.clone(),
                        value: *n,
                    });
                    block.instrs.push(Instruction::Print { src: t });
                }
                Expr::StringLit(s) => {
                    let t = format!("__t{}", *temp_id);
                    *temp_id += 1;
                    block.instrs.push(Instruction::ConstStr {
                        dest: t.clone(),
                        value: s.clone(),
                    });
                    block.instrs.push(Instruction::Print { src: t });
                }
                Expr::Var(name) => {
                    block.instrs.push(Instruction::Print { src: name.clone() });
                }
                _ => {}
            },
            Stmt::ExprStmt(expr) => match expr {
                Expr::Number(n) => {
                    let t = format!("__t{}", *temp_id);
                    *temp_id += 1;
                    block
                        .instrs
                        .push(Instruction::ConstInt { dest: t, value: *n });
                }
                Expr::StringLit(s) => {
                    let t = format!("__t{}", *temp_id);
                    *temp_id += 1;
                    block.instrs.push(Instruction::ConstStr {
                        dest: t,
                        value: s.clone(),
                    });
                }
                Expr::Var(_) => {}
                Expr::BinaryOp { op, left, right } => {
                    let l = match left.as_ref() {
                        Expr::Var(v) => v.clone(),
                        _ => {
                            let t = format!("__t{}", *temp_id);
                            *temp_id += 1;
                            t
                        }
                    };
                    let r = match right.as_ref() {
                        Expr::Var(v) => v.clone(),
                        _ => {
                            let t = format!("__t{}", *temp_id);
                            *temp_id += 1;
                            t
                        }
                    };
                    let dest = format!("__t{}", *temp_id);
                    *temp_id += 1;
                    block.instrs.push(Instruction::BinaryOp {
                        dest,
                        op: op.clone(),
                        left: l,
                        right: r,
                    });
                }
                Expr::Call(fname, args) => {
                    let mut arg_values: Vec<String> = Vec::new();
                    for arg in args {
                        let arg_temp = format!("__arg{}", *temp_id);
                        *temp_id += 1;
                        match arg {
                            Expr::Number(n) => {
                                block.instrs.push(Instruction::ConstInt {
                                    dest: arg_temp.clone(),
                                    value: *n,
                                });
                            }
                            Expr::StringLit(s) => {
                                block.instrs.push(Instruction::ConstStr {
                                    dest: arg_temp.clone(),
                                    value: s.clone(),
                                });
                            }
                            Expr::Bool(b) => {
                                block.instrs.push(Instruction::ConstBool {
                                    dest: arg_temp.clone(),
                                    value: *b,
                                });
                            }
                            Expr::Var(v) => {
                                block.instrs.push(Instruction::Move {
                                    dest: arg_temp.clone(),
                                    src: v.clone(),
                                });
                            }
                            _ => {
                                block.instrs.push(Instruction::ConstInt {
                                    dest: arg_temp.clone(),
                                    value: 0,
                                });
                            }
                        }
                        arg_values.push(arg_temp);
                    }
                    let dest = format!("__call_res{}", *temp_id);
                    *temp_id += 1;
                    block.instrs.push(Instruction::Call {
                        dest: dest.clone(),
                        func: fname.clone(),
                        args: arg_values,
                    });
                }
                Expr::FieldAccess { base, field } => {
                    let base_var = match base.as_ref() {
                        Expr::Var(v) => v.clone(),
                        _ => {
                            let t = format!("__t{}", *temp_id);
                            *temp_id += 1;
                            t
                        }
                    };
                    let dest = format!("__t{}", *temp_id);
                    *temp_id += 1;
                    block.instrs.push(Instruction::FieldAccess {
                        dest,
                        base: base_var,
                        field: field.clone(),
                    });
                }
                Expr::Match { expr, arms } => {
                    let cond_var = match expr.as_ref() {
                        Expr::Var(v) => v.clone(),
                        _ => {
                            let t = format!("__t{}", *temp_id);
                            *temp_id += 1;
                            t
                        }
                    };
                    for (i, _arm) in arms.iter().enumerate() {
                        let next_block_id = *temp_id;
                        *temp_id += 1;
                        let then_block_id = *temp_id;
                        *temp_id += 1;
                        block.instrs.push(Instruction::JumpIf {
                            cond: cond_var.clone(),
                            target: then_block_id,
                        });
                        block.instrs.push(Instruction::Jump {
                            target: next_block_id,
                        });
                        block.instrs.push(Instruction::Label { id: then_block_id });
                        if i < arms.len() - 1 {
                            block.instrs.push(Instruction::Jump {
                                target: next_block_id,
                            });
                        }
                        block.instrs.push(Instruction::Label { id: next_block_id });
                    }
                }
                _ => {}
            },
            Stmt::Block(inner) => {
                scopes.push(Vec::new());
                for s in inner {
                    lower_stmt(s, block, temp_id, scopes);
                }
                if let Some(decls) = scopes.pop() {
                    for name in decls.iter().rev() {
                        block.instrs.push(Instruction::Drop { var: name.clone() });
                    }
                }
            }
            Stmt::Fn { .. } => {}
            Stmt::If {
                cond,
                then_body,
                else_body,
                ..
            } => {
                let then_block_id = *temp_id;
                *temp_id += 1;
                let else_block_id = *temp_id;
                *temp_id += 1;
                let end_block_id = *temp_id;
                *temp_id += 1;

                let cond_var = match cond.as_ref() {
                    Expr::Var(v) => v.clone(),
                    _ => {
                        let t = format!("__t{}", *temp_id);
                        *temp_id += 1;
                        t
                    }
                };
                block.instrs.push(Instruction::JumpIf {
                    cond: cond_var,
                    target: then_block_id,
                });

                scopes.push(Vec::new());
                for s in else_body {
                    lower_stmt(s, block, temp_id, scopes);
                }
                if let Some(decls) = scopes.pop() {
                    for name in decls.iter().rev() {
                        block.instrs.push(Instruction::Drop { var: name.clone() });
                    }
                }

                block.instrs.push(Instruction::Jump {
                    target: end_block_id,
                });
                block.instrs.push(Instruction::Label { id: then_block_id });

                scopes.push(Vec::new());
                for s in then_body {
                    lower_stmt(s, block, temp_id, scopes);
                }
                if let Some(decls) = scopes.pop() {
                    for name in decls.iter().rev() {
                        block.instrs.push(Instruction::Drop { var: name.clone() });
                    }
                }

                block.instrs.push(Instruction::Jump {
                    target: end_block_id,
                });
                block.instrs.push(Instruction::Label { id: else_block_id });
                block.instrs.push(Instruction::Jump {
                    target: end_block_id,
                });
                block.instrs.push(Instruction::Label { id: end_block_id });
            }
            Stmt::Loop { body } => {
                let loop_start = *temp_id;
                *temp_id += 1;
                let loop_end = *temp_id;
                *temp_id += 1;

                block.instrs.push(Instruction::Jump { target: loop_start });
                block.instrs.push(Instruction::Label { id: loop_start });

                scopes.push(Vec::new());
                for s in body {
                    lower_stmt(s, block, temp_id, scopes);
                }
                if let Some(decls) = scopes.pop() {
                    for name in decls.iter().rev() {
                        block.instrs.push(Instruction::Drop { var: name.clone() });
                    }
                }

                block.instrs.push(Instruction::Jump { target: loop_start });
                block.instrs.push(Instruction::Label { id: loop_end });
            }
            Stmt::For {
                var_name,
                iterable,
                body,
            } => {
                let for_start = *temp_id;
                *temp_id += 1;
                let for_end = *temp_id;
                *temp_id += 1;

                let iter_var = match iterable.as_ref() {
                    Expr::Var(v) => v.clone(),
                    Expr::Number(n) => {
                        let t = format!("__t{}", *temp_id);
                        *temp_id += 1;
                        block.instrs.push(Instruction::ConstInt {
                            dest: t.clone(),
                            value: *n,
                        });
                        t
                    }
                    _ => {
                        let t = format!("__t{}", *temp_id);
                        *temp_id += 1;
                        t
                    }
                };

                let counter_var = format!("__counter{}", temp_id);
                *temp_id += 1;
                block.instrs.push(Instruction::ConstInt {
                    dest: counter_var.clone(),
                    value: 0,
                });
                block.instrs.push(Instruction::Jump { target: for_start });
                block.instrs.push(Instruction::Label { id: for_start });

                let cond_var = format!("__cond{}", temp_id);
                *temp_id += 1;
                block.instrs.push(Instruction::BinaryOp {
                    dest: cond_var.clone(),
                    op: TokenKind::Lt,
                    left: counter_var.clone(),
                    right: iter_var,
                });

                block.instrs.push(Instruction::JumpIf {
                    cond: cond_var,
                    target: for_start + 1,
                });
                block.instrs.push(Instruction::Jump { target: for_end });
                block.instrs.push(Instruction::Label { id: for_start + 1 });

                scopes.push(Vec::new());
                scopes.last_mut().unwrap().push(var_name.clone());
                for s in body {
                    lower_stmt(s, block, temp_id, scopes);
                }
                if let Some(decls) = scopes.pop() {
                    for name in decls.iter().rev() {
                        block.instrs.push(Instruction::Drop { var: name.clone() });
                    }
                }

                let inc_var = format!("__inc{}", temp_id);
                *temp_id += 1;
                block.instrs.push(Instruction::BinaryOp {
                    dest: inc_var.clone(),
                    op: TokenKind::Plus,
                    left: counter_var.clone(),
                    right: "__1".to_string(),
                });

                block.instrs.push(Instruction::Jump { target: for_start });
                block.instrs.push(Instruction::Label { id: for_end });
            }
            Stmt::While { cond, body } => {
                let while_start = *temp_id;
                *temp_id += 1;
                let while_end = *temp_id;
                *temp_id += 1;

                block.instrs.push(Instruction::Jump {
                    target: while_start,
                });
                block.instrs.push(Instruction::Label { id: while_start });

                let cond_var = match cond.as_ref() {
                    Expr::Var(v) => v.clone(),
                    _ => {
                        let t = format!("__t{}", *temp_id);
                        *temp_id += 1;
                        t
                    }
                };
                block.instrs.push(Instruction::JumpIf {
                    cond: cond_var,
                    target: while_start + 1,
                });
                block.instrs.push(Instruction::Jump { target: while_end });
                block.instrs.push(Instruction::Label {
                    id: while_start + 1,
                });

                scopes.push(Vec::new());
                for s in body {
                    lower_stmt(s, block, temp_id, scopes);
                }
                if let Some(decls) = scopes.pop() {
                    for name in decls.iter().rev() {
                        block.instrs.push(Instruction::Drop { var: name.clone() });
                    }
                }

                block.instrs.push(Instruction::Jump {
                    target: while_start,
                });
                block.instrs.push(Instruction::Label { id: while_end });
            }
            Stmt::Return(expr) => {
                let val = match expr {
                    Expr::Var(v) => v.clone(),
                    Expr::Number(n) => {
                        let t = format!("__t{}", *temp_id);
                        *temp_id += 1;
                        block.instrs.push(Instruction::ConstInt {
                            dest: t.clone(),
                            value: *n,
                        });
                        t
                    }
                    Expr::BinaryOp { op, left, right } => {
                        let l = match left.as_ref() {
                            Expr::Var(v) => v.clone(),
                            _ => {
                                let t = format!("__t{}", *temp_id);
                                *temp_id += 1;
                                t
                            }
                        };
                        let r = match right.as_ref() {
                            Expr::Var(v) => v.clone(),
                            _ => {
                                let t = format!("__t{}", *temp_id);
                                *temp_id += 1;
                                t
                            }
                        };
                        let dest = format!("__t{}", *temp_id);
                        *temp_id += 1;
                        block.instrs.push(Instruction::BinaryOp {
                            dest: dest.clone(),
                            op: op.clone(),
                            left: l,
                            right: r,
                        });
                        dest
                    }
                    _ => {
                        let t = format!("__t{}", *temp_id);
                        *temp_id += 1;
                        t
                    }
                };
                block.instrs.push(Instruction::Return { value: val });
            }
            Stmt::Break => {}
            Stmt::Continue => {}
            Stmt::Assign(name, expr) => match expr {
                Expr::Var(other) => {
                    block.instrs.push(Instruction::Assign {
                        dest: name.clone(),
                        src: other.clone(),
                    });
                }
                Expr::Number(n) => {
                    block.instrs.push(Instruction::Assign {
                        dest: name.clone(),
                        src: format!("{}", n),
                    });
                }
                Expr::BinaryOp { op, left, right } => {
                    let l = match left.as_ref() {
                        Expr::Var(v) => v.clone(),
                        _ => "__0".to_string(),
                    };
                    let r = match right.as_ref() {
                        Expr::Var(v) => v.clone(),
                        _ => "__0".to_string(),
                    };
                    block.instrs.push(Instruction::BinaryOp {
                        dest: name.clone(),
                        op: op.clone(),
                        left: l,
                        right: r,
                    });
                }
                _ => {
                    block.instrs.push(Instruction::Assign {
                        dest: name.clone(),
                        src: "__0".to_string(),
                    });
                }
            },
            Stmt::ExprFieldAssign(base, field, expr) => {
                let base_var = match base.as_ref() {
                    Expr::Var(v) => v.clone(),
                    _ => "__base".to_string(),
                };
                let _val_var = match expr {
                    Expr::Var(v) => v.clone(),
                    Expr::Number(n) => n.to_string(),
                    _ => "__0".to_string(),
                };
                block.instrs.push(Instruction::StructAccess {
                    dest: format!("{}.{}", base_var, field),
                    base: base_var,
                    field: field.clone(),
                });
            }
            Stmt::WhileIn {
                var_name,
                iterable,
                body,
            } => {
                let iter_var = match iterable.as_ref() {
                    Expr::Var(v) => v.clone(),
                    Expr::Number(n) => {
                        let t = format!("__t{}", *temp_id);
                        *temp_id += 1;
                        block.instrs.push(Instruction::ConstInt {
                            dest: t.clone(),
                            value: *n,
                        });
                        t
                    }
                    _ => "__0".to_string(),
                };

                let in_start = *temp_id;
                *temp_id += 1;
                let in_end = *temp_id;
                *temp_id += 1;

                block.instrs.push(Instruction::Jump { target: in_start });
                block.instrs.push(Instruction::Label { id: in_start });

                let cond_var = format!("__cond{}", temp_id);
                *temp_id += 1;
                block.instrs.push(Instruction::BinaryOp {
                    dest: cond_var.clone(),
                    op: TokenKind::Lt,
                    left: var_name.clone(),
                    right: iter_var,
                });

                block.instrs.push(Instruction::JumpIf {
                    cond: cond_var,
                    target: in_start + 1,
                });
                block.instrs.push(Instruction::Jump { target: in_end });
                block.instrs.push(Instruction::Label { id: in_start + 1 });

                scopes.push(Vec::new());
                scopes.last_mut().unwrap().push(var_name.clone());
                for s in body {
                    lower_stmt(s, block, temp_id, scopes);
                }
                if let Some(decls) = scopes.pop() {
                    for name in decls.iter().rev() {
                        block.instrs.push(Instruction::Drop { var: name.clone() });
                    }
                }

                block.instrs.push(Instruction::Jump { target: in_start });
                block.instrs.push(Instruction::Label { id: in_end });
            }
            Stmt::Unsafe { body } => {
                for s in body {
                    lower_stmt(s, block, temp_id, scopes);
                }
            }
            Stmt::LetLinear(name, expr) => {
                match expr {
                    Expr::Number(n) => {
                        block.instrs.push(Instruction::ConstInt {
                            dest: name.clone(),
                            value: *n,
                        });
                    }
                    Expr::StringLit(s) => {
                        block.instrs.push(Instruction::ConstStr {
                            dest: name.clone(),
                            value: s.clone(),
                        });
                    }
                    Expr::Var(other) => {
                        block.instrs.push(Instruction::LinearMove {
                            dest: name.clone(),
                            src: other.clone(),
                        });
                    }
                    _ => {
                        let t = format!("__t{}", *temp_id);
                        *temp_id += 1;
                        block.instrs.push(Instruction::ConstInt {
                            dest: t.clone(),
                            value: 0,
                        });
                        block.instrs.push(Instruction::LinearMove {
                            dest: name.clone(),
                            src: t,
                        });
                    }
                }
                if let Some(cur) = scopes.last_mut() {
                    cur.push(name.clone());
                }
            }
            Stmt::Struct {
                name,
                fields,
                is_linear,
            } => {
                block.instrs.push(Instruction::StructDef {
                    name: name.clone(),
                    fields: fields.clone(),
                    is_linear: *is_linear,
                });
            }
            Stmt::Enum {
                name,
                variants,
                is_sealed: _,
            } => {
                block.instrs.push(Instruction::EnumDef {
                    name: name.clone(),
                    variants: variants.clone(),
                });
            }
            Stmt::ErrorSet { name, variants } => {
                block.instrs.push(Instruction::EnumDef {
                    name: name.clone(),
                    variants: variants.clone(),
                });
            }
            Stmt::Impl { .. } => {}
            Stmt::Trait { .. } => {}
            Stmt::TypeAlias { .. } => {}
            Stmt::Use { .. } => {}
            Stmt::GcMode { .. } => {}
            Stmt::CancelToken { .. } => {}
            Stmt::EffectHandler { .. } => {}
            Stmt::Spawn { .. } => {}
            Stmt::Channel { .. } => {}
            Stmt::Actor { .. } => {}
            Stmt::WorkStealingExecutor { .. } => {}
            Stmt::DeterministicRuntime { .. } => {}
            Stmt::Tensor { .. } => {}
            Stmt::Simd { .. } => {}
            Stmt::DocComment { .. } => {}
            Stmt::DebugSession { .. } => {}
            Stmt::Capability { .. } => {}
            Stmt::FfiSandbox { .. } => {}
        }
    }

    for stmt in &prog.stmts {
        lower_stmt(stmt, &mut block, &mut temp_id, &mut scopes);
    }

    if let Some(top) = scopes.pop() {
        for name in top.iter().rev() {
            block.instrs.push(Instruction::Drop { var: name.clone() });
        }
    }

    func.blocks.push(block);
    module.functions.push(func);

    for stmt in &prog.stmts {
        if let Stmt::Fn {
            name, params, body, ..
        } = stmt
        {
            if body.is_empty() {
                continue;
            }
            let mut func2 = MirFunction::new(&name);
            let mut block2 = BasicBlock::new(0);
            let mut temp_id_func = temp_id;
            let mut scopes_func: Vec<Vec<String>> = vec![Vec::new()];

            if let Some(cur) = scopes_func.last_mut() {
                for param in params {
                    cur.push(param.clone());
                }
            }

            for body_stmt in body {
                lower_stmt(body_stmt, &mut block2, &mut temp_id_func, &mut scopes_func);
            }

            if let Some(top) = scopes_func.pop() {
                for var_name in top.iter().rev() {
                    if !params.contains(var_name) {
                        block2.instrs.push(Instruction::Drop {
                            var: var_name.clone(),
                        });
                    }
                }
            }

            func2.blocks.push(block2);
            module.functions.push(func2);
        }
    }

    module
}

pub fn format_mir(module: &MirModule) -> String {
    let mut out = String::new();
    for f in &module.functions {
        out.push_str(&format!("fn {}:\n", f.name));
        for b in &f.blocks {
            out.push_str(&format!("  block{}:\n", b.id));
            for instr in &b.instrs {
                match instr {
                    Instruction::ConstInt { dest, value } => {
                        out.push_str(&format!("    {} = const_int {}\n", dest, value));
                    }
                    Instruction::ConstStr { dest, value } => {
                        out.push_str(&format!("    {} = const_str \"{}\"\n", dest, value));
                    }
                    Instruction::ConstBool { dest, value } => {
                        out.push_str(&format!("    {} = const_bool {}\n", dest, value));
                    }
                    Instruction::Move { dest, src } => {
                        out.push_str(&format!("    {} = move {}\n", dest, src));
                    }
                    Instruction::Drop { var } => {
                        out.push_str(&format!("    drop {}\n", var));
                    }
                    Instruction::Print { src } => {
                        out.push_str(&format!("    print {}\n", src));
                    }
                    Instruction::Jump { target } => {
                        out.push_str(&format!("    jump block{}\n", target));
                    }
                    Instruction::JumpIf { cond, target } => {
                        out.push_str(&format!("    jump_if {} block{}\n", cond, target));
                    }
                    Instruction::Label { id } => {
                        out.push_str(&format!("    label block{}\n", id));
                    }
                    Instruction::BinaryOp {
                        dest,
                        op,
                        left,
                        right,
                    } => {
                        out.push_str(&format!(
                            "    {} = binary_op {:?} {} {}\n",
                            dest, op, left, right
                        ));
                    }
                    Instruction::UnaryOp { dest, op, operand } => {
                        out.push_str(&format!("    {} = unary_op {:?} {}\n", dest, op, operand));
                    }
                    Instruction::Return { value } => {
                        out.push_str(&format!("    return {}\n", value));
                    }
                    Instruction::Assign { dest, src } => {
                        out.push_str(&format!("    {} = {}\n", dest, src));
                    }
                    Instruction::Call { dest, func, args } => {
                        out.push_str(&format!(
                            "    {} = call {}({})\n",
                            dest,
                            func,
                            args.join(", ")
                        ));
                    }
                    Instruction::FieldAccess { dest, base, field } => {
                        out.push_str(&format!("    {} = {}.{}\n", dest, base, field));
                    }
                    Instruction::StructAccess { dest, base, field } => {
                        out.push_str(&format!("    {} = {}.{}\n", dest, base, field));
                    }
                    Instruction::IndexAccess { dest, base, index } => {
                        out.push_str(&format!("    {} = {}[{}]\n", dest, base, index));
                    }
                    Instruction::StructDef {
                        name,
                        fields,
                        is_linear,
                    } => {
                        out.push_str(&format!("    struct {} linear={} {{\n", name, is_linear));
                        for (f_name, f_type) in fields {
                            out.push_str(&format!("      {}: {}\n", f_name, f_type));
                        }
                        out.push_str("    }\n");
                    }
                    Instruction::EnumDef { name, variants } => {
                        out.push_str(&format!("    enum {} {{\n", name));
                        for v in variants {
                            out.push_str(&format!("      {}\n", v.name));
                        }
                        out.push_str("    }\n");
                    }
                    Instruction::MatchBranch {
                        cond,
                        then_block,
                        else_block,
                    } => {
                        out.push_str(&format!(
                            "    match_branch {} -> block{} else block{}\n",
                            cond, then_block, else_block
                        ));
                    }
                    Instruction::LinearMove { dest, src } => {
                        out.push_str(&format!("    {} = linear_move {}\n", dest, src));
                    }
                    Instruction::DropLinear { var } => {
                        out.push_str(&format!("    drop_linear {}\n", var));
                    }
                }
            }
        }
    }
    out
}
