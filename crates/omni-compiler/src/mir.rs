use crate::ast::{Expr, Program, Stmt};
use crate::complete_lexer::TokenKind;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct MirModule {
    pub functions: Vec<MirFunction>,
    pub gc_mode: Option<String>,
    pub unsafe_blocks: Vec<UnsafeBlockInfo>,
}

#[derive(Debug, Clone)]
pub struct UnsafeBlockInfo {
    pub start_instr: usize,
    pub end_instr: usize,
    pub function: String,
    pub block: usize,
}

#[derive(Debug, Clone)]
pub struct MirFunction {
    pub name: String,
    /// Source-level parameter names in ABI order.
    pub params: Vec<String>,
    /// Source-level parameter type annotations in ABI order.  The type checker
    /// has already validated these before MIR is consumed by native lowering.
    /// Keeping them in MIR is required for the v0.1.4 value ABI: parameter
    /// names alone cannot distinguish scalar and indirect aggregate values.
    pub param_types: Vec<Option<String>>,
    /// Source-level declared return type. `None` is unit/inferred top-level
    /// bootstrap code.
    pub return_type: Option<String>,
    /// Whether this function produces one source-level return value in the current
    /// Stage-0 ABI. `false` represents a unit/void return.
    pub returns_value: bool,
    /// True only for compiler-synthesized top-level entry/initialization functions.
    pub synthetic: bool,
    pub blocks: Vec<BasicBlock>,
    pub is_safe_wrapper: bool,
    pub effects: Vec<String>,
}

impl MirFunction {
    pub fn new(name: &str, is_safe_wrapper: bool) -> Self {
        Self {
            name: name.to_string(),
            params: Vec::new(),
            param_types: Vec::new(),
            return_type: None,
            // Historical Stage-0 functions returned i64 by default. Source
            // functions overwrite this from their declared return type below.
            returns_value: true,
            synthetic: false,
            blocks: Vec::new(),
            is_safe_wrapper,
            effects: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: usize,
    pub instrs: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    ConstInt {
        dest: String,
        value: i64,
    },
    ConstStr {
        dest: String,
        value: String,
    },
    ConstBytes {
        dest: String,
        value: Vec<u8>,
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
    Borrow {
        dest: String,
        place: String,
        mutable: bool,
    },
    Reborrow {
        dest: String,
        parent: String,
        mutable: bool,
    },
    Deref {
        dest: String,
        reference: String,
    },
    DerefAssign {
        reference: String,
        src: String,
    },
    Return {
        value: String,
    },
    Assign {
        dest: String,
        src: String,
    },
    Spawn {
        func: String,
        args: Vec<String>,
    },
    Channel {
        dest: String,
        elem_type: String,
        capacity: Option<usize>,
    },
    Call {
        dest: String,
        func: String,
        args: Vec<String>,
    },
    AggregateInit {
        dest: String,
        type_name: String,
        fields: Vec<(String, String)>,
    },
    EnumInit {
        dest: String,
        type_name: String,
        variant: String,
        tag: u32,
        fields: Vec<(String, String)>,
    },
    EnumTag {
        dest: String,
        base: String,
    },
    EnumPayloadAccess {
        dest: String,
        base: String,
        index: u32,
    },
    FieldAccess {
        dest: String,
        base: String,
        field: String,
        linear: bool,
    },
    FieldAssign {
        base: String,
        field: String,
        src: String,
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
    SliceAccess {
        dest: String,
        base: String,
        start: String,
        end: String,
        inclusive: bool,
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
        Self {
            functions: Vec::new(),
            gc_mode: None,
            unsafe_blocks: Vec::new(),
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

#[derive(Debug, Clone)]
struct EnumConstructorInfo {
    type_name: String,
    variant: String,
    tag: u32,
    fields: Vec<String>,
}

#[derive(Debug, Default, Clone)]
struct LoweringContext {
    enum_constructors: HashMap<String, EnumConstructorInfo>,
    linear_bindings: std::collections::HashSet<String>,
}

impl LoweringContext {
    fn from_program(program: &Program) -> Self {
        let mut enum_constructors = HashMap::new();
        for stmt in &program.stmts {
            if let Stmt::Enum { name, variants, .. } = stmt {
                for (tag, variant) in variants.iter().enumerate() {
                    let Ok(tag) = u32::try_from(tag) else {
                        continue;
                    };
                    enum_constructors.insert(
                        format!("{name}::{}", variant.name),
                        EnumConstructorInfo {
                            type_name: name.clone(),
                            variant: variant.name.clone(),
                            tag,
                            fields: variant
                                .fields
                                .iter()
                                .map(|(field_name, _)| field_name.clone())
                                .collect(),
                        },
                    );
                }
            }
        }
        Self {
            enum_constructors,
            linear_bindings: std::collections::HashSet::new(),
        }
    }

    fn constructor(&self, path: &str) -> Option<&EnumConstructorInfo> {
        if let Some(info) = self.enum_constructors.get(path) {
            return Some(info);
        }
        let suffix = format!("::{path}");
        let mut matches = self
            .enum_constructors
            .iter()
            .filter(|(name, _)| name.ends_with(&suffix))
            .map(|(_, info)| info);
        let first = matches.next()?;
        if matches.next().is_some() {
            None
        } else {
            Some(first)
        }
    }

    fn enum_variant_count(&self, type_name: &str) -> usize {
        self.enum_constructors
            .values()
            .filter(|info| info.type_name == type_name)
            .count()
    }
}

fn collect_linear_bindings(stmts: &[Stmt], out: &mut std::collections::HashSet<String>) {
    for stmt in stmts {
        match stmt {
            Stmt::LetLinear(name, ..) => {
                out.insert(name.clone());
            }
            Stmt::Block(body, _)
            | Stmt::Loop { body, .. }
            | Stmt::While { body, .. }
            | Stmt::For { body, .. }
            | Stmt::WhileIn { body, .. }
            | Stmt::Unsafe { body, .. }
            | Stmt::UseScoped { body, .. } => collect_linear_bindings(body, out),
            Stmt::If {
                then_body,
                else_body,
                ..
            } => {
                collect_linear_bindings(then_body, out);
                collect_linear_bindings(else_body, out);
            }
            _ => {}
        }
    }
}

/// Emit a backend-visible sentinel for a source feature whose Edition-1
/// semantics are not implemented by the current MIR/native pipeline.  The
/// owned AOT backend rejects unresolved calls, so unsupported syntax can never
/// silently become a value such as `0`.
fn emit_unsupported_value(block: &mut BasicBlock, temp_id: &mut usize, feature: &str) -> String {
    let dest = format!("__unsupported_value{}", *temp_id);
    *temp_id += 1;
    block.instrs.push(Instruction::Call {
        dest: dest.clone(),
        func: format!("__omni_unsupported_{}", feature),
        args: Vec::new(),
    });
    dest
}

fn emit_unsupported_stmt(block: &mut BasicBlock, temp_id: &mut usize, feature: &str) {
    let _ = emit_unsupported_value(block, temp_id, feature);
}

fn lower_expr(
    expr: &Expr,
    block: &mut BasicBlock,
    temp_id: &mut usize,
    ctx: &LoweringContext,
) -> String {
    match expr {
        Expr::Number(n, _) => {
            let t = format!("__t{}", *temp_id);
            *temp_id += 1;
            block.instrs.push(Instruction::ConstInt {
                dest: t.clone(),
                value: *n,
            });
            t
        }
        Expr::StringLit(s, _) => {
            let t = format!("__t{}", *temp_id);
            *temp_id += 1;
            block.instrs.push(Instruction::ConstStr {
                dest: t.clone(),
                value: s.clone(),
            });
            t
        }
        Expr::ByteString(bytes, _) => {
            let t = format!("__t{}", *temp_id);
            *temp_id += 1;
            block.instrs.push(Instruction::ConstBytes {
                dest: t.clone(),
                value: bytes.clone(),
            });
            t
        }
        Expr::Byte(value, _) => {
            let t = format!("__t{}", *temp_id);
            *temp_id += 1;
            block.instrs.push(Instruction::ConstInt {
                dest: t.clone(),
                value: i64::from(*value),
            });
            t
        }
        Expr::Bool(b, _) => {
            let t = format!("__t{}", *temp_id);
            *temp_id += 1;
            block.instrs.push(Instruction::ConstBool {
                dest: t.clone(),
                value: *b,
            });
            t
        }
        Expr::Var(v, _) => v.clone(),
        Expr::BinaryOp {
            op, left, right, ..
        } => {
            let l = lower_expr(left, block, temp_id, ctx);
            let r = lower_expr(right, block, temp_id, ctx);
            let t = format!("__t{}", *temp_id);
            *temp_id += 1;
            block.instrs.push(Instruction::BinaryOp {
                dest: t.clone(),
                op: op.clone(),
                left: l,
                right: r,
            });
            t
        }
        Expr::UnaryOp { op, inner, .. } => {
            let src = lower_expr(inner, block, temp_id, ctx);
            let t = format!("__t{}", *temp_id);
            *temp_id += 1;
            block.instrs.push(Instruction::UnaryOp {
                dest: t.clone(),
                op: op.clone(),
                operand: src,
            });
            t
        }
        Expr::Borrow { mutable, inner, .. } => {
            let t = format!("__t{}", *temp_id);
            *temp_id += 1;
            match inner.as_ref() {
                Expr::Var(name, _) => block.instrs.push(Instruction::Borrow {
                    dest: t.clone(),
                    place: name.clone(),
                    mutable: *mutable,
                }),
                Expr::Deref { inner: parent, .. } => {
                    let parent = lower_expr(parent, block, temp_id, ctx);
                    block.instrs.push(Instruction::Reborrow {
                        dest: t.clone(),
                        parent,
                        mutable: *mutable,
                    });
                }
                _ => return emit_unsupported_value(block, temp_id, "borrow_nonlocal_place"),
            }
            t
        }
        Expr::Deref { inner, .. } => {
            let reference = lower_expr(inner, block, temp_id, ctx);
            let t = format!("__t{}", *temp_id);
            *temp_id += 1;
            block.instrs.push(Instruction::Deref {
                dest: t.clone(),
                reference,
            });
            t
        }
        Expr::Call(fname, args, _) => {
            let mut arg_values = Vec::new();
            for arg in args {
                arg_values.push(lower_expr(arg, block, temp_id, ctx));
            }
            let t = format!("__t{}", *temp_id);
            *temp_id += 1;
            if let Some(constructor) = ctx.enum_constructors.get(fname) {
                if constructor.fields.len() != arg_values.len() {
                    return emit_unsupported_value(
                        block,
                        temp_id,
                        "invalid_enum_constructor_arity",
                    );
                }
                let fields = constructor.fields.iter().cloned().zip(arg_values).collect();
                block.instrs.push(Instruction::EnumInit {
                    dest: t.clone(),
                    type_name: constructor.type_name.clone(),
                    variant: constructor.variant.clone(),
                    tag: constructor.tag,
                    fields,
                });
            } else {
                block.instrs.push(Instruction::Call {
                    dest: t.clone(),
                    func: fname.clone(),
                    args: arg_values,
                });
            }
            t
        }
        Expr::FieldAccess { base, field, .. } => {
            let linear =
                matches!(base.as_ref(), Expr::Var(name, _) if ctx.linear_bindings.contains(name));
            let b = lower_expr(base, block, temp_id, ctx);
            let t = format!("__t{}", *temp_id);
            *temp_id += 1;
            block.instrs.push(Instruction::FieldAccess {
                dest: t.clone(),
                base: b,
                field: field.clone(),
                linear,
            });
            t
        }
        Expr::IfExpr {
            cond, then, else_, ..
        } => {
            let cond_var = lower_expr(cond, block, temp_id, ctx);
            let then_block_id = *temp_id;
            *temp_id += 1;
            let else_block_id = *temp_id;
            *temp_id += 1;
            let end_block_id = *temp_id;
            *temp_id += 1;
            let result_var = format!("__t{}", *temp_id);
            *temp_id += 1;

            block.instrs.push(Instruction::JumpIf {
                cond: cond_var,
                target: then_block_id,
            });
            block.instrs.push(Instruction::Jump {
                target: else_block_id,
            });
            block.instrs.push(Instruction::Label { id: then_block_id });

            let then_val = lower_expr(then, block, temp_id, ctx);
            block.instrs.push(Instruction::Move {
                dest: result_var.clone(),
                src: then_val,
            });
            block.instrs.push(Instruction::Jump {
                target: end_block_id,
            });

            block.instrs.push(Instruction::Label { id: else_block_id });
            let else_val = lower_expr(else_, block, temp_id, ctx);
            block.instrs.push(Instruction::Move {
                dest: result_var.clone(),
                src: else_val,
            });

            block.instrs.push(Instruction::Label { id: end_block_id });
            result_var
        }
        Expr::Block(_, _) => emit_unsupported_value(block, temp_id, "block_expression"),
        Expr::Tuple(items, _) => {
            let mut fields = Vec::with_capacity(items.len());
            for (index, item) in items.iter().enumerate() {
                let value = lower_expr(item, block, temp_id, ctx);
                fields.push((index.to_string(), value));
            }
            let dest = format!("__t{}", *temp_id);
            *temp_id += 1;
            block.instrs.push(Instruction::AggregateInit {
                dest: dest.clone(),
                type_name: "Tuple".to_string(),
                fields,
            });
            dest
        }
        Expr::Array(items, _) => {
            let mut fields = Vec::with_capacity(items.len());
            for (index, item) in items.iter().enumerate() {
                let value = lower_expr(item, block, temp_id, ctx);
                fields.push((index.to_string(), value));
            }
            let dest = format!("__t{}", *temp_id);
            *temp_id += 1;
            block.instrs.push(Instruction::AggregateInit {
                dest: dest.clone(),
                type_name: "Array".to_string(),
                fields,
            });
            dest
        }
        Expr::Index(base, index, _) => {
            let b = lower_expr(base, block, temp_id, ctx);
            if let Expr::Range {
                start,
                end,
                inclusive,
                ..
            } = index.as_ref()
            {
                let start = lower_expr(start, block, temp_id, ctx);
                let end = lower_expr(end, block, temp_id, ctx);
                let t = format!("__t{}", *temp_id);
                *temp_id += 1;
                block.instrs.push(Instruction::SliceAccess {
                    dest: t.clone(),
                    base: b,
                    start,
                    end,
                    inclusive: *inclusive,
                });
                t
            } else {
                let idx = lower_expr(index, block, temp_id, ctx);
                let t = format!("__t{}", *temp_id);
                *temp_id += 1;
                block.instrs.push(Instruction::IndexAccess {
                    dest: t.clone(),
                    base: b,
                    index: idx,
                });
                t
            }
        }
        Expr::Match {
            expr: scrutinee,
            arms,
            ..
        } => {
            if arms.iter().any(|arm| arm.guard.is_some()) {
                return emit_unsupported_value(block, temp_id, "guarded_match_expression");
            }

            let scrutinee_value = lower_expr(scrutinee, block, temp_id, ctx);
            let result = format!("__t{}", *temp_id);
            *temp_id += 1;
            let end_label = *temp_id;
            *temp_id += 1;
            let mut terminated = false;

            // When every arm is a distinct variant of one declared enum, the final
            // variant is selected by elimination after all earlier tag checks. This
            // relies on TYPE-0005: every safe enum value has one declared live variant.
            let exhaustive_enum_type = {
                let mut enum_name: Option<String> = None;
                let mut tags = Vec::new();
                let mut valid = !arms.is_empty();
                for arm in arms {
                    let crate::ast::Pattern::Struct(path, _) = &arm.pattern else {
                        valid = false;
                        break;
                    };
                    let Some(constructor) = ctx.constructor(path) else {
                        valid = false;
                        break;
                    };
                    if let Some(existing) = &enum_name {
                        if existing != &constructor.type_name {
                            valid = false;
                            break;
                        }
                    } else {
                        enum_name = Some(constructor.type_name.clone());
                    }
                    if tags.contains(&constructor.tag) {
                        valid = false;
                        break;
                    }
                    tags.push(constructor.tag);
                }
                enum_name.filter(|name| valid && tags.len() == ctx.enum_variant_count(name))
            };

            for (arm_index, arm) in arms.iter().enumerate() {
                if terminated {
                    break;
                }
                let arm_label = *temp_id;
                *temp_id += 1;
                let next_label = *temp_id;
                *temp_id += 1;

                match &arm.pattern {
                    crate::ast::Pattern::Struct(path, fields) => {
                        let Some(constructor) = ctx.constructor(path).cloned() else {
                            return emit_unsupported_value(
                                block,
                                temp_id,
                                "unresolved_enum_match_variant",
                            );
                        };
                        if constructor.fields.len() != fields.len() {
                            return emit_unsupported_value(
                                block,
                                temp_id,
                                "invalid_enum_match_payload_arity",
                            );
                        }

                        let final_exhaustive_variant = exhaustive_enum_type
                            .as_deref()
                            .is_some_and(|name| name == constructor.type_name)
                            && arm_index + 1 == arms.len();

                        if !final_exhaustive_variant {
                            let tag_value = format!("__t{}", *temp_id);
                            *temp_id += 1;
                            block.instrs.push(Instruction::EnumTag {
                                dest: tag_value.clone(),
                                base: scrutinee_value.clone(),
                            });
                            let expected_tag = format!("__t{}", *temp_id);
                            *temp_id += 1;
                            block.instrs.push(Instruction::ConstInt {
                                dest: expected_tag.clone(),
                                value: i64::from(constructor.tag),
                            });
                            let matches = format!("__t{}", *temp_id);
                            *temp_id += 1;
                            block.instrs.push(Instruction::BinaryOp {
                                dest: matches.clone(),
                                op: TokenKind::EqEq,
                                left: tag_value,
                                right: expected_tag,
                            });
                            block.instrs.push(Instruction::JumpIf {
                                cond: matches,
                                target: arm_label,
                            });
                            block.instrs.push(Instruction::Jump { target: next_label });
                            block.instrs.push(Instruction::Label { id: arm_label });
                        }

                        for (index, (_, nested_pattern)) in fields.iter().enumerate() {
                            match nested_pattern {
                                crate::ast::Pattern::Var(name) => {
                                    let payload = format!("__t{}", *temp_id);
                                    *temp_id += 1;
                                    block.instrs.push(Instruction::EnumPayloadAccess {
                                        dest: payload.clone(),
                                        base: scrutinee_value.clone(),
                                        index: u32::try_from(index).unwrap_or(u32::MAX),
                                    });
                                    block.instrs.push(Instruction::Move {
                                        dest: name.clone(),
                                        src: payload,
                                    });
                                }
                                crate::ast::Pattern::Wildcard => {}
                                _ => {
                                    return emit_unsupported_value(
                                        block,
                                        temp_id,
                                        "nested_enum_payload_pattern",
                                    );
                                }
                            }
                        }

                        let body = lower_expr(&arm.body, block, temp_id, ctx);
                        block.instrs.push(Instruction::Move {
                            dest: result.clone(),
                            src: body,
                        });
                        block.instrs.push(Instruction::Jump { target: end_label });
                        if final_exhaustive_variant {
                            terminated = true;
                        } else {
                            block.instrs.push(Instruction::Label { id: next_label });
                        }
                    }
                    crate::ast::Pattern::Literal(value) => {
                        let literal = format!("__t{}", *temp_id);
                        *temp_id += 1;
                        block.instrs.push(Instruction::ConstInt {
                            dest: literal.clone(),
                            value: *value,
                        });
                        let matches = format!("__t{}", *temp_id);
                        *temp_id += 1;
                        block.instrs.push(Instruction::BinaryOp {
                            dest: matches.clone(),
                            op: TokenKind::EqEq,
                            left: scrutinee_value.clone(),
                            right: literal,
                        });
                        block.instrs.push(Instruction::JumpIf {
                            cond: matches,
                            target: arm_label,
                        });
                        block.instrs.push(Instruction::Jump { target: next_label });
                        block.instrs.push(Instruction::Label { id: arm_label });
                        let body = lower_expr(&arm.body, block, temp_id, ctx);
                        block.instrs.push(Instruction::Move {
                            dest: result.clone(),
                            src: body,
                        });
                        block.instrs.push(Instruction::Jump { target: end_label });
                        block.instrs.push(Instruction::Label { id: next_label });
                    }
                    crate::ast::Pattern::Wildcard => {
                        let body = lower_expr(&arm.body, block, temp_id, ctx);
                        block.instrs.push(Instruction::Move {
                            dest: result.clone(),
                            src: body,
                        });
                        block.instrs.push(Instruction::Jump { target: end_label });
                        terminated = true;
                    }
                    crate::ast::Pattern::Var(name) => {
                        block.instrs.push(Instruction::Move {
                            dest: name.clone(),
                            src: scrutinee_value.clone(),
                        });
                        let body = lower_expr(&arm.body, block, temp_id, ctx);
                        block.instrs.push(Instruction::Move {
                            dest: result.clone(),
                            src: body,
                        });
                        block.instrs.push(Instruction::Jump { target: end_label });
                        terminated = true;
                    }
                    crate::ast::Pattern::Or(_) => {
                        return emit_unsupported_value(block, temp_id, "or_pattern_match");
                    }
                }
            }

            if !terminated {
                emit_unsupported_stmt(block, temp_id, "non_exhaustive_match_runtime");
            }
            block.instrs.push(Instruction::Label { id: end_label });
            result
        }
        Expr::Interpolated(_, _) => emit_unsupported_value(block, temp_id, "interpolated_string"),
        Expr::Float(_, _) => emit_unsupported_value(block, temp_id, "floating_point"),
        Expr::Char(_, _) => emit_unsupported_value(block, temp_id, "char_value"),
        Expr::Range { .. } => emit_unsupported_value(block, temp_id, "range_value"),
        Expr::Lambda { .. } => emit_unsupported_value(block, temp_id, "lambda"),
        Expr::Await(_, _) => emit_unsupported_value(block, temp_id, "await"),
        Expr::Try(_, _) => emit_unsupported_value(block, temp_id, "try_operator"),
        Expr::StructLit { name, fields, .. } => {
            let mut lowered_fields = Vec::with_capacity(fields.len());
            for (field_name, expr) in fields {
                let value = lower_expr(expr, block, temp_id, ctx);
                lowered_fields.push((field_name.clone(), value));
            }
            let dest = format!("__t{}", *temp_id);
            *temp_id += 1;
            block.instrs.push(Instruction::AggregateInit {
                dest: dest.clone(),
                type_name: name.clone(),
                fields: lowered_fields,
            });
            dest
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct LoopContext {
    break_target: usize,
    continue_target: usize,
    scope_base: usize,
}

fn emit_loop_scope_cleanup(block: &mut BasicBlock, scopes: &[Vec<String>], scope_base: usize) {
    for scope in scopes.iter().skip(scope_base).rev() {
        for name in scope.iter().rev() {
            block.instrs.push(Instruction::Drop { var: name.clone() });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_stmt(
    stmt: &Stmt,
    block: &mut BasicBlock,
    temp_id: &mut usize,
    scopes: &mut Vec<Vec<String>>,
    in_unsafe_block: &mut bool,
    unsafe_instr_start: &mut usize,
    module: &mut MirModule,
    func_name: &str,
    ctx: &LoweringContext,
) {
    lower_stmt_with_loop(
        stmt,
        block,
        temp_id,
        scopes,
        in_unsafe_block,
        unsafe_instr_start,
        module,
        func_name,
        None,
        ctx,
    );
}

#[allow(clippy::too_many_arguments)]
fn lower_stmt_with_loop(
    stmt: &Stmt,
    block: &mut BasicBlock,
    temp_id: &mut usize,
    scopes: &mut Vec<Vec<String>>,
    in_unsafe_block: &mut bool,
    unsafe_instr_start: &mut usize,
    module: &mut MirModule,
    func_name: &str,
    loop_ctx: Option<LoopContext>,
    ctx: &LoweringContext,
) {
    match stmt {
        Stmt::Let(name, _type_ann, expr, _) | Stmt::LetMut(name, _type_ann, expr, _) => {
            let src = lower_expr(expr, block, temp_id, ctx);
            if src != name.as_str() {
                let rebound_borrow = block.instrs.last_mut().is_some_and(|instr| {
                    match instr {
                        Instruction::Borrow { dest, .. } | Instruction::Reborrow { dest, .. }
                            if dest == &src =>
                        {
                            *dest = name.clone();
                            return true;
                        }
                        _ => {}
                    }
                    false
                });
                if !rebound_borrow {
                    block.instrs.push(Instruction::Move {
                        dest: name.clone(),
                        src,
                    });
                }
            }
            if let Some(cur) = scopes.last_mut() {
                cur.push(name.clone());
            }
        }
        Stmt::Print(expr, _) => {
            let src = lower_expr(expr, block, temp_id, ctx);
            block.instrs.push(Instruction::Print { src });
        }
        Stmt::Annotation(_, _) => {}
        Stmt::ExprStmt(expr, _) => {
            lower_expr(expr, block, temp_id, ctx);
        }
        Stmt::Block(inner, _) => {
            scopes.push(Vec::new());
            for s in inner {
                lower_stmt_with_loop(
                    s,
                    block,
                    temp_id,
                    scopes,
                    in_unsafe_block,
                    unsafe_instr_start,
                    module,
                    func_name,
                    loop_ctx,
                    ctx,
                );
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

            let cond_var = lower_expr(cond, block, temp_id, ctx);
            block.instrs.push(Instruction::JumpIf {
                cond: cond_var,
                target: then_block_id,
            });

            scopes.push(Vec::new());
            for s in else_body {
                lower_stmt_with_loop(
                    s,
                    block,
                    temp_id,
                    scopes,
                    in_unsafe_block,
                    unsafe_instr_start,
                    module,
                    func_name,
                    loop_ctx,
                    ctx,
                );
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
                lower_stmt_with_loop(
                    s,
                    block,
                    temp_id,
                    scopes,
                    in_unsafe_block,
                    unsafe_instr_start,
                    module,
                    func_name,
                    loop_ctx,
                    ctx,
                );
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
        Stmt::Loop { body, .. } => {
            let loop_start = *temp_id;
            *temp_id += 1;
            let loop_end = *temp_id;
            *temp_id += 1;

            block.instrs.push(Instruction::Jump { target: loop_start });
            block.instrs.push(Instruction::Label { id: loop_start });

            let loop_scope_base = scopes.len();
            scopes.push(Vec::new());
            let nested_loop_ctx = Some(LoopContext {
                break_target: loop_end,
                continue_target: loop_start,
                scope_base: loop_scope_base,
            });
            for s in body {
                lower_stmt_with_loop(
                    s,
                    block,
                    temp_id,
                    scopes,
                    in_unsafe_block,
                    unsafe_instr_start,
                    module,
                    func_name,
                    nested_loop_ctx,
                    ctx,
                );
            }
            if let Some(decls) = scopes.pop() {
                for name in decls.iter().rev() {
                    block.instrs.push(Instruction::Drop { var: name.clone() });
                }
            }

            block.instrs.push(Instruction::Jump { target: loop_start });
            block.instrs.push(Instruction::Label { id: loop_end });
        }
        Stmt::For { .. } => {
            emit_unsupported_stmt(block, temp_id, "for_loop");
        }
        Stmt::While { cond, body, .. } => {
            // Reserve three independent labels. The previous lowering reused
            // `while_start + 1` as both body and end, which made the loop CFG
            // ambiguous after LIR label patching.
            let while_cond = *temp_id;
            *temp_id += 1;
            let while_body = *temp_id;
            *temp_id += 1;
            let while_end = *temp_id;
            *temp_id += 1;

            block.instrs.push(Instruction::Label { id: while_cond });
            let cond_var = lower_expr(cond, block, temp_id, ctx);
            block.instrs.push(Instruction::JumpIf {
                cond: cond_var,
                target: while_body,
            });
            block.instrs.push(Instruction::Jump { target: while_end });
            block.instrs.push(Instruction::Label { id: while_body });

            let loop_scope_base = scopes.len();
            scopes.push(Vec::new());
            let nested_loop_ctx = Some(LoopContext {
                break_target: while_end,
                continue_target: while_cond,
                scope_base: loop_scope_base,
            });
            for s in body {
                lower_stmt_with_loop(
                    s,
                    block,
                    temp_id,
                    scopes,
                    in_unsafe_block,
                    unsafe_instr_start,
                    module,
                    func_name,
                    nested_loop_ctx,
                    ctx,
                );
            }
            if let Some(decls) = scopes.pop() {
                for name in decls.iter().rev() {
                    block.instrs.push(Instruction::Drop { var: name.clone() });
                }
            }

            block.instrs.push(Instruction::Jump { target: while_cond });
            block.instrs.push(Instruction::Label { id: while_end });
        }
        Stmt::Return(expr, _) => {
            let val = lower_expr(expr, block, temp_id, ctx);
            block.instrs.push(Instruction::Return { value: val });
        }
        Stmt::Break(_) => {
            if let Some(ctx) = loop_ctx {
                emit_loop_scope_cleanup(block, scopes, ctx.scope_base);
                block.instrs.push(Instruction::Jump {
                    target: ctx.break_target,
                });
            } else {
                emit_unsupported_stmt(block, temp_id, "break_outside_loop");
            }
        }
        Stmt::Continue(_) => {
            if let Some(ctx) = loop_ctx {
                emit_loop_scope_cleanup(block, scopes, ctx.scope_base);
                block.instrs.push(Instruction::Jump {
                    target: ctx.continue_target,
                });
            } else {
                emit_unsupported_stmt(block, temp_id, "continue_outside_loop");
            }
        }
        Stmt::Assign(name, expr, _) => {
            let src = lower_expr(expr, block, temp_id, ctx);
            block.instrs.push(Instruction::Assign {
                dest: name.clone(),
                src,
            });
        }
        Stmt::DerefAssign(reference, expr, _) => {
            let reference = lower_expr(reference, block, temp_id, ctx);
            let src = lower_expr(expr, block, temp_id, ctx);
            block
                .instrs
                .push(Instruction::DerefAssign { reference, src });
        }
        Stmt::ExprFieldAssign(base, field, expr, _) => {
            let base_var = lower_expr(base, block, temp_id, ctx);
            let value_var = lower_expr(expr, block, temp_id, ctx);
            block.instrs.push(Instruction::FieldAssign {
                base: base_var,
                field: field.clone(),
                src: value_var,
            });
        }
        Stmt::WhileIn { .. } => {
            emit_unsupported_stmt(block, temp_id, "while_in");
        }
        Stmt::Unsafe { body, .. } => {
            let was_unsafe = *in_unsafe_block;
            *in_unsafe_block = true;
            *unsafe_instr_start = block.instrs.len();

            for s in body {
                lower_stmt_with_loop(
                    s,
                    block,
                    temp_id,
                    scopes,
                    in_unsafe_block,
                    unsafe_instr_start,
                    module,
                    func_name,
                    loop_ctx,
                    ctx,
                );
            }

            module.unsafe_blocks.push(UnsafeBlockInfo {
                start_instr: *unsafe_instr_start,
                end_instr: block.instrs.len(),
                function: func_name.to_string(),
                block: block.id,
            });

            *in_unsafe_block = was_unsafe;
        }
        Stmt::LetLinear(name, _type_ann, expr, _) => {
            let src = lower_expr(expr, block, temp_id, ctx);
            block.instrs.push(Instruction::LinearMove {
                dest: name.clone(),
                src,
            });
            if let Some(cur) = scopes.last_mut() {
                cur.push(name.clone());
            }
        }
        Stmt::Struct {
            name,
            fields,
            is_linear,
            ..
        } => {
            block.instrs.push(Instruction::StructDef {
                name: name.clone(),
                fields: fields.clone(),
                is_linear: *is_linear,
            });
        }
        Stmt::Enum { name, variants, .. } => {
            block.instrs.push(Instruction::EnumDef {
                name: name.clone(),
                variants: variants.clone(),
            });
        }
        Stmt::ErrorSet { name, variants, .. } => {
            block.instrs.push(Instruction::EnumDef {
                name: name.clone(),
                variants: variants.clone(),
            });
        }
        Stmt::Impl { .. } => {
            emit_unsupported_stmt(block, temp_id, "impl_runtime_dispatch");
        }
        Stmt::Trait { .. } => {}
        Stmt::TypeAlias { .. } => {}
        Stmt::Use { .. } => {}
        Stmt::GcMode { .. } => {
            emit_unsupported_stmt(block, temp_id, "managed_memory");
        }
        Stmt::CancelToken { .. } => {
            emit_unsupported_stmt(block, temp_id, "cancellation");
        }
        Stmt::EffectHandler { .. } => {
            emit_unsupported_stmt(block, temp_id, "effect_handler_runtime");
        }
        Stmt::Spawn { .. } => {
            emit_unsupported_stmt(block, temp_id, "structured_concurrency_spawn");
        }
        Stmt::Channel { .. } => {
            emit_unsupported_stmt(block, temp_id, "channel_runtime");
        }
        Stmt::Actor { .. } => emit_unsupported_stmt(block, temp_id, "actor_runtime"),
        Stmt::WorkStealingExecutor { .. } => {
            emit_unsupported_stmt(block, temp_id, "work_stealing_executor")
        }
        Stmt::DeterministicRuntime { .. } => {
            emit_unsupported_stmt(block, temp_id, "deterministic_runtime")
        }
        Stmt::Tensor { .. } => emit_unsupported_stmt(block, temp_id, "tensor_runtime"),
        Stmt::Simd { .. } => emit_unsupported_stmt(block, temp_id, "simd_runtime"),
        Stmt::DocComment { .. } => {}
        Stmt::DebugSession { .. } => emit_unsupported_stmt(block, temp_id, "debug_session"),
        Stmt::Capability { .. } => emit_unsupported_stmt(block, temp_id, "capability_runtime"),
        Stmt::FfiSandbox { .. } => emit_unsupported_stmt(block, temp_id, "ffi_sandbox"),
        Stmt::UseScoped { body, .. } => {
            for s in body {
                lower_stmt_with_loop(
                    s,
                    block,
                    temp_id,
                    scopes,
                    in_unsafe_block,
                    unsafe_instr_start,
                    module,
                    func_name,
                    loop_ctx,
                    ctx,
                );
            }
        }
        Stmt::ContractRequires { .. } => emit_unsupported_stmt(block, temp_id, "contracts"),
        Stmt::ContractEnsures { .. } => emit_unsupported_stmt(block, temp_id, "contracts"),
        Stmt::ContractInvariant { .. } => emit_unsupported_stmt(block, temp_id, "contracts"),
        Stmt::ComptimeLimit { .. } => emit_unsupported_stmt(block, temp_id, "comptime_limit"),
        Stmt::Mod(_, _) | Stmt::ModBlock(_, _, _) => {}
    }
}

pub fn lower_program_to_mir(prog: &Program) -> MirModule {
    let mut ctx = LoweringContext::from_program(prog);
    let top_level_stmts: Vec<Stmt> = prog
        .stmts
        .iter()
        .filter(|stmt| !matches!(stmt, Stmt::Fn { .. }))
        .cloned()
        .collect();
    collect_linear_bindings(&top_level_stmts, &mut ctx.linear_bindings);
    let mut module = MirModule::new();
    for stmt in &prog.stmts {
        if let Stmt::GcMode { mode, .. } = stmt {
            module.gc_mode = Some(mode.clone());
            break;
        }
    }

    let gc_mode = module.gc_mode.clone();
    let has_user_main = prog.stmts.iter().any(|stmt| {
        if let Stmt::Fn { name, .. } = stmt {
            name == "main"
        } else {
            false
        }
    });
    let top_level_name = if has_user_main {
        "__top_level_init"
    } else {
        "main"
    };
    let mut func = MirFunction::new(top_level_name, false);
    func.synthetic = true;
    func.returns_value = top_level_name == "main";
    let mut block = BasicBlock::new(0);
    let mut temp_id: usize = 0;
    let mut scopes: Vec<Vec<String>> = vec![Vec::new()];
    let mut in_unsafe_block = false;
    let mut unsafe_instr_start: usize = 0;

    for stmt in &prog.stmts {
        lower_stmt(
            stmt,
            &mut block,
            &mut temp_id,
            &mut scopes,
            &mut in_unsafe_block,
            &mut unsafe_instr_start,
            &mut module,
            top_level_name,
            &ctx,
        );
    }

    if gc_mode.as_deref() != Some("refcount") {
        if let Some(top) = scopes.pop() {
            for name in top.iter().rev() {
                block.instrs.push(Instruction::Drop { var: name.clone() });
            }
        }
    }

    func.blocks.push(block);
    module.functions.push(func);

    for i in 0..prog.stmts.len() {
        let stmt = &prog.stmts[i];
        if let Stmt::Fn {
            name,
            is_async,
            params,
            ret_type,
            contracts,
            body,
            effects,
            ..
        } = stmt
        {
            if body.is_empty() {
                continue;
            }
            let mut is_safe_wrapper = false;
            if i > 0 {
                if let Stmt::Annotation(annot, _) = &prog.stmts[i - 1] {
                    if annot == "safe_wrapper" {
                        is_safe_wrapper = true;
                    }
                }
            }
            let mut func2 = MirFunction::new(name, is_safe_wrapper);
            func2.params = params.iter().map(|(param, _)| param.clone()).collect();
            func2.param_types = params.iter().map(|(_, ty)| ty.clone()).collect();
            func2.return_type = ret_type.clone();
            func2.returns_value = ret_type
                .as_deref()
                .map(|ty| !matches!(ty.to_ascii_lowercase().as_str(), "unit" | "void" | "()"))
                .unwrap_or(false);
            func2.effects = effects.clone();
            let mut block2 = BasicBlock::new(0);
            let mut temp_id_func = temp_id;
            let mut scopes_func: Vec<Vec<String>> = vec![Vec::new()];
            let mut in_unsafe_block_func = false;
            let mut unsafe_instr_start_func: usize = 0;

            if *is_async {
                emit_unsupported_stmt(&mut block2, &mut temp_id_func, "async_function");
            }
            if !contracts.is_empty() {
                emit_unsupported_stmt(&mut block2, &mut temp_id_func, "function_contracts");
            }

            if let Some(cur) = scopes_func.last_mut() {
                for param in params {
                    cur.push(param.0.clone());
                }
            }

            let param_names: Vec<String> = params.iter().map(|(p, _)| p.clone()).collect();

            let mut fn_ctx = ctx.clone();
            fn_ctx.linear_bindings.clear();
            collect_linear_bindings(body, &mut fn_ctx.linear_bindings);

            // v0.1.2 requires explicit `return` for value-returning
            // functions. Edition-1 tail expressions need parser/CST support
            // that preserves semicolon presence; silently treating every final
            // ExprStmt as a return would make `expr;` and `expr` indistinguishable.
            for body_stmt in body {
                lower_stmt(
                    body_stmt,
                    &mut block2,
                    &mut temp_id_func,
                    &mut scopes_func,
                    &mut in_unsafe_block_func,
                    &mut unsafe_instr_start_func,
                    &mut module,
                    name,
                    &fn_ctx,
                );
            }

            if gc_mode.as_deref() != Some("refcount") {
                if let Some(top) = scopes_func.pop() {
                    for var_name in top.iter().rev() {
                        if !param_names.contains(var_name) {
                            block2.instrs.push(Instruction::Drop {
                                var: var_name.clone(),
                            });
                        }
                    }
                }
            }

            func2.blocks.push(block2);
            module.functions.push(func2);
        }
    }

    module
}

/// Validate structural control-flow invariants required by every backend.
///
/// This verifier intentionally runs after MIR optimization as well as before
/// LIR lowering. A compiler bug that creates a duplicate or dangling label is
/// therefore reported as a compiler diagnostic rather than becoming an
/// arbitrary native branch target.
pub fn validate_control_flow(module: &MirModule) -> Result<(), String> {
    use std::collections::HashSet;

    for function in &module.functions {
        let mut labels = HashSet::new();
        let mut targets = Vec::new();
        for block in &function.blocks {
            for (index, instr) in block.instrs.iter().enumerate() {
                match instr {
                    Instruction::Label { id } => {
                        if !labels.insert(*id) {
                            return Err(format!(
                                "function '{}': duplicate MIR label {} at instruction {}",
                                function.name, id, index
                            ));
                        }
                    }
                    Instruction::Jump { target } => targets.push((*target, index, "jump")),
                    Instruction::JumpIf { target, .. } => {
                        targets.push((*target, index, "conditional jump"))
                    }
                    _ => {}
                }
            }
        }
        for (target, index, kind) in targets {
            if !labels.contains(&target) {
                return Err(format!(
                    "function '{}': {} at instruction {} targets missing MIR label {}",
                    function.name, kind, index, target
                ));
            }
        }
    }
    Ok(())
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
                    Instruction::ConstBytes { dest, value } => {
                        out.push_str(&format!("    {} = const_bytes {:?}\n", dest, value));
                    }
                    Instruction::ConstBool { dest, value } => {
                        out.push_str(&format!("    {} = const_bool {}\n", dest, value));
                    }
                    Instruction::Move { dest, src } => {
                        out.push_str(&format!("    {} = move {}\n", dest, src));
                    }
                    Instruction::Borrow {
                        dest,
                        place,
                        mutable,
                    } => {
                        let prefix = if *mutable { "&mut " } else { "&" };
                        out.push_str(&format!("    {} = borrow {}{}\n", dest, prefix, place));
                    }
                    Instruction::Reborrow {
                        dest,
                        parent,
                        mutable,
                    } => {
                        let prefix = if *mutable { "&mut *" } else { "&*" };
                        out.push_str(&format!("    {} = reborrow {}{}\n", dest, prefix, parent));
                    }
                    Instruction::Deref { dest, reference } => {
                        out.push_str(&format!("    {} = deref {}\n", dest, reference));
                    }
                    Instruction::DerefAssign { reference, src } => {
                        out.push_str(&format!("    *{} = {}\n", reference, src));
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
                    Instruction::AggregateInit {
                        dest,
                        type_name,
                        fields,
                    } => {
                        let values: Vec<String> = fields
                            .iter()
                            .map(|(name, value)| format!("{name}: {value}"))
                            .collect();
                        out.push_str(&format!(
                            "    {} = aggregate {} {{ {} }}\n",
                            dest,
                            type_name,
                            values.join(", ")
                        ));
                    }
                    Instruction::EnumInit {
                        dest,
                        type_name,
                        variant,
                        tag,
                        fields,
                    } => {
                        let values: Vec<String> = fields
                            .iter()
                            .map(|(name, value)| format!("{name}: {value}"))
                            .collect();
                        out.push_str(&format!(
                            "    {} = enum {}::{} tag={} {{ {} }}\n",
                            dest,
                            type_name,
                            variant,
                            tag,
                            values.join(", ")
                        ));
                    }
                    Instruction::EnumTag { dest, base } => {
                        out.push_str(&format!("    {} = enum_tag {}\n", dest, base));
                    }
                    Instruction::EnumPayloadAccess { dest, base, index } => {
                        out.push_str(&format!(
                            "    {} = enum_payload {}[{}]\n",
                            dest, base, index
                        ));
                    }
                    Instruction::FieldAccess {
                        dest, base, field, ..
                    } => {
                        out.push_str(&format!("    {} = {}.{}\n", dest, base, field));
                    }
                    Instruction::FieldAssign { base, field, src } => {
                        out.push_str(&format!("    {}.{} = {}\n", base, field, src));
                    }
                    Instruction::StructAccess { dest, base, field } => {
                        out.push_str(&format!("    {} = {}.{}\n", dest, base, field));
                    }
                    Instruction::IndexAccess { dest, base, index } => {
                        out.push_str(&format!("    {} = {}[{}]\n", dest, base, index));
                    }
                    Instruction::SliceAccess {
                        dest,
                        base,
                        start,
                        end,
                        inclusive,
                    } => {
                        let sep = if *inclusive { "..." } else { ".." };
                        out.push_str(&format!(
                            "    {} = {}[{}{}{}]\n",
                            dest, base, start, sep, end
                        ));
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
                        let vars: Vec<String> = variants.iter().map(|v| v.name.clone()).collect();
                        out.push_str(&format!(
                            "    enum_def {} with variants {}\n",
                            name,
                            vars.join(", ")
                        ));
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
                    Instruction::Spawn { func, args } => {
                        out.push_str(&format!("    spawn {}({})\n", func, args.join(", ")));
                    }
                    Instruction::Channel {
                        dest,
                        elem_type,
                        capacity,
                    } => {
                        out.push_str(&format!(
                            "    {} = channel<{}> capacity={:?}\n",
                            dest, elem_type, capacity
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

pub fn is_in_unsafe_block(
    module: &MirModule,
    func_name: &str,
    block_id: usize,
    instr_idx: usize,
) -> bool {
    for ub in &module.unsafe_blocks {
        if ub.function == func_name
            && ub.block == block_id
            && instr_idx >= ub.start_instr
            && instr_idx < ub.end_instr
        {
            return true;
        }
    }
    false
}

pub fn validate_unsafe_usage(module: &MirModule) -> Vec<String> {
    let mut warnings = Vec::new();

    for func in &module.functions {
        for block in &func.blocks {
            for (idx, instr) in block.instrs.iter().enumerate() {
                let needs_unsafe = matches!(instr, Instruction::DropLinear { .. });

                if needs_unsafe && !is_in_unsafe_block(module, &func.name, block.id, idx) {
                    warnings.push(format!(
                        "Unsafe operation {:?} in function '{}' block {} at instruction {} is not within an unsafe block",
                        instr, func.name, block.id, idx
                    ));
                }
            }
        }
    }

    warnings
}
