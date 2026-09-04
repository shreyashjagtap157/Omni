use crate::mir::{Instruction, MirFunction, MirModule};
use std::collections::HashMap;

pub struct RegionInfo {
    pub name: String,
    pub start_block: usize,
    pub start_instr: usize,
    pub end_block: usize,
    pub end_instr: usize,
    pub lifetime_start: usize,
    pub lifetime_end: usize,
    pub parent_region: Option<String>,
    pub is_universal: bool,
}

impl RegionInfo {
    pub fn new(
        name: String,
        start_block: usize,
        start_instr: usize,
        end_block: usize,
        end_instr: usize,
    ) -> Self {
        RegionInfo {
            name,
            start_block,
            start_instr,
            end_block,
            end_instr,
            lifetime_start: start_block,
            lifetime_end: end_block,
            parent_region: None,
            is_universal: false,
        }
    }

    pub fn with_lifetime(mut self, start: usize, end: usize) -> Self {
        self.lifetime_start = start;
        self.lifetime_end = end;
        self
    }

    pub fn with_parent(mut self, parent: String) -> Self {
        self.parent_region = Some(parent);
        self
    }

    pub fn mark_universal(mut self) -> Self {
        self.is_universal = true;
        self
    }

    pub fn contains_point(&self, block: usize, instr: usize) -> bool {
        if block < self.start_block || block > self.end_block {
            return false;
        }
        if block == self.start_block && instr < self.start_instr {
            return false;
        }
        if block == self.end_block && instr > self.end_instr {
            return false;
        }
        true
    }
}

pub struct LoanInfo {
    pub name: String,
    pub region: String,
    pub borrower: String,
    pub kind: LoanKind,
}

#[derive(Debug, Clone)]
pub enum LoanKind {
    Shared,
    Exclusive,
    Mutable,
}

pub fn export_polonius_input(module: &MirModule) -> String {
    let mut out = String::new();
    for f in &module.functions {
        if f.is_safe_wrapper {
            continue;
        }
        out.push_str(&format!("function {}\n", f.name));
        for b in &f.blocks {
            out.push_str(&format!(" block {}\n", b.id));
            for (i, instr) in b.instrs.iter().enumerate() {
                match instr {
                    Instruction::ConstInt { dest, value } => {
                        out.push_str(&format!("  {}: const_int {}\n", dest, value))
                    }
                    Instruction::ConstStr { dest, value } => {
                        out.push_str(&format!("  {}: const_str \"{}\"\n", dest, value))
                    }
                    Instruction::ConstBytes { dest, value } => {
                        out.push_str(&format!("  {}: const_bytes {:?}\n", dest, value))
                    }
                    Instruction::ConstBool { dest, value } => {
                        out.push_str(&format!("  {}: const_bool {}\n", dest, value))
                    }
                    Instruction::Move { dest, src } => {
                        out.push_str(&format!("  {}: move {}\n", dest, src))
                    }
                    Instruction::Borrow {
                        dest,
                        place,
                        mutable,
                    } => out.push_str(&format!(
                        "  {}: borrow {}{}\n",
                        dest,
                        if *mutable { "&mut " } else { "&" },
                        place
                    )),
                    Instruction::Reborrow {
                        dest,
                        parent,
                        mutable,
                    } => out.push_str(&format!(
                        "  {}: reborrow {}{}\n",
                        dest,
                        if *mutable { "&mut *" } else { "&*" },
                        parent
                    )),
                    Instruction::Deref { dest, reference } => {
                        out.push_str(&format!("  {}: deref {}\n", dest, reference))
                    }
                    Instruction::DerefAssign { reference, src } => {
                        out.push_str(&format!("  {}: deref_assign {} {}\n", i, reference, src))
                    }
                    Instruction::Print { src } => {
                        out.push_str(&format!("  {}: print {}\n", i, src))
                    }
                    Instruction::Drop { var } => out.push_str(&format!("  {}: drop {}\n", i, var)),
                    Instruction::Jump { target } => {
                        out.push_str(&format!("  {}: jump block{}\n", i, target))
                    }
                    Instruction::JumpIf { cond, target } => {
                        out.push_str(&format!("  {}: jump_if {} block{}\n", i, cond, target))
                    }
                    Instruction::Label { id } => {
                        out.push_str(&format!("  {}: label block{}\n", i, id))
                    }
                    Instruction::BinaryOp {
                        dest,
                        op,
                        left,
                        right,
                    } => out.push_str(&format!(
                        "  {}: binary_op {:?} {} {}\n",
                        dest, op, left, right
                    )),
                    Instruction::UnaryOp { dest, op, operand } => {
                        out.push_str(&format!("  {}: unary_op {:?} {}\n", dest, op, operand))
                    }
                    Instruction::Return { value } => {
                        out.push_str(&format!("  {}: return {}\n", i, value))
                    }
                    Instruction::Assign { dest, src } => {
                        out.push_str(&format!("  {}: assign {} = {}\n", dest, dest, src))
                    }
                    Instruction::Call { dest, func, args } => out.push_str(&format!(
                        "  {}: call {} ({})\n",
                        dest,
                        func,
                        args.join(", ")
                    )),
                    Instruction::AggregateInit {
                        dest,
                        type_name,
                        fields,
                    } => {
                        let values: Vec<String> = fields
                            .iter()
                            .map(|(name, value)| format!("{name}:{value}"))
                            .collect();
                        out.push_str(&format!(
                            "  {}: aggregate {} {{{}}}\n",
                            dest,
                            type_name,
                            values.join(",")
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
                            .map(|(name, value)| format!("{name}:{value}"))
                            .collect();
                        out.push_str(&format!(
                            "  {}: enum {}::{} tag={} {{{}}}\n",
                            dest,
                            type_name,
                            variant,
                            tag,
                            values.join(",")
                        ));
                    }
                    Instruction::EnumTag { dest, base } => {
                        out.push_str(&format!("  {}: enum_tag {}\n", dest, base));
                    }
                    Instruction::EnumPayloadAccess { dest, base, index } => {
                        out.push_str(&format!("  {}: enum_payload {}[{}]\n", dest, base, index));
                    }
                    Instruction::FieldAccess {
                        dest, base, field, ..
                    } => out.push_str(&format!("  {}: field_access {}.{}\n", dest, base, field)),
                    Instruction::FieldAssign { base, field, src } => out.push_str(&format!(
                        "  {}: field_assign {}.{} <- {}\n",
                        i, base, field, src
                    )),
                    Instruction::StructAccess { dest, base, field } => {
                        out.push_str(&format!("  {}: struct_access {}.{}\n", dest, base, field))
                    }
                    Instruction::IndexAccess { dest, base, index } => {
                        out.push_str(&format!("  {}: index_access {}[{}]\n", dest, base, index))
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
                            "  {}: slice_access {}[{}{}{}]\n",
                            dest, base, start, sep, end
                        ));
                    }
                    Instruction::LinearMove { dest, src } => {
                        out.push_str(&format!("  {}: linear_move {}\n", dest, src))
                    }
                    Instruction::DropLinear { var } => {
                        out.push_str(&format!("  {}: drop_linear {}\n", i, var))
                    }
                    Instruction::StructDef {
                        name,
                        fields,
                        is_linear,
                    } => {
                        let field_strs: Vec<String> = fields
                            .iter()
                            .map(|(n, t)| format!("{}: {}", n, t))
                            .collect();
                        out.push_str(&format!(
                            "  {}: struct_def {} with {} linear={}\n",
                            i,
                            name,
                            field_strs.join(", "),
                            is_linear
                        ))
                    }
                    Instruction::EnumDef { name, variants } => {
                        let var_strs: Vec<String> =
                            variants.iter().map(|v| v.name.clone()).collect();
                        out.push_str(&format!(
                            "  {}: enum_def {} with variants {}\n",
                            i,
                            name,
                            var_strs.join(", ")
                        ))
                    }
                    Instruction::MatchBranch {
                        cond,
                        then_block,
                        else_block,
                    } => out.push_str(&format!(
                        "  {}: match_branch {} -> block{} else block{}\n",
                        i, cond, then_block, else_block
                    )),
                    Instruction::Spawn { func, args } => {
                        out.push_str(&format!("  {}: spawn {} ({})\n", i, func, args.join(", ")))
                    }
                    Instruction::Channel {
                        dest,
                        elem_type,
                        capacity,
                    } => out.push_str(&format!(
                        "  {}: channel {}<{}> capacity={:?}\n",
                        i, dest, elem_type, capacity
                    )),
                }
            }
        }
    }
    out
}

// For convenience, also expose a combined textual exporter that includes the
// human-readable instruction listing followed by the region/loan facts
// (points, def/use/move/drop/jump relations). Tests and the adapter expect
// the `point`/`def` style lines to be available, so append them here.
pub fn export_polonius_input_with_region_facts(module: &MirModule) -> String {
    let mut out = export_polonius_input(module);
    out.push('\n');
    for line in generate_region_loan_facts(module) {
        out.push_str(&line);
        out.push('\n');
    }
    out
}

pub fn build_polonius_facts(module: &MirModule) -> Vec<String> {
    export_polonius_input_with_region_facts(module)
        .lines()
        .map(|s| s.to_string())
        .collect()
}

pub fn generate_region_loan_facts(module: &MirModule) -> Vec<String> {
    let mut facts: Vec<String> = Vec::new();
    for f in &module.functions {
        if f.is_safe_wrapper {
            continue;
        }
        for b in &f.blocks {
            for (i, instr) in b.instrs.iter().enumerate() {
                facts.push(format!("point {} {} {}", f.name, b.id, i));
                match instr {
                    Instruction::ConstInt { dest, .. }
                    | Instruction::ConstStr { dest, .. }
                    | Instruction::ConstBytes { dest, .. }
                    | Instruction::ConstBool { dest, .. } => {
                        facts.push(format!("def {} {} {} {}", f.name, b.id, i, dest));
                    }
                    Instruction::Move { dest, src } => {
                        facts.push(format!("move {} {} {} {} {}", f.name, b.id, i, src, dest));
                        facts.push(format!("use {} {} {} {}", f.name, b.id, i, src));
                        facts.push(format!("def {} {} {} {}", f.name, b.id, i, dest));
                    }
                    Instruction::Borrow {
                        dest,
                        place,
                        mutable,
                    } => {
                        facts.push(format!(
                            "borrow {} {} {} {} {} {}",
                            f.name,
                            b.id,
                            i,
                            place,
                            dest,
                            if *mutable { "mutable" } else { "shared" }
                        ));
                        facts.push(format!("use {} {} {} {}", f.name, b.id, i, place));
                        facts.push(format!("def {} {} {} {}", f.name, b.id, i, dest));
                    }
                    Instruction::Reborrow {
                        dest,
                        parent,
                        mutable,
                    } => {
                        facts.push(format!(
                            "reborrow {} {} {} {} {} {}",
                            f.name,
                            b.id,
                            i,
                            parent,
                            dest,
                            if *mutable { "mutable" } else { "shared" }
                        ));
                        facts.push(format!("use {} {} {} {}", f.name, b.id, i, parent));
                        facts.push(format!("def {} {} {} {}", f.name, b.id, i, dest));
                    }
                    Instruction::Deref { dest, reference } => {
                        facts.push(format!(
                            "deref {} {} {} {} {}",
                            f.name, b.id, i, reference, dest
                        ));
                        facts.push(format!("use {} {} {} {}", f.name, b.id, i, reference));
                        facts.push(format!("def {} {} {} {}", f.name, b.id, i, dest));
                    }
                    Instruction::DerefAssign { reference, src } => {
                        facts.push(format!(
                            "deref_assign {} {} {} {} {}",
                            f.name, b.id, i, reference, src
                        ));
                        facts.push(format!("use {} {} {} {}", f.name, b.id, i, reference));
                        facts.push(format!("use {} {} {} {}", f.name, b.id, i, src));
                    }
                    Instruction::LinearMove { dest, src } => {
                        facts.push(format!(
                            "linear_move {} {} {} {} {}",
                            f.name, b.id, i, src, dest
                        ));
                    }
                    Instruction::Call {
                        dest,
                        func: _,
                        args,
                    } => {
                        // Call defines its destination and uses its arguments
                        facts.push(format!("def {} {} {} {}", f.name, b.id, i, dest));
                        for arg in args {
                            facts.push(format!("use {} {} {} {}", f.name, b.id, i, arg));
                        }
                    }
                    Instruction::Print { src } => {
                        facts.push(format!("use {} {} {} {}", f.name, b.id, i, src));
                    }
                    Instruction::Drop { var } => {
                        facts.push(format!("drop {} {} {} {}", f.name, b.id, i, var));
                    }
                    Instruction::DropLinear { var } => {
                        facts.push(format!("drop_linear {} {} {} {}", f.name, b.id, i, var));
                    }
                    Instruction::Jump { target } => {
                        facts.push(format!("jump {} {} {} {}", f.name, b.id, i, target));
                    }
                    Instruction::JumpIf { cond, target } => {
                        facts.push(format!(
                            "jump_if {} {} {} {} {}",
                            f.name, b.id, i, cond, target
                        ));
                    }
                    Instruction::Label { id } => {
                        facts.push(format!("label {} {} {} {}", f.name, b.id, i, id));
                    }
                    Instruction::BinaryOp {
                        dest, left, right, ..
                    } => {
                        facts.push(format!("def {} {} {} {}", f.name, b.id, i, dest));
                        facts.push(format!("use {} {} {} {}", f.name, b.id, i, left));
                        facts.push(format!("use {} {} {} {}", f.name, b.id, i, right));
                    }
                    Instruction::UnaryOp { dest, operand, .. } => {
                        facts.push(format!("def {} {} {} {}", f.name, b.id, i, dest));
                        facts.push(format!("use {} {} {} {}", f.name, b.id, i, operand));
                    }
                    Instruction::Return { value } => {
                        facts.push(format!("use {} {} {} {}", f.name, b.id, i, value));
                    }
                    Instruction::Assign { dest, src } => {
                        facts.push(format!("def {} {} {} {}", f.name, b.id, i, dest));
                        facts.push(format!("use {} {} {} {}", f.name, b.id, i, src));
                    }
                    Instruction::AggregateInit { dest, fields, .. } => {
                        facts.push(format!("def {} {} {} {}", f.name, b.id, i, dest));
                        for (_, value) in fields {
                            facts.push(format!("use {} {} {} {}", f.name, b.id, i, value));
                        }
                    }
                    Instruction::EnumInit {
                        dest,
                        type_name,
                        variant,
                        tag,
                        fields,
                    } => {
                        facts.push(format!("def {} {} {} {}", f.name, b.id, i, dest));
                        for (_, value) in fields {
                            facts.push(format!("use {} {} {} {}", f.name, b.id, i, value));
                        }
                        facts.push(format!(
                            "enum_init {} {} {} {}::{} tag={}",
                            f.name, b.id, i, type_name, variant, tag
                        ));
                    }
                    Instruction::EnumTag { dest, base } => {
                        facts.push(format!("def {} {} {} {}", f.name, b.id, i, dest));
                        facts.push(format!("use {} {} {} {}", f.name, b.id, i, base));
                    }
                    Instruction::EnumPayloadAccess { dest, base, index } => {
                        facts.push(format!("def {} {} {} {}", f.name, b.id, i, dest));
                        facts.push(format!("use {} {} {} {}", f.name, b.id, i, base));
                        facts.push(format!(
                            "enum_payload {} {} {} {}[{}]",
                            f.name, b.id, i, base, index
                        ));
                    }
                    // Removed unreachable Call branch (handled by previous generic Call arm)
                    Instruction::FieldAccess {
                        dest, base, field, ..
                    } => {
                        facts.push(format!("def {} {} {} {}", f.name, b.id, i, dest));
                        facts.push(format!("use {} {} {} {}", f.name, b.id, i, base));
                        facts.push(format!(
                            "field {} {} {} {}.{}",
                            f.name, b.id, i, base, field
                        ));
                    }
                    Instruction::FieldAssign { base, field, src } => {
                        facts.push(format!("use {} {} {} {}", f.name, b.id, i, src));
                        facts.push(format!(
                            "field_assign {} {} {} {}.{}",
                            f.name, b.id, i, base, field
                        ));
                    }
                    Instruction::StructAccess { dest, base, field } => {
                        facts.push(format!("def {} {} {} {}", f.name, b.id, i, dest));
                        facts.push(format!("use {} {} {} {}", f.name, b.id, i, base));
                        facts.push(format!(
                            "struct_field {} {} {} {}.{}",
                            f.name, b.id, i, base, field
                        ));
                    }
                    Instruction::IndexAccess { dest, base, index } => {
                        facts.push(format!("def {} {} {} {}", f.name, b.id, i, dest));
                        facts.push(format!("use {} {} {} {}", f.name, b.id, i, base));
                        facts.push(format!("use {} {} {} {}", f.name, b.id, i, index));
                        facts.push(format!(
                            "index {} {} {} {}[{}]",
                            f.name, b.id, i, base, index
                        ));
                    }
                    Instruction::SliceAccess {
                        dest,
                        base,
                        start,
                        end,
                        inclusive,
                    } => {
                        facts.push(format!("def {} {} {} {}", f.name, b.id, i, dest));
                        for value in [base, start, end] {
                            facts.push(format!("use {} {} {} {}", f.name, b.id, i, value));
                        }
                        facts.push(format!(
                            "slice {} {} {} {}[{}{}{}]",
                            f.name,
                            b.id,
                            i,
                            base,
                            start,
                            if *inclusive { "..." } else { ".." },
                            end
                        ));
                    }
                    Instruction::StructDef {
                        name,
                        fields,
                        is_linear,
                    } => {
                        let fstr: Vec<String> =
                            fields.iter().map(|(n, t)| format!("{}:{}", n, t)).collect();
                        facts.push(format!(
                            "struct_def {} {} {} {} linear={}",
                            f.name, b.id, i, name, is_linear
                        ));
                        facts.push(format!("struct_fields {}", fstr.join(",")));
                    }
                    Instruction::EnumDef { name, variants } => {
                        let vstr: Vec<String> = variants.iter().map(|v| v.name.clone()).collect();
                        facts.push(format!(
                            "enum_def {} {} {} variants={}",
                            f.name, b.id, i, name
                        ));
                        facts.push(format!("enum_variants {}", vstr.join(",")));
                    }
                    Instruction::MatchBranch { cond, .. } => {
                        facts.push(format!("use {} {} {} {}", f.name, b.id, i, cond));
                    }
                    Instruction::Spawn { args, .. } => {
                        for arg in args {
                            facts.push(format!("use {} {} {} {}", f.name, b.id, i, arg));
                        }
                    }
                    Instruction::Channel { dest, .. } => {
                        facts.push(format!("def {} {} {} {}", f.name, b.id, i, dest));
                    }
                }
            }
        }
    }

    // Simple intra-block liveness: for each variable, find def sites and uses within the same block
    // and emit `live` facts for points between a def and its last use before the next def.
    for f in &module.functions {
        for b in &f.blocks {
            let mut defs: HashMap<String, Vec<usize>> = HashMap::new();
            let mut uses: HashMap<String, Vec<usize>> = HashMap::new();
            for (i, instr) in b.instrs.iter().enumerate() {
                match instr {
                    Instruction::ConstInt { dest, .. }
                    | Instruction::ConstStr { dest, .. }
                    | Instruction::ConstBytes { dest, .. }
                    | Instruction::ConstBool { dest, .. } => {
                        defs.entry(dest.clone()).or_default().push(i);
                    }
                    Instruction::BinaryOp {
                        dest, left, right, ..
                    } => {
                        defs.entry(dest.clone()).or_default().push(i);
                        uses.entry(left.clone()).or_default().push(i);
                        uses.entry(right.clone()).or_default().push(i);
                    }
                    Instruction::UnaryOp { dest, operand, .. } => {
                        defs.entry(dest.clone()).or_default().push(i);
                        uses.entry(operand.clone()).or_default().push(i);
                    }
                    Instruction::Assign { dest, src } => {
                        defs.entry(dest.clone()).or_default().push(i);
                        uses.entry(src.clone()).or_default().push(i);
                    }
                    Instruction::Borrow { dest, place, .. } => {
                        defs.entry(dest.clone()).or_default().push(i);
                        uses.entry(place.clone()).or_default().push(i);
                    }
                    Instruction::Reborrow { dest, parent, .. } => {
                        defs.entry(dest.clone()).or_default().push(i);
                        uses.entry(parent.clone()).or_default().push(i);
                    }
                    Instruction::Deref { dest, reference } => {
                        defs.entry(dest.clone()).or_default().push(i);
                        uses.entry(reference.clone()).or_default().push(i);
                    }
                    Instruction::DerefAssign { reference, src } => {
                        uses.entry(reference.clone()).or_default().push(i);
                        uses.entry(src.clone()).or_default().push(i);
                    }
                    Instruction::Call { dest, args, .. } => {
                        defs.entry(dest.clone()).or_default().push(i);
                        for a in args {
                            uses.entry(a.clone()).or_default().push(i);
                        }
                    }
                    Instruction::AggregateInit { dest, fields, .. }
                    | Instruction::EnumInit { dest, fields, .. } => {
                        defs.entry(dest.clone()).or_default().push(i);
                        for (_, value) in fields {
                            uses.entry(value.clone()).or_default().push(i);
                        }
                    }
                    Instruction::FieldAssign { base, src, .. } => {
                        uses.entry(base.clone()).or_default().push(i);
                        uses.entry(src.clone()).or_default().push(i);
                    }
                    Instruction::EnumTag { dest, base }
                    | Instruction::EnumPayloadAccess { dest, base, .. }
                    | Instruction::FieldAccess { dest, base, .. }
                    | Instruction::StructAccess { dest, base, .. } => {
                        defs.entry(dest.clone()).or_default().push(i);
                        uses.entry(base.clone()).or_default().push(i);
                    }
                    Instruction::IndexAccess { dest, base, index } => {
                        defs.entry(dest.clone()).or_default().push(i);
                        uses.entry(base.clone()).or_default().push(i);
                        uses.entry(index.clone()).or_default().push(i);
                    }
                    Instruction::SliceAccess {
                        dest,
                        base,
                        start,
                        end,
                        ..
                    } => {
                        defs.entry(dest.clone()).or_default().push(i);
                        uses.entry(base.clone()).or_default().push(i);
                        uses.entry(start.clone()).or_default().push(i);
                        uses.entry(end.clone()).or_default().push(i);
                    }
                    Instruction::LinearMove { dest, src } => {
                        defs.entry(dest.clone()).or_default().push(i);
                        uses.entry(src.clone()).or_default().push(i);
                    }
                    Instruction::Move { dest, src } => {
                        defs.entry(dest.clone()).or_default().push(i);
                        uses.entry(src.clone()).or_default().push(i);
                    }
                    Instruction::Print { src } => {
                        uses.entry(src.clone()).or_default().push(i);
                    }
                    Instruction::Return { value } => {
                        uses.entry(value.clone()).or_default().push(i);
                    }
                    Instruction::Drop { var } | Instruction::DropLinear { var } => {
                        uses.entry(var.clone()).or_default().push(i);
                    }
                    Instruction::Jump { .. }
                    | Instruction::JumpIf { .. }
                    | Instruction::Label { .. }
                    | Instruction::StructDef { .. }
                    | Instruction::EnumDef { .. }
                    | Instruction::MatchBranch { .. } => {}
                    Instruction::Spawn { args, .. } => {
                        for a in args {
                            uses.entry(a.clone()).or_default().push(i);
                        }
                    }
                    Instruction::Channel { dest, .. } => {
                        defs.entry(dest.clone()).or_default().push(i);
                    }
                }
            }

            // For each def, find the last use before the next def (or block end) and emit live facts.
            for (var, def_positions) in defs.iter() {
                for (idx, &def_pos) in def_positions.iter().enumerate() {
                    let next_def = def_positions
                        .get(idx + 1)
                        .copied()
                        .unwrap_or(b.instrs.len());
                    // find max use u such that def_pos <= u < next_def
                    if let Some(all_uses) = uses.get(var) {
                        let mut last_use_in_segment: Option<usize> = None;
                        for &u in all_uses.iter() {
                            if u >= def_pos && u < next_def {
                                if let Some(prev) = last_use_in_segment {
                                    if u > prev {
                                        last_use_in_segment = Some(u);
                                    }
                                } else {
                                    last_use_in_segment = Some(u);
                                }
                            }
                        }
                        if let Some(lu) = last_use_in_segment {
                            for point in def_pos..=lu {
                                facts.push(format!("live {} {} {} {}", f.name, b.id, point, var));
                            }
                        }
                    }
                }
            }
        }
    }

    // Cross-block liveness: emit `live` facts for uses that occur in later blocks
    // for a given definition. This is a conservative, simple pass that looks
    // at defs and uses across the whole function (using the block order in
    // `f.blocks`) and marks use points as live when they are dominated by a
    // prior definition and occur before the next definition of the same var.
    for f in &module.functions {
        // Build function-level def/use positions indexed by (block_index, instr)
        let mut func_defs: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
        let mut func_uses: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
        for (block_idx, b) in f.blocks.iter().enumerate() {
            for (i, instr) in b.instrs.iter().enumerate() {
                match instr {
                    Instruction::ConstInt { dest, .. }
                    | Instruction::ConstStr { dest, .. }
                    | Instruction::ConstBytes { dest, .. }
                    | Instruction::ConstBool { dest, .. } => {
                        func_defs
                            .entry(dest.clone())
                            .or_default()
                            .push((block_idx, i));
                    }
                    Instruction::BinaryOp {
                        dest, left, right, ..
                    } => {
                        func_defs
                            .entry(dest.clone())
                            .or_default()
                            .push((block_idx, i));
                        func_uses
                            .entry(left.clone())
                            .or_default()
                            .push((block_idx, i));
                        func_uses
                            .entry(right.clone())
                            .or_default()
                            .push((block_idx, i));
                    }
                    Instruction::UnaryOp { dest, operand, .. } => {
                        func_defs
                            .entry(dest.clone())
                            .or_default()
                            .push((block_idx, i));
                        func_uses
                            .entry(operand.clone())
                            .or_default()
                            .push((block_idx, i));
                    }
                    Instruction::Assign { dest, src } => {
                        func_defs
                            .entry(dest.clone())
                            .or_default()
                            .push((block_idx, i));
                        func_uses
                            .entry(src.clone())
                            .or_default()
                            .push((block_idx, i));
                    }
                    Instruction::Call { dest, args, .. } => {
                        func_defs
                            .entry(dest.clone())
                            .or_default()
                            .push((block_idx, i));
                        for a in args {
                            func_uses.entry(a.clone()).or_default().push((block_idx, i));
                        }
                    }
                    Instruction::AggregateInit { dest, fields, .. }
                    | Instruction::EnumInit { dest, fields, .. } => {
                        func_defs
                            .entry(dest.clone())
                            .or_default()
                            .push((block_idx, i));
                        for (_, value) in fields {
                            func_uses
                                .entry(value.clone())
                                .or_default()
                                .push((block_idx, i));
                        }
                    }
                    Instruction::EnumTag { dest, base }
                    | Instruction::EnumPayloadAccess { dest, base, .. }
                    | Instruction::FieldAccess { dest, base, .. }
                    | Instruction::StructAccess { dest, base, .. } => {
                        func_defs
                            .entry(dest.clone())
                            .or_default()
                            .push((block_idx, i));
                        func_uses
                            .entry(base.clone())
                            .or_default()
                            .push((block_idx, i));
                    }
                    Instruction::IndexAccess { dest, base, index } => {
                        func_defs
                            .entry(dest.clone())
                            .or_default()
                            .push((block_idx, i));
                        for value in [base, index] {
                            func_uses
                                .entry(value.clone())
                                .or_default()
                                .push((block_idx, i));
                        }
                    }
                    Instruction::SliceAccess {
                        dest,
                        base,
                        start,
                        end,
                        ..
                    } => {
                        func_defs
                            .entry(dest.clone())
                            .or_default()
                            .push((block_idx, i));
                        for value in [base, start, end] {
                            func_uses
                                .entry(value.clone())
                                .or_default()
                                .push((block_idx, i));
                        }
                    }
                    Instruction::LinearMove { dest, src } => {
                        func_defs
                            .entry(dest.clone())
                            .or_default()
                            .push((block_idx, i));
                        func_uses
                            .entry(src.clone())
                            .or_default()
                            .push((block_idx, i));
                    }
                    Instruction::Move { dest, src } => {
                        func_defs
                            .entry(dest.clone())
                            .or_default()
                            .push((block_idx, i));
                        func_uses
                            .entry(src.clone())
                            .or_default()
                            .push((block_idx, i));
                    }
                    Instruction::Print { src } => {
                        func_uses
                            .entry(src.clone())
                            .or_default()
                            .push((block_idx, i));
                    }
                    Instruction::Return { value } => {
                        func_uses
                            .entry(value.clone())
                            .or_default()
                            .push((block_idx, i));
                    }
                    Instruction::Drop { var } | Instruction::DropLinear { var } => {
                        func_uses
                            .entry(var.clone())
                            .or_default()
                            .push((block_idx, i));
                    }
                    Instruction::Spawn { args, .. } => {
                        for a in args {
                            func_uses.entry(a.clone()).or_default().push((block_idx, i));
                        }
                    }
                    Instruction::Channel { dest, .. } => {
                        func_defs
                            .entry(dest.clone())
                            .or_default()
                            .push((block_idx, i));
                    }
                    _ => {}
                }
            }
        }

        // Sort def/use positions and emit live facts for uses that fall between
        // a definition and the next definition (possibly in a later block).
        for defs in func_defs.values_mut() {
            defs.sort_unstable();
        }
        for (var, def_positions) in func_defs.iter() {
            if let Some(all_uses) = func_uses.get(var) {
                let mut uses_sorted = all_uses.clone();
                uses_sorted.sort_unstable();
                for (idx, def_pos) in def_positions.iter().enumerate() {
                    let next_def = def_positions.get(idx + 1).copied();
                    for &use_pos in uses_sorted.iter() {
                        // use_pos >= def_pos and, when there is another definition,
                        // use_pos < next_def. Avoid a sentinel position so the model
                        // stays meaningful on every target pointer width.
                        let ge = (use_pos.0 > def_pos.0)
                            || (use_pos.0 == def_pos.0 && use_pos.1 >= def_pos.1);
                        let lt = next_def.is_none_or(|next| use_pos < next);
                        if ge && lt {
                            let (use_block_idx, use_instr) = use_pos;
                            if let Some(b) = f.blocks.get(use_block_idx) {
                                facts.push(format!(
                                    "live {} {} {} {}",
                                    f.name, b.id, use_instr, var
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    facts
}

/// The experimental ownership oracle has been archived to
/// `docs/archive/polonius/`. This function always fails closed.
pub fn run_polonius_adapter(_module: &MirModule) -> Result<(), String> {
    Err(
        "experimental ownership oracle has been archived; full ownership checking is a v0.2.0 milestone"
            .to_string(),
    )
}

pub fn check_mir(module: &MirModule) -> Result<(), String> {
    use std::collections::HashMap;
    let signatures: HashMap<String, Vec<Option<bool>>> = module
        .functions
        .iter()
        .map(|function| {
            let params = function
                .param_types
                .iter()
                .map(|annotation| reference_annotation(annotation.as_deref()))
                .collect();
            (function.name.clone(), params)
        })
        .collect();
    for function in &module.functions {
        check_function_local_shared_borrows(function, &signatures)?;
        check_function_linear_places(function)?;
    }
    Ok(())
}

fn reference_annotation(annotation: Option<&str>) -> Option<bool> {
    let annotation = annotation?.trim();
    if annotation.starts_with("&mut ") {
        Some(true)
    } else if annotation.starts_with('&') {
        Some(false)
    } else {
        None
    }
}

fn check_function_local_shared_borrows(
    function: &MirFunction,
    signatures: &std::collections::HashMap<String, Vec<Option<bool>>>,
) -> Result<(), String> {
    use std::collections::{HashMap, HashSet};

    #[derive(Debug, Clone)]
    struct Loan {
        reference: String,
        origin: String,
        mutable: bool,
        start: usize,
        end: usize,
        parent: Option<String>,
    }

    let instructions: Vec<&Instruction> = function.blocks.iter().flat_map(|b| &b.instrs).collect();
    let mut aggregate_places = HashSet::new();
    let mut linear_places = HashSet::new();
    for instr in &instructions {
        match instr {
            Instruction::AggregateInit { dest, .. } | Instruction::EnumInit { dest, .. } => {
                aggregate_places.insert(dest.clone());
            }
            Instruction::LinearMove { dest, .. } => {
                linear_places.insert(dest.clone());
            }
            _ => {}
        }
    }

    let mut last_use: HashMap<String, usize> = HashMap::new();
    for (index, instr) in instructions.iter().enumerate() {
        match instr {
            Instruction::Deref { reference, .. } | Instruction::DerefAssign { reference, .. } => {
                last_use.insert(reference.clone(), index);
            }
            Instruction::Reborrow { parent, .. } => {
                last_use.insert(parent.clone(), index);
            }
            Instruction::Call { args, .. } | Instruction::Spawn { args, .. } => {
                for arg in args {
                    last_use.insert(arg.clone(), index);
                }
            }
            Instruction::Return { value } => {
                last_use.insert(value.clone(), index);
            }
            Instruction::Move { src, .. }
            | Instruction::Assign { src, .. }
            | Instruction::LinearMove { src, .. } => {
                last_use.insert(src.clone(), index);
            }
            _ => {}
        }
    }

    let mut loans: Vec<Loan> = Vec::new();
    let mut loan_by_ref: HashMap<String, usize> = HashMap::new();
    for (position, param) in function.params.iter().enumerate() {
        let Some(mutable) = function
            .param_types
            .get(position)
            .and_then(|annotation| reference_annotation(annotation.as_deref()))
        else {
            continue;
        };
        if let Some(end) = last_use.get(param).copied() {
            let loan_index = loans.len();
            loans.push(Loan {
                reference: param.clone(),
                origin: format!("@param:{param}"),
                mutable,
                start: 0,
                end,
                parent: None,
            });
            loan_by_ref.insert(param.clone(), loan_index);
        }
    }
    for (index, instr) in instructions.iter().enumerate() {
        match instr {
            Instruction::Borrow {
                dest,
                place,
                mutable,
            } => {
                if aggregate_places.contains(place) || place.contains('.') {
                    return Err(format!(
                        "function '{}': only named scalar local borrows are qualified; got '{}'",
                        function.name, place
                    ));
                }
                let Some(end) = last_use.get(dest).copied() else {
                    return Err(format!(
                        "function '{}': reference '{}' is never used; escaping/stored references are not yet qualified",
                        function.name, dest
                    ));
                };
                let loan_index = loans.len();
                loans.push(Loan {
                    reference: dest.clone(),
                    origin: place.clone(),
                    mutable: *mutable,
                    start: index,
                    end,
                    parent: None,
                });
                loan_by_ref.insert(dest.clone(), loan_index);
            }
            Instruction::Reborrow {
                dest,
                parent,
                mutable,
            } => {
                let Some(&parent_index) = loan_by_ref.get(parent) else {
                    return Err(format!(
                        "function '{}': reborrow '{}' has no proven parent loan",
                        function.name, parent
                    ));
                };
                let parent_loan = &loans[parent_index];
                if *mutable && !parent_loan.mutable {
                    return Err(format!(
                        "function '{}': cannot create mutable reborrow '{}' from shared reference '{}'",
                        function.name, dest, parent
                    ));
                }
                let Some(end) = last_use.get(dest).copied() else {
                    return Err(format!(
                        "function '{}': reborrow '{}' is never used",
                        function.name, dest
                    ));
                };
                let loan_index = loans.len();
                loans.push(Loan {
                    reference: dest.clone(),
                    origin: parent_loan.origin.clone(),
                    mutable: *mutable,
                    start: index,
                    end,
                    parent: Some(parent.clone()),
                });
                loan_by_ref.insert(dest.clone(), loan_index);
            }
            _ => {}
        }
    }

    if loans.is_empty() {
        return Ok(());
    }

    fn is_ancestor(
        loans: &[Loan],
        loan_by_ref: &HashMap<String, usize>,
        ancestor: &str,
        child: &Loan,
    ) -> bool {
        let mut cursor = child.parent.as_deref();
        while let Some(parent) = cursor {
            if parent == ancestor {
                return true;
            }
            cursor = loan_by_ref
                .get(parent)
                .and_then(|idx| loans.get(*idx))
                .and_then(|loan| loan.parent.as_deref());
        }
        false
    }

    for (i, left) in loans.iter().enumerate() {
        for right in loans.iter().skip(i + 1) {
            if left.origin != right.origin || left.end < right.start || right.end < left.start {
                continue;
            }
            let lineage = is_ancestor(&loans, &loan_by_ref, &left.reference, right)
                || is_ancestor(&loans, &loan_by_ref, &right.reference, left);
            if !lineage && (left.mutable || right.mutable) {
                return Err(format!(
                    "function '{}': conflicting {} borrow '{}' of '{}' while {} borrow '{}' is live",
                    function.name,
                    if right.mutable { "mutable" } else { "shared" },
                    right.reference,
                    right.origin,
                    if left.mutable { "mutable" } else { "shared" },
                    left.reference
                ));
            }
        }
    }

    for (index, instr) in instructions.iter().enumerate() {
        if let Instruction::Move { src, .. }
        | Instruction::Assign { src, .. }
        | Instruction::LinearMove { src, .. } = instr
        {
            if loan_by_ref.contains_key(src) {
                return Err(format!(
                    "function '{}': copying or moving safe reference '{}' is not yet qualified",
                    function.name, src
                ));
            }
        }
        if let Instruction::Return { value } = instr {
            if loan_by_ref.contains_key(value) {
                return Err(format!(
                    "function '{}': local reference '{}' cannot escape its function",
                    function.name, value
                ));
            }
        }
        if let Instruction::Spawn { args, .. } = instr {
            if let Some(reference) = args.iter().find(|arg| loan_by_ref.contains_key(*arg)) {
                return Err(format!(
                    "function '{}': spawning with safe reference '{}' is not qualified",
                    function.name, reference
                ));
            }
        }
        if let Instruction::Call { func, args, .. } = instr {
            let reference_args: Vec<&String> = args
                .iter()
                .filter(|arg| loan_by_ref.contains_key(*arg))
                .collect();
            if !reference_args.is_empty() {
                let Some(param_signature) = signatures.get(func) else {
                    return Err(format!(
                        "function '{}': unresolved call '{}' cannot receive safe references",
                        function.name, func
                    ));
                };
                let mut seen = HashSet::new();
                for (position, arg) in args.iter().enumerate() {
                    let Some(&loan_index) = loan_by_ref.get(arg) else {
                        if param_signature.get(position).copied().flatten().is_some() {
                            return Err(format!(
                                "function '{}': call '{}' reference parameter {} requires a safe reference argument",
                                function.name, func, position
                            ));
                        }
                        continue;
                    };
                    if !seen.insert(arg.clone()) {
                        return Err(format!(
                            "function '{}': passing the same safe reference '{}' more than once to '{}' is not yet qualified",
                            function.name, arg, func
                        ));
                    }
                    let Some(expected_mutable) = param_signature.get(position).copied().flatten()
                    else {
                        return Err(format!(
                            "function '{}': safe reference '{}' cannot be passed to non-reference parameter {} of '{}'",
                            function.name, arg, position, func
                        ));
                    };
                    let actual = &loans[loan_index];
                    if expected_mutable && !actual.mutable {
                        return Err(format!(
                            "function '{}': shared reference '{}' cannot satisfy mutable reference parameter {} of '{}'",
                            function.name, arg, position, func
                        ));
                    }
                }
            }
        }

        for loan in &loans {
            if index < loan.start || index > loan.end {
                continue;
            }
            // A parent permission is suspended while any child reborrow is live.
            let parent_suspended = loans.iter().any(|child| {
                child.start <= index
                    && index <= child.end
                    && is_ancestor(&loans, &loan_by_ref, &loan.reference, child)
            });
            if parent_suspended
                && (matches!(instr, Instruction::Deref { reference, .. } if reference == &loan.reference)
                    || matches!(instr, Instruction::DerefAssign { reference, .. } if reference == &loan.reference))
            {
                return Err(format!(
                    "function '{}': parent reference '{}' is suspended while a reborrow is live",
                    function.name, loan.reference
                ));
            }

            if matches!(instr, Instruction::Borrow { dest, .. } if dest == &loan.reference)
                || matches!(instr, Instruction::Reborrow { dest, .. } if dest == &loan.reference)
                || matches!(instr, Instruction::Deref { reference, .. } if reference == &loan.reference)
                || matches!(instr, Instruction::DerefAssign { reference, .. } if reference == &loan.reference)
            {
                continue;
            }

            let place = &loan.origin;
            let reads_owner = match instr {
                Instruction::Move { src, .. }
                | Instruction::Assign { src, .. }
                | Instruction::LinearMove { src, .. }
                | Instruction::Print { src }
                | Instruction::Return { value: src } => src == place,
                Instruction::BinaryOp { left, right, .. } => left == place || right == place,
                Instruction::UnaryOp { operand, .. } => operand == place,
                Instruction::Call { args, .. } | Instruction::Spawn { args, .. } => {
                    args.iter().any(|arg| arg == place)
                }
                Instruction::AggregateInit { fields, .. }
                | Instruction::EnumInit { fields, .. } => {
                    fields.iter().any(|(_, value)| value == place)
                }
                Instruction::FieldAccess { base, .. } => base == place,
                _ => false,
            };
            let writes_or_consumes_owner = match instr {
                Instruction::LinearMove { src, .. } => src == place,
                Instruction::DropLinear { var } | Instruction::Drop { var } => var == place,
                Instruction::Assign { dest, .. } => dest == place,
                Instruction::FieldAssign { base, .. } => base == place,
                _ => false,
            };
            let shared_linear_consume =
                !loan.mutable && linear_places.contains(place) && reads_owner;
            if (loan.mutable && (reads_owner || writes_or_consumes_owner))
                || (!loan.mutable && (writes_or_consumes_owner || shared_linear_consume))
            {
                return Err(format!(
                    "function '{}': place '{}' is {} while {} reference '{}' is live",
                    function.name,
                    place,
                    if loan.mutable {
                        "used or modified"
                    } else {
                        "modified or consumed"
                    },
                    if loan.mutable { "mutable" } else { "shared" },
                    loan.reference
                ));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinearPlaceState {
    Available,
    PartiallyMoved,
    Moved,
    MaybeMoved,
}

fn check_function_linear_places(function: &MirFunction) -> Result<(), String> {
    use std::collections::{HashMap, VecDeque};

    type StateMap = HashMap<String, LinearPlaceState>;

    fn merge_into(target: &mut StateMap, incoming: &StateMap) -> bool {
        let mut changed = false;
        let target_before = target.clone();
        let mut keys: std::collections::HashSet<String> = target_before.keys().cloned().collect();
        keys.extend(incoming.keys().cloned());

        for name in keys {
            let root = name.split('.').next().unwrap_or(name.as_str());
            let is_projection = name.contains('.');
            let left = target_before.get(&name).copied().or_else(|| {
                if is_projection && target_before.contains_key(root) {
                    Some(LinearPlaceState::Available)
                } else {
                    None
                }
            });
            let right = incoming.get(&name).copied().or_else(|| {
                if is_projection && incoming.contains_key(root) {
                    Some(LinearPlaceState::Available)
                } else {
                    None
                }
            });
            let joined = match (left, right) {
                (Some(l), Some(r)) if l == r => l,
                (Some(_), Some(_)) => LinearPlaceState::MaybeMoved,
                (Some(l), None) => l,
                (None, Some(r)) => r,
                (None, None) => continue,
            };
            if target.get(&name).copied() != Some(joined) {
                target.insert(name, joined);
                changed = true;
            }
        }
        changed
    }

    fn place_is_descendant(candidate: &str, base: &str) -> bool {
        candidate
            .strip_prefix(base)
            .is_some_and(|suffix| suffix.starts_with('.'))
    }

    fn ancestor_state(states: &StateMap, var: &str) -> Option<(String, LinearPlaceState)> {
        let mut cursor = var;
        while let Some(dot) = cursor.rfind('.') {
            cursor = &cursor[..dot];
            if let Some(state) = states.get(cursor) {
                return Some((cursor.to_string(), *state));
            }
        }
        None
    }

    fn moved_error(
        function_name: &str,
        var: &str,
        owner: &str,
        op: &str,
        state: LinearPlaceState,
    ) -> String {
        match state {
            LinearPlaceState::Moved => format!(
                "function '{}': use of moved linear value '{}' during {}",
                function_name, owner, op
            ),
            LinearPlaceState::PartiallyMoved => format!(
                "function '{}': linear value '{}' is partially moved during {}",
                function_name, owner, op
            ),
            LinearPlaceState::MaybeMoved => format!(
                "function '{}': linear value '{}' is only conditionally available during {}",
                function_name, owner, op
            ),
            LinearPlaceState::Available => format!(
                "function '{}': linear value '{}' unexpectedly blocked while using '{}' during {}",
                function_name, owner, var, op
            ),
        }
    }

    fn check_ancestors_available(
        states: &StateMap,
        function_name: &str,
        var: &str,
        op: &str,
    ) -> Result<(), String> {
        if let Some((owner, state @ (LinearPlaceState::Moved | LinearPlaceState::MaybeMoved))) =
            ancestor_state(states, var)
        {
            return Err(moved_error(function_name, var, owner.as_str(), op, state));
        }
        Ok(())
    }

    fn mark_place_moved(states: &mut StateMap, var: &str) {
        if let Some(state) = states.get_mut(var) {
            *state = LinearPlaceState::Moved;
        }
        let descendants: Vec<String> = states
            .keys()
            .filter(|candidate| place_is_descendant(candidate, var))
            .cloned()
            .collect();
        for descendant in descendants {
            states.insert(descendant, LinearPlaceState::Moved);
        }
    }

    fn consume(
        states: &mut StateMap,
        function_name: &str,
        var: &str,
        op: &str,
    ) -> Result<bool, String> {
        check_ancestors_available(states, function_name, var, op)?;
        if let Some(dot) = var.find('.') {
            let root = &var[..dot];
            match states.get(root).copied() {
                Some(LinearPlaceState::Moved | LinearPlaceState::MaybeMoved) => {
                    let state = states[root];
                    return Err(moved_error(function_name, var, root, op, state));
                }
                Some(LinearPlaceState::Available | LinearPlaceState::PartiallyMoved) => {
                    match states.get(var).copied() {
                        Some(LinearPlaceState::Moved | LinearPlaceState::PartiallyMoved) => {
                            return Err(moved_error(
                                function_name,
                                var,
                                var,
                                op,
                                LinearPlaceState::Moved,
                            ));
                        }
                        Some(LinearPlaceState::MaybeMoved) => {
                            return Err(moved_error(
                                function_name,
                                var,
                                var,
                                op,
                                LinearPlaceState::MaybeMoved,
                            ));
                        }
                        _ => {}
                    }
                    states.insert(var.to_string(), LinearPlaceState::Moved);
                    states.insert(root.to_string(), LinearPlaceState::PartiallyMoved);
                    return Ok(true);
                }
                None => {}
            }
        }
        match states.get(var).copied() {
            Some(LinearPlaceState::Available) => {
                mark_place_moved(states, var);
                Ok(true)
            }
            Some(
                state @ (LinearPlaceState::PartiallyMoved
                | LinearPlaceState::Moved
                | LinearPlaceState::MaybeMoved),
            ) => Err(moved_error(function_name, var, var, op, state)),
            None => Ok(false),
        }
    }

    fn observe(states: &StateMap, function_name: &str, var: &str, op: &str) -> Result<(), String> {
        check_ancestors_available(states, function_name, var, op)?;
        if let Some(dot) = var.find('.') {
            let root = &var[..dot];
            if let Some(state @ (LinearPlaceState::Moved | LinearPlaceState::MaybeMoved)) =
                states.get(root).copied()
            {
                return Err(moved_error(function_name, var, root, op, state));
            }
            if let Some(state @ (LinearPlaceState::Moved | LinearPlaceState::MaybeMoved)) =
                states.get(var).copied()
            {
                return Err(moved_error(function_name, var, var, op, state));
            }
            return Ok(());
        }
        match states.get(var).copied() {
            Some(
                state @ (LinearPlaceState::PartiallyMoved
                | LinearPlaceState::Moved
                | LinearPlaceState::MaybeMoved),
            ) => Err(moved_error(function_name, var, var, op, state)),
            _ => Ok(()),
        }
    }

    fn cleanup_drop_place(states: &mut StateMap, var: &str) {
        // Compiler-generated cleanup is guarded by the conceptual drop flag:
        // an already-moved place is a no-op, while any remaining initialized
        // portion of a partial/conditional place is consumed exactly once.
        mark_place_moved(states, var);
    }

    fn explicit_drop_linear(
        states: &mut StateMap,
        function_name: &str,
        var: &str,
    ) -> Result<(), String> {
        check_ancestors_available(states, function_name, var, "explicit linear drop")?;
        match states.get(var).copied() {
            Some(LinearPlaceState::Moved) => Err(format!(
                "function '{}': explicit double-drop of moved linear value '{}'",
                function_name, var
            )),
            Some(LinearPlaceState::MaybeMoved) => Err(format!(
                "function '{}': linear value '{}' is only conditionally initialized at explicit drop",
                function_name, var
            )),
            Some(LinearPlaceState::Available | LinearPlaceState::PartiallyMoved) => {
                mark_place_moved(states, var);
                Ok(())
            }
            None => Ok(()),
        }
    }

    fn transfer_instruction(
        states: &mut StateMap,
        function_name: &str,
        instruction: &Instruction,
    ) -> Result<(), String> {
        match instruction {
            Instruction::LinearMove { dest, src } => {
                let _src_was_linear = consume(states, function_name, src.as_str(), "linear move")?;
                match states.get(dest.as_str()).copied() {
                    None | Some(LinearPlaceState::Moved) => {
                        states.insert(dest.clone(), LinearPlaceState::Available);
                    }
                    Some(LinearPlaceState::Available) => {
                        return Err(format!(
                            "function '{}': linear destination '{}' is already initialized",
                            function_name, dest
                        ));
                    }
                    Some(LinearPlaceState::PartiallyMoved) => {
                        return Err(format!(
                            "function '{}': linear destination '{}' is only partially moved and cannot be overwritten",
                            function_name, dest
                        ));
                    }
                    Some(LinearPlaceState::MaybeMoved) => {
                        return Err(format!(
                            "function '{}': linear destination '{}' is only conditionally available for reinitialization",
                            function_name, dest
                        ));
                    }
                }
            }
            Instruction::Move { dest, src } | Instruction::Assign { dest, src } => {
                if consume(states, function_name, src.as_str(), "move")? {
                    states.insert(dest.clone(), LinearPlaceState::Available);
                }
            }
            Instruction::Reborrow { parent, .. } => {
                observe(states, function_name, parent.as_str(), "reborrow")?;
            }
            Instruction::Print { src } => {
                consume(states, function_name, src.as_str(), "print")?;
            }
            Instruction::Return { value } => {
                consume(states, function_name, value.as_str(), "return")?;
            }
            Instruction::DropLinear { var } => {
                explicit_drop_linear(states, function_name, var.as_str())?;
            }
            Instruction::Drop { var } => {
                cleanup_drop_place(states, var.as_str());
            }
            Instruction::BinaryOp { left, right, .. } => {
                consume(states, function_name, left.as_str(), "binary operation")?;
                consume(states, function_name, right.as_str(), "binary operation")?;
            }
            Instruction::UnaryOp { operand, .. } => {
                consume(states, function_name, operand.as_str(), "unary operation")?;
            }
            Instruction::JumpIf { cond, .. } => {
                observe(states, function_name, cond.as_str(), "conditional branch")?;
            }
            Instruction::Spawn { args, .. } | Instruction::Call { args, .. } => {
                for arg in args {
                    consume(states, function_name, arg.as_str(), "call")?;
                }
            }
            Instruction::AggregateInit { fields, .. } | Instruction::EnumInit { fields, .. } => {
                for (_, value) in fields {
                    consume(
                        states,
                        function_name,
                        value.as_str(),
                        "aggregate initialization",
                    )?;
                }
            }
            Instruction::FieldAssign { base, field, src } => {
                observe(
                    states,
                    function_name,
                    src.as_str(),
                    "field reinitialization value",
                )?;
                let place = format!("{}.{}", base, field);
                let root = base.as_str();
                if states.contains_key(root) {
                    match states.get(&place).copied() {
                        Some(LinearPlaceState::Moved) => {
                            states.insert(place.clone(), LinearPlaceState::Available);
                            let any_missing = states.iter().any(|(name, state)| {
                                name.starts_with(&format!("{}.", root))
                                    && matches!(
                                        state,
                                        LinearPlaceState::Moved | LinearPlaceState::MaybeMoved
                                    )
                            });
                            states.insert(
                                root.to_string(),
                                if any_missing {
                                    LinearPlaceState::PartiallyMoved
                                } else {
                                    LinearPlaceState::Available
                                },
                            );
                        }
                        Some(LinearPlaceState::MaybeMoved) => {
                            return Err(format!("function '{}': linear field '{}' is only conditionally moved and cannot be reinitialized", function_name, place));
                        }
                        _ => {
                            return Err(format!("function '{}': linear field '{}' is still initialized; live-field mutation is not qualified", function_name, place));
                        }
                    }
                }
            }
            Instruction::FieldAccess {
                dest,
                base,
                field,
                linear,
            } => {
                if *linear {
                    let place = format!("{}.{}", base, field);
                    if consume(states, function_name, &place, "linear field move")? {
                        states.insert(dest.clone(), LinearPlaceState::Available);
                    }
                } else {
                    observe(states, function_name, base.as_str(), "place projection")?;
                }
            }
            Instruction::EnumTag { base, .. }
            | Instruction::EnumPayloadAccess { base, .. }
            | Instruction::StructAccess { base, .. } => {
                observe(states, function_name, base.as_str(), "place projection")?;
            }
            Instruction::IndexAccess { base, index, .. } => {
                observe(states, function_name, base.as_str(), "index projection")?;
                observe(states, function_name, index.as_str(), "index projection")?;
            }
            Instruction::SliceAccess {
                base, start, end, ..
            } => {
                observe(states, function_name, base.as_str(), "slice projection")?;
                observe(states, function_name, start.as_str(), "slice projection")?;
                observe(states, function_name, end.as_str(), "slice projection")?;
            }
            Instruction::Borrow { place, mutable, .. } => {
                observe(
                    states,
                    function_name,
                    place.as_str(),
                    if *mutable {
                        "mutable borrow"
                    } else {
                        "shared borrow"
                    },
                )?;
            }
            Instruction::Deref { .. } => {
                // The dedicated loan checker below validates reference provenance.
            }
            Instruction::DerefAssign { src, .. } => {
                observe(
                    states,
                    function_name,
                    src.as_str(),
                    "dereference assignment",
                )?;
                // The dedicated loan checker validates reference provenance and mutability.
            }
            Instruction::ConstInt { .. }
            | Instruction::ConstStr { .. }
            | Instruction::ConstBytes { .. }
            | Instruction::ConstBool { .. }
            | Instruction::Jump { .. }
            | Instruction::Label { .. }
            | Instruction::Channel { .. }
            | Instruction::StructDef { .. }
            | Instruction::EnumDef { .. }
            | Instruction::MatchBranch { .. } => {}
        }
        Ok(())
    }

    fn block_successors(function: &MirFunction, block_index: usize) -> Vec<usize> {
        let Some(block) = function.blocks.get(block_index) else {
            return Vec::new();
        };
        let mut successors = Vec::new();
        let last = block.instrs.last();
        match last {
            Some(Instruction::Jump { target }) => {
                if let Some(index) = function.blocks.iter().position(|b| b.id == *target) {
                    successors.push(index);
                }
            }
            Some(Instruction::JumpIf { target, .. }) => {
                if let Some(index) = function.blocks.iter().position(|b| b.id == *target) {
                    successors.push(index);
                }
                if block_index + 1 < function.blocks.len() {
                    successors.push(block_index + 1);
                }
            }
            Some(Instruction::MatchBranch {
                then_block,
                else_block,
                ..
            }) => {
                for target in [then_block, else_block] {
                    if let Some(index) = function.blocks.iter().position(|b| b.id == *target) {
                        successors.push(index);
                    }
                }
            }
            Some(Instruction::Return { .. }) => {}
            _ => {
                if block_index + 1 < function.blocks.len() {
                    successors.push(block_index + 1);
                }
            }
        }
        successors.sort_unstable();
        successors.dedup();
        successors
    }

    if function.blocks.is_empty() {
        return Ok(());
    }

    let mut entries: Vec<Option<StateMap>> = vec![None; function.blocks.len()];
    entries[0] = Some(StateMap::new());
    let mut worklist = VecDeque::from([0usize]);
    let mut exits = Vec::new();
    let mut iterations = 0usize;
    let iteration_limit = function.blocks.len().saturating_mul(64).max(64);

    while let Some(block_index) = worklist.pop_front() {
        iterations += 1;
        if iterations > iteration_limit {
            return Err(format!(
                "function '{}': ownership analysis did not converge after {} iterations",
                function.name, iteration_limit
            ));
        }
        let Some(block) = function.blocks.get(block_index) else {
            continue;
        };
        let mut state = entries[block_index].clone().unwrap_or_default();
        for instruction in &block.instrs {
            transfer_instruction(&mut state, &function.name, instruction)?;
        }
        let successors = block_successors(function, block_index);
        if successors.is_empty() {
            exits.push(state);
        } else {
            for successor in successors {
                let changed = if let Some(existing) = entries[successor].as_mut() {
                    merge_into(existing, &state)
                } else {
                    entries[successor] = Some(state.clone());
                    true
                };
                if changed && !worklist.contains(&successor) {
                    worklist.push_back(successor);
                }
            }
        }
    }

    let mut exit_state = StateMap::new();
    for state in exits {
        merge_into(&mut exit_state, &state);
    }
    let still_available: Vec<_> = exit_state
        .iter()
        .filter_map(|(name, state)| match state {
            LinearPlaceState::Available => Some(format!("{name} (not consumed)")),
            LinearPlaceState::PartiallyMoved => Some(format!("{name} (partially consumed)")),
            LinearPlaceState::MaybeMoved => Some(format!("{name} (not consumed on every path)")),
            LinearPlaceState::Moved => None,
        })
        .collect();
    if !still_available.is_empty() {
        return Err(format!(
            "function '{}': linear value(s) not fully consumed before function exit: {}",
            function.name,
            still_available.join(", ")
        ));
    }

    Ok(())
}

pub fn generate_cfg_regions(module: &MirModule) -> Vec<RegionInfo> {
    let mut regions = Vec::new();

    for f in &module.functions {
        if f.is_safe_wrapper {
            continue;
        }
        // Function root region spans the concrete MIR extent; do not use
        // usize::MAX as a pseudo-position because region bounds are part of
        // diagnostics and analysis output.
        let func_root = format!("{}_root", f.name);
        let end_block = f.blocks.len().saturating_sub(1);
        let end_instr = f
            .blocks
            .last()
            .map(|block| block.instrs.len().saturating_sub(1))
            .unwrap_or(0);
        regions.push(RegionInfo::new(func_root, 0, 0, end_block, end_instr).mark_universal());
        // region_counter removed

        // Track all blocks for CFG region generation
        // block_ids removed

        for (block_idx, block) in f.blocks.iter().enumerate() {
            // Block entry region
            let block_entry_region = format!("{}_b{}_entry", f.name, block.id);
            regions.push(
                RegionInfo::new(block_entry_region.clone(), block_idx, 0, block_idx, 0)
                    .with_parent(format!("{}_root", f.name)),
            );
            // region_counter removed

            // Create regions for each instruction that introduces a new scope
            for (instr_idx, instr) in block.instrs.iter().enumerate() {
                match instr {
                    Instruction::Jump { target } => {
                        // Region from jump point to target block
                        let jump_region = format!("{}_b{}_jump_{}", f.name, block.id, instr_idx);
                        regions.push(
                            RegionInfo::new(jump_region, block_idx, instr_idx, *target, 0)
                                .with_parent(format!("{}_root", f.name)),
                        );
                        // region_counter removed
                    }
                    Instruction::JumpIf { cond: _, target } => {
                        // Region for each branch
                        let then_region = format!("{}_b{}_then_{}", f.name, block.id, instr_idx);
                        regions.push(
                            RegionInfo::new(then_region, block_idx, instr_idx, *target, 0)
                                .with_parent(format!("{}_root", f.name)),
                        );
                        // region_counter removed

                        // Also create else region (fallthrough)
                        let else_region = format!("{}_b{}_else_{}", f.name, block.id, instr_idx);
                        let next_block = block_idx + 1;
                        regions.push(
                            RegionInfo::new(else_region, block_idx, instr_idx, next_block, 0)
                                .with_parent(format!("{}_root", f.name)),
                        );
                        // region_counter removed
                    }
                    Instruction::Call { dest: _, func, .. } => {
                        // Call region - represents the loan region for function call
                        let call_region = format!("{}_call_{}_{}", f.name, func, instr_idx);
                        regions.push(
                            RegionInfo::new(
                                call_region,
                                block_idx,
                                instr_idx,
                                block_idx,
                                instr_idx,
                            )
                            .with_lifetime(block_idx, block_idx)
                            .with_parent(format!("{}_root", f.name)),
                        );
                        // region_counter removed
                    }
                    Instruction::FieldAccess {
                        dest: _,
                        base,
                        field,
                        ..
                    } => {
                        // Field access creates a loan region for the field
                        let field_region =
                            format!("{}_field_{}_{}_{}", f.name, base, field, instr_idx);
                        regions.push(
                            RegionInfo::new(
                                field_region,
                                block_idx,
                                instr_idx,
                                block_idx,
                                instr_idx,
                            )
                            .with_parent(format!("{}_root", f.name)),
                        );
                        // region_counter removed
                    }
                    Instruction::StructAccess {
                        dest: _,
                        base,
                        field,
                    } => {
                        // Struct field access region
                        let struct_region =
                            format!("{}_struct_{}_{}_{}", f.name, base, field, instr_idx);
                        regions.push(
                            RegionInfo::new(
                                struct_region,
                                block_idx,
                                instr_idx,
                                block_idx,
                                instr_idx,
                            )
                            .with_parent(format!("{}_root", f.name)),
                        );
                        // region_counter removed
                    }
                    _ => {}
                }
            }

            // Block exit region
            let last_idx = if block.instrs.is_empty() {
                0
            } else {
                block.instrs.len() - 1
            };
            let block_exit_region = format!("{}_b{}_exit", f.name, block.id);
            regions.push(
                RegionInfo::new(block_exit_region, block_idx, last_idx, block_idx, last_idx)
                    .with_parent(format!("{}_root", f.name)),
            );
            // region_counter removed
        }
    }

    regions
}

pub fn generate_loan_facts(module: &MirModule) -> Vec<LoanInfo> {
    let mut loans = Vec::new();
    let mut loan_counter: usize = 0;

    for f in &module.functions {
        let regions = generate_cfg_regions(module);

        for (block_idx, block) in f.blocks.iter().enumerate() {
            for (instr_idx, instr) in block.instrs.iter().enumerate() {
                match instr {
                    Instruction::FieldAccess { dest, base, .. } => {
                        let loan_name = format!("loan_{}", loan_counter);
                        let region = find_containing_region(&regions, block_idx, instr_idx)
                            .unwrap_or_else(|| format!("{}_root", f.name));
                        loans.push(LoanInfo {
                            name: loan_name,
                            region: region.clone(),
                            borrower: dest.clone(),
                            kind: LoanKind::Shared,
                        });
                        // Also create a loan for the base if it's a reference
                        let base_loan = format!("loan_{}_base", loan_counter);
                        loans.push(LoanInfo {
                            name: base_loan,
                            region,
                            borrower: base.clone(),
                            kind: LoanKind::Shared,
                        });
                        loan_counter += 1;
                    }
                    Instruction::StructAccess {
                        dest,
                        base: _,
                        field: _,
                    } => {
                        let loan_name = format!("loan_{}", loan_counter);
                        let region = find_containing_region(&regions, block_idx, instr_idx)
                            .unwrap_or_else(|| format!("{}_root", f.name));
                        loans.push(LoanInfo {
                            name: loan_name,
                            region: region.clone(),
                            borrower: dest.clone(),
                            kind: LoanKind::Mutable,
                        });
                        loan_counter += 1;
                    }
                    Instruction::IndexAccess {
                        dest,
                        base: _,
                        index: _,
                    } => {
                        let loan_name = format!("loan_{}", loan_counter);
                        let region = find_containing_region(&regions, block_idx, instr_idx)
                            .unwrap_or_else(|| format!("{}_root", f.name));
                        loans.push(LoanInfo {
                            name: loan_name,
                            region: region.clone(),
                            borrower: dest.clone(),
                            kind: LoanKind::Shared,
                        });
                        loan_counter += 1;
                    }
                    Instruction::Move { dest, src: _ } => {
                        // Track move as a loan transfer
                        let loan_name = format!("loan_{}", loan_counter);
                        let region = find_containing_region(&regions, block_idx, instr_idx)
                            .unwrap_or_else(|| format!("{}_root", f.name));
                        loans.push(LoanInfo {
                            name: loan_name,
                            region,
                            borrower: dest.clone(),
                            kind: LoanKind::Shared,
                        });
                        loan_counter += 1;
                    }
                    Instruction::LinearMove { dest, src: _ } => {
                        // Linear moves are consuming loans
                        let loan_name = format!("loan_{}", loan_counter);
                        let region = find_containing_region(&regions, block_idx, instr_idx)
                            .unwrap_or_else(|| format!("{}_root", f.name));
                        loans.push(LoanInfo {
                            name: loan_name,
                            region,
                            borrower: dest.clone(),
                            kind: LoanKind::Mutable,
                        });
                        loan_counter += 1;
                    }
                    Instruction::Call {
                        dest: _,
                        func: _,
                        args,
                    } => {
                        // Function calls create loans for each argument
                        for (i, arg) in args.iter().enumerate() {
                            let loan_name = format!("loan_{}_arg_{}", loan_counter, i);
                            let region = find_containing_region(&regions, block_idx, instr_idx)
                                .unwrap_or_else(|| format!("{}_root", f.name));
                            loans.push(LoanInfo {
                                name: loan_name,
                                region,
                                borrower: arg.clone(),
                                kind: LoanKind::Shared,
                            });
                        }
                        loan_counter += 1;
                    }
                    _ => {}
                }
            }
        }
    }

    loans
}

fn find_containing_region(
    regions: &[RegionInfo],
    block_idx: usize,
    instr_idx: usize,
) -> Option<String> {
    regions
        .iter()
        .find(|r| r.contains_point(block_idx, instr_idx))
        .map(|r| r.name.clone())
}

pub fn export_polonius_with_regions_and_loans(module: &MirModule) -> String {
    let mut out = export_polonius_input_with_region_facts(module);

    out.push_str("\n# Regions\n");
    for region in generate_cfg_regions(module) {
        out.push_str(&format!(
            "region {} {}:{} -> {}:{}\n",
            region.name, region.start_block, region.start_instr, region.end_block, region.end_instr
        ));
    }

    out.push_str("\n# Loans\n");
    for loan in generate_loan_facts(module) {
        out.push_str(&format!(
            "loan {} in {} {} {:?}\n",
            loan.name, loan.region, loan.borrower, loan.kind
        ));
    }

    out
}
