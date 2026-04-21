use crate::mir::{Instruction, MirModule};
use std::collections::HashMap;

pub struct RegionInfo {
    pub name: String,
    pub start_block: usize,
    pub start_instr: usize,
    pub end_block: usize,
    pub end_instr: usize,
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
                    Instruction::ConstBool { dest, value } => {
                        out.push_str(&format!("  {}: const_bool {}\n", dest, value))
                    }
                    Instruction::Move { dest, src } => {
                        out.push_str(&format!("  {}: move {}\n", dest, src))
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
                    Instruction::FieldAccess { dest, base, field } => {
                        out.push_str(&format!("  {}: field_access {}.{}\n", dest, base, field))
                    }
                    Instruction::StructAccess { dest, base, field } => {
                        out.push_str(&format!("  {}: struct_access {}.{}\n", dest, base, field))
                    }
                    Instruction::IndexAccess { dest, base, index } => {
                        out.push_str(&format!("  {}: index_access {}[{}]\n", dest, base, index))
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
        for b in &f.blocks {
            for (i, instr) in b.instrs.iter().enumerate() {
                facts.push(format!("point {} {} {}", f.name, b.id, i));
                match instr {
                    Instruction::ConstInt { dest, .. }
                    | Instruction::ConstStr { dest, .. }
                    | Instruction::ConstBool { dest, .. } => {
                        facts.push(format!("def {} {} {} {}", f.name, b.id, i, dest));
                    }
                    Instruction::Move { dest, src } => {
                        facts.push(format!("move {} {} {} {} {}", f.name, b.id, i, src, dest));
                    }
                    Instruction::LinearMove { dest, src } => {
                        facts.push(format!(
                            "linear_move {} {} {} {} {}",
                            f.name, b.id, i, src, dest
                        ));
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
                    Instruction::Call { dest, func, args } => {
                        facts.push(format!("def {} {} {} {}", f.name, b.id, i, dest));
                        facts.push(format!(
                            "call {} {} {} {} ({})",
                            f.name,
                            b.id,
                            i,
                            func,
                            args.join(",")
                        ));
                        for a in args {
                            facts.push(format!("use {} {} {} {}", f.name, b.id, i, a));
                        }
                    }
                    Instruction::FieldAccess { dest, base, field } => {
                        facts.push(format!("def {} {} {} {}", f.name, b.id, i, dest));
                        facts.push(format!("use {} {} {} {}", f.name, b.id, i, base));
                        facts.push(format!(
                            "field {} {} {} {}.{}",
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
                        facts.push(format!(
                            "index {} {} {} {}[{}]",
                            f.name, b.id, i, base, index
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
                    Instruction::Call { dest, args, .. } => {
                        defs.entry(dest.clone()).or_default().push(i);
                        for a in args {
                            uses.entry(a.clone()).or_default().push(i);
                        }
                    }
                    Instruction::FieldAccess { dest, base, .. }
                    | Instruction::StructAccess { dest, base, .. }
                    | Instruction::IndexAccess { dest, base, .. } => {
                        defs.entry(dest.clone()).or_default().push(i);
                        uses.entry(base.clone()).or_default().push(i);
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
                    | Instruction::EnumDef { .. } => {}
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
                    Instruction::FieldAccess { dest, base, .. }
                    | Instruction::StructAccess { dest, base, .. }
                    | Instruction::IndexAccess { dest, base, .. } => {
                        func_defs
                            .entry(dest.clone())
                            .or_default()
                            .push((block_idx, i));
                        func_uses
                            .entry(base.clone())
                            .or_default()
                            .push((block_idx, i));
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
                    _ => {}
                }
            }
        }

        // Sort def/use positions and emit live facts for uses that fall between
        // a definition and the next definition (possibly in a later block).
        for (_var, defs) in func_defs.iter_mut() {
            defs.sort_unstable();
        }
        for (var, def_positions) in func_defs.iter() {
            if let Some(all_uses) = func_uses.get(var) {
                let mut uses_sorted = all_uses.clone();
                uses_sorted.sort_unstable();
                for (idx, def_pos) in def_positions.iter().enumerate() {
                    let next_def = def_positions
                        .get(idx + 1)
                        .copied()
                        .unwrap_or((usize::MAX, usize::MAX));
                    for &use_pos in uses_sorted.iter() {
                        // use_pos >= def_pos && use_pos < next_def
                        let ge = (use_pos.0 > def_pos.0)
                            || (use_pos.0 == def_pos.0 && use_pos.1 >= def_pos.1);
                        let lt = (use_pos.0 < next_def.0)
                            || (use_pos.0 == next_def.0 && use_pos.1 < next_def.1);
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

pub fn run_polonius_adapter(module: &MirModule) -> Result<(), String> {
    // Adapter expects the textual export format produced by `export_polonius_input`.
    let facts = build_polonius_facts(module).join("\n");
    polonius_engine_adapter::check_facts(&facts)
}

pub fn check_mir(module: &MirModule) -> Result<(), String> {
    run_polonius_adapter(module)
}

pub fn generate_cfg_regions(module: &MirModule) -> Vec<RegionInfo> {
    let mut regions = Vec::new();
    let mut _region_counter: usize = 0;

    for f in &module.functions {
        for (block_idx, block) in f.blocks.iter().enumerate() {
            let block_entry_region = format!("{}_b{}_entry", f.name, block.id);
            regions.push(RegionInfo {
                name: block_entry_region.clone(),
                start_block: block_idx,
                start_instr: 0,
                end_block: block_idx,
                end_instr: block.instrs.len().saturating_sub(1),
            });
            _region_counter += 1;

            for (instr_idx, instr) in block.instrs.iter().enumerate() {
                match instr {
                    Instruction::Jump { target } => {
                        let exit_region = format!("{}_b{}_exit", f.name, block.id);
                        regions.push(RegionInfo {
                            name: exit_region,
                            start_block: block_idx,
                            start_instr: instr_idx,
                            end_block: *target,
                            end_instr: 0,
                        });
                        _region_counter += 1;
                    }
                    Instruction::JumpIf { cond: _, target } => {
                        let exit_region = format!("{}_b{}_exit", f.name, block.id);
                        regions.push(RegionInfo {
                            name: exit_region,
                            start_block: block_idx,
                            start_instr: instr_idx,
                            end_block: *target,
                            end_instr: 0,
                        });
                        _region_counter += 1;
                    }
                    Instruction::Call { dest: _, .. } => {
                        let call_region = format!("{}_call_{}", f.name, instr_idx);
                        regions.push(RegionInfo {
                            name: call_region,
                            start_block: block_idx,
                            start_instr: instr_idx,
                            end_block: block_idx,
                            end_instr: instr_idx,
                        });
                        _region_counter += 1;
                    }
                    _ => {}
                }
            }

            let last_idx = if block.instrs.is_empty() {
                0
            } else {
                block.instrs.len() - 1
            };
            regions.push(RegionInfo {
                name: format!("{}_b{}_exit", f.name, block.id),
                start_block: block_idx,
                start_instr: last_idx,
                end_block: block_idx,
                end_instr: last_idx,
            });
        }

        let func_root = format!("{}_root", f.name);
        regions.push(RegionInfo {
            name: func_root,
            start_block: 0,
            start_instr: 0,
            end_block: f.blocks.len().saturating_sub(1),
            end_instr: usize::MAX,
        });
    }

    regions
}

pub fn generate_loan_facts(module: &MirModule) -> Vec<LoanInfo> {
    let mut loans = Vec::new();
    let mut loan_counter: usize = 0;

    for f in &module.functions {
        let regions = generate_cfg_regions(module);

        for (block_idx, block) in f.blocks.iter().enumerate() {
            for instr in block.instrs.iter() {
                match instr {
                    Instruction::FieldAccess { dest, .. } => {
                        let loan_name = format!("loan_{}", loan_counter);
                        let region = regions
                            .iter()
                            .find(|r| r.start_block <= block_idx && block_idx <= r.end_block)
                            .map(|r| r.name.clone())
                            .unwrap_or_else(|| format!("{}_root", f.name));
                        loans.push(LoanInfo {
                            name: loan_name,
                            region,
                            borrower: dest.clone(),
                            kind: LoanKind::Shared,
                        });
                        loan_counter += 1;
                    }
                    Instruction::StructAccess { dest, .. } => {
                        let loan_name = format!("loan_{}", loan_counter);
                        let region = regions
                            .iter()
                            .find(|r| r.start_block <= block_idx && block_idx <= r.end_block)
                            .map(|r| r.name.clone())
                            .unwrap_or_else(|| format!("{}_root", f.name));
                        loans.push(LoanInfo {
                            name: loan_name,
                            region,
                            borrower: dest.clone(),
                            kind: LoanKind::Mutable,
                        });
                        loan_counter += 1;
                    }
                    Instruction::IndexAccess { dest, .. } => {
                        let loan_name = format!("loan_{}", loan_counter);
                        let region = regions
                            .iter()
                            .find(|r| r.start_block <= block_idx && block_idx <= r.end_block)
                            .map(|r| r.name.clone())
                            .unwrap_or_else(|| format!("{}_root", f.name));
                        loans.push(LoanInfo {
                            name: loan_name,
                            region,
                            borrower: dest.clone(),
                            kind: LoanKind::Shared,
                        });
                        loan_counter += 1;
                    }
                    _ => {}
                }
            }
        }
    }

    loans
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
