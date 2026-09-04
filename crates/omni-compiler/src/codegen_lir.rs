use crate::mir;
use lir::{Function as LirFunction, Instr as LirInstr, Module as LirModule, Type as LirType};
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct AggregateStorage {
    type_name: String,
    /// Frame-local base cell for an owned local value.
    base_slot: Option<u32>,
    /// Incoming pointer slot for an indirectly-passed ABI value.
    indirect_slot: Option<u32>,
    fields: Vec<String>,
}

impl AggregateStorage {
    fn offset_of(&self, field: &str) -> Option<i64> {
        self.fields
            .iter()
            .position(|candidate| candidate == field)
            .map(|index| (index as i64) * 8)
    }
}

#[derive(Debug, Clone)]
enum ValueAbi {
    Unit,
    Scalar,
    Reference {
        mutable: bool,
    },
    Indirect {
        type_name: String,
        fields: Vec<String>,
    },
}

impl ValueAbi {
    fn lir_param_type(&self) -> Result<LirType, String> {
        match self {
            ValueAbi::Scalar => Ok(LirType::I64),
            ValueAbi::Reference { .. } => Ok(LirType::Ptr(1)),
            ValueAbi::Indirect { fields, .. } => {
                let cells = u32::try_from(fields.len())
                    .map_err(|_| "indirect ABI value exceeds cell-count limit".to_string())?;
                if cells == 0 {
                    return Err("zero-sized indirect ABI values are not yet qualified".to_string());
                }
                Ok(LirType::Ptr(cells))
            }
            ValueAbi::Unit => Err("unit is not a parameter value class".to_string()),
        }
    }
}

#[derive(Debug, Clone)]
struct FunctionAbi {
    params: Vec<ValueAbi>,
    ret: ValueAbi,
}

fn collect_struct_layouts(
    m: &mir::MirModule,
) -> Result<HashMap<String, Vec<(String, String)>>, String> {
    let mut layouts = HashMap::new();
    for func in &m.functions {
        for block in &func.blocks {
            for instr in &block.instrs {
                if let mir::Instruction::StructDef { name, fields, .. } = instr {
                    match layouts.get(name) {
                        Some(existing) if existing != fields => {
                            return Err(format!("conflicting MIR struct definitions for '{name}'"));
                        }
                        Some(_) => {}
                        None => {
                            layouts.insert(name.clone(), fields.clone());
                        }
                    }
                }
            }
        }
    }
    Ok(layouts)
}

fn collect_enum_layouts(
    m: &mir::MirModule,
) -> Result<HashMap<String, Vec<crate::ast::EnumVariant>>, String> {
    let mut layouts = HashMap::new();
    for func in &m.functions {
        for block in &func.blocks {
            for instr in &block.instrs {
                if let mir::Instruction::EnumDef { name, variants } = instr {
                    match layouts.get(name) {
                        Some(existing) if existing != variants => {
                            return Err(format!("conflicting MIR enum definitions for '{name}'"));
                        }
                        Some(_) => {}
                        None => {
                            layouts.insert(name.clone(), variants.clone());
                        }
                    }
                }
            }
        }
    }
    Ok(layouts)
}

fn validate_enum_initializer(
    type_name: &str,
    variant_name: &str,
    tag: u32,
    supplied: &[(String, String)],
    enum_layouts: &HashMap<String, Vec<crate::ast::EnumVariant>>,
) -> Result<usize, String> {
    let variants = enum_layouts
        .get(type_name)
        .ok_or_else(|| format!("enum initializer references unknown enum '{type_name}'"))?;
    let tag_index = usize::try_from(tag).map_err(|_| "enum tag does not fit usize".to_string())?;
    let declared_variant = variants.get(tag_index).ok_or_else(|| {
        format!(
            "enum initializer for '{type_name}' uses invalid tag {tag}; {} variants declared",
            variants.len()
        )
    })?;
    if declared_variant.name != variant_name {
        return Err(format!(
            "enum initializer for '{type_name}' tag {tag} names '{}' but declaration names '{}'",
            variant_name, declared_variant.name
        ));
    }
    if supplied.len() != declared_variant.fields.len() {
        return Err(format!(
            "enum initializer for '{type_name}::{variant_name}' has {} payload fields, expected {}",
            supplied.len(),
            declared_variant.fields.len()
        ));
    }
    for ((supplied_name, _), (declared_name, field_type)) in
        supplied.iter().zip(declared_variant.fields.iter())
    {
        if supplied_name != declared_name {
            return Err(format!(
                "enum initializer for '{type_name}::{variant_name}' payload field '{}' is out of declaration order; expected '{}'",
                supplied_name, declared_name
            ));
        }
        match field_type.to_ascii_lowercase().as_str() {
            "int" | "i64" | "isize" | "bool" | "boolean" | "byte" | "u8" => {}
            other => {
                return Err(format!(
                    "enum '{type_name}::{variant_name}' payload type '{other}' does not yet have a qualified v0.1.4 scalar-cell layout"
                ))
            }
        }
    }
    Ok(variants
        .iter()
        .map(|variant| variant.fields.len())
        .max()
        .unwrap_or(0))
}

fn ordered_aggregate_fields(
    type_name: &str,
    supplied: &[(String, String)],
    struct_layouts: &HashMap<String, Vec<(String, String)>>,
) -> Result<Vec<String>, String> {
    if matches!(type_name, "Tuple" | "Array") {
        let expected: Vec<String> = (0..supplied.len()).map(|index| index.to_string()).collect();
        let actual: Vec<String> = supplied.iter().map(|(name, _)| name.clone()).collect();
        if actual != expected {
            return Err(format!(
                "tuple MIR fields must be canonical 0..N order, got {actual:?}"
            ));
        }
        return Ok(expected);
    }
    let declared = struct_layouts
        .get(type_name)
        .ok_or_else(|| format!("aggregate initializer references unknown struct '{type_name}'"))?;
    for (_, field_type) in declared {
        match field_type.to_ascii_lowercase().as_str() {
            "int" | "i64" | "isize" | "bool" | "boolean" | "byte" | "u8" => {}
            other => return Err(format!(
                "struct '{type_name}' field type '{other}' does not yet have a qualified v0.1.4 scalar-cell layout"
            )),
        }
    }
    let expected: Vec<String> = declared.iter().map(|(name, _)| name.clone()).collect();
    if supplied.len() != expected.len() {
        return Err(format!(
            "aggregate initializer for '{type_name}' has {} fields, expected {}",
            supplied.len(),
            expected.len()
        ));
    }
    for name in &expected {
        if !supplied.iter().any(|(candidate, _)| candidate == name) {
            return Err(format!(
                "aggregate initializer for '{type_name}' is missing field '{name}'"
            ));
        }
    }
    for (name, _) in supplied {
        if !expected.iter().any(|candidate| candidate == name) {
            return Err(format!(
                "aggregate initializer for '{type_name}' has unknown field '{name}'"
            ));
        }
    }
    Ok(expected)
}

fn scalar_cell_annotation(annotation: &str) -> bool {
    matches!(
        annotation.trim().to_ascii_lowercase().as_str(),
        "int" | "i64" | "isize" | "bool" | "boolean" | "byte" | "u8"
    )
}

fn abi_for_annotation(
    annotation: Option<&str>,
    struct_layouts: &HashMap<String, Vec<(String, String)>>,
    enum_layouts: &HashMap<String, Vec<crate::ast::EnumVariant>>,
) -> Result<ValueAbi, String> {
    let Some(annotation) = annotation else {
        // Preserve the historical unannotated Stage-0 scalar convention.
        return Ok(ValueAbi::Scalar);
    };
    let trimmed = annotation.trim();
    if scalar_cell_annotation(trimmed) {
        return Ok(ValueAbi::Scalar);
    }
    if let Some(inner) = trimmed.strip_prefix("&mut ") {
        if !scalar_cell_annotation(inner.trim()) {
            return Err(format!(
                "reference ABI currently requires a scalar-cell pointee; got '{inner}'"
            ));
        }
        return Ok(ValueAbi::Reference { mutable: true });
    }
    if let Some(inner) = trimmed.strip_prefix('&') {
        if !scalar_cell_annotation(inner.trim()) {
            return Err(format!(
                "reference ABI currently requires a scalar-cell pointee; got '{inner}'"
            ));
        }
        return Ok(ValueAbi::Reference { mutable: false });
    }
    if matches!(
        trimmed.to_ascii_lowercase().as_str(),
        "unit" | "void" | "()"
    ) {
        return Ok(ValueAbi::Unit);
    }
    if trimmed.eq_ignore_ascii_case("string") {
        return Ok(ValueAbi::Indirect {
            type_name: "String".to_string(),
            fields: vec!["@data".to_string(), "len".to_string()],
        });
    }
    if trimmed.eq_ignore_ascii_case("bytes") {
        return Ok(ValueAbi::Indirect {
            type_name: "Bytes".to_string(),
            fields: vec!["@data".to_string(), "len".to_string()],
        });
    }
    if let Some(fields) = struct_layouts.get(trimmed) {
        for (_, field_type) in fields {
            if !scalar_cell_annotation(field_type) {
                return Err(format!(
                    "aggregate ABI for struct '{trimmed}' requires scalar-cell fields in v0.1.4; field type '{field_type}' is not qualified"
                ));
            }
        }
        return Ok(ValueAbi::Indirect {
            type_name: trimmed.to_string(),
            fields: fields.iter().map(|(name, _)| name.clone()).collect(),
        });
    }
    if let Some(variants) = enum_layouts.get(trimmed) {
        for variant in variants {
            for (_, field_type) in &variant.fields {
                if !scalar_cell_annotation(field_type) {
                    return Err(format!(
                        "aggregate ABI for enum '{trimmed}' requires scalar-cell payloads in v0.1.4; payload type '{field_type}' is not qualified"
                    ));
                }
            }
        }
        let payload_width = variants
            .iter()
            .map(|variant| variant.fields.len())
            .max()
            .unwrap_or(0);
        let mut fields = Vec::with_capacity(payload_width + 1);
        fields.push("@tag".to_string());
        fields.extend((0..payload_width).map(|index| format!("@payload{index}")));
        return Ok(ValueAbi::Indirect {
            type_name: trimmed.to_string(),
            fields,
        });
    }
    Err(format!(
        "type '{trimmed}' does not yet have a qualified v0.1.4 native value ABI"
    ))
}

fn collect_function_abis(
    m: &mir::MirModule,
    struct_layouts: &HashMap<String, Vec<(String, String)>>,
    enum_layouts: &HashMap<String, Vec<crate::ast::EnumVariant>>,
) -> Result<HashMap<String, FunctionAbi>, String> {
    let mut result = HashMap::new();
    for func in &m.functions {
        if func.param_types.len() != func.params.len() {
            return Err(format!(
                "function '{}' MIR signature has {} parameter names but {} parameter types",
                func.name,
                func.params.len(),
                func.param_types.len()
            ));
        }
        let mut params = Vec::with_capacity(func.params.len());
        for annotation in &func.param_types {
            let abi = abi_for_annotation(annotation.as_deref(), struct_layouts, enum_layouts)?;
            if matches!(abi, ValueAbi::Unit) {
                return Err(format!(
                    "function '{}' cannot accept a unit/void parameter",
                    func.name
                ));
            }
            params.push(abi);
        }
        let ret = if func.returns_value {
            abi_for_annotation(func.return_type.as_deref(), struct_layouts, enum_layouts)?
        } else {
            ValueAbi::Unit
        };
        if matches!(ret, ValueAbi::Reference { .. }) {
            return Err(format!(
                "function '{}': returning safe references is not yet qualified; reference returns require outlives proof",
                func.name
            ));
        }
        if func.returns_value && matches!(ret, ValueAbi::Unit) {
            return Err(format!(
                "function '{}' is marked value-returning but declares unit/void",
                func.name
            ));
        }
        if result
            .insert(func.name.clone(), FunctionAbi { params, ret })
            .is_some()
        {
            return Err(format!("duplicate MIR function name '{}'", func.name));
        }
    }
    Ok(result)
}

fn allocate_local_aggregate(
    next_slot: &mut u32,
    type_name: String,
    fields: Vec<String>,
    function_name: &str,
) -> Result<AggregateStorage, String> {
    let width: u32 = fields
        .len()
        .try_into()
        .map_err(|_| format!("function '{function_name}': aggregate '{type_name}' is too large"))?;
    let base_slot = if width == 0 {
        None
    } else {
        let base = next_slot
            .checked_add(width - 1)
            .ok_or_else(|| format!("function '{function_name}': local slot space overflow"))?;
        *next_slot = next_slot
            .checked_add(width)
            .ok_or_else(|| format!("function '{function_name}': local slot space overflow"))?;
        Some(base)
    };
    Ok(AggregateStorage {
        type_name,
        base_slot,
        indirect_slot: None,
        fields,
    })
}

fn emit_aggregate_cell_load(
    out: &mut Vec<LirInstr>,
    storage: &AggregateStorage,
    offset: i64,
    function_name: &str,
) -> Result<(), String> {
    if let Some(base) = storage.base_slot {
        out.push(LirInstr::LoadOffset(base, offset));
        return Ok(());
    }
    if let Some(ptr_slot) = storage.indirect_slot {
        out.push(LirInstr::LoadPtrOffset(ptr_slot, offset));
        return Ok(());
    }
    Err(format!(
        "function '{function_name}': zero-sized aggregate '{}' has no readable storage",
        storage.type_name
    ))
}

fn emit_aggregate_pointer(
    out: &mut Vec<LirInstr>,
    storage: &AggregateStorage,
    function_name: &str,
) -> Result<(), String> {
    if let Some(base) = storage.base_slot {
        out.push(LirInstr::GetAddr(base));
        return Ok(());
    }
    if let Some(ptr_slot) = storage.indirect_slot {
        out.push(LirInstr::Load(ptr_slot));
        return Ok(());
    }
    Err(format!(
        "function '{function_name}': zero-sized aggregate '{}' cannot be passed indirectly",
        storage.type_name
    ))
}

fn constant_slice_window(
    start: i64,
    end: i64,
    inclusive: bool,
    source_len: usize,
) -> Result<(usize, usize), String> {
    if start < 0 || end < 0 {
        return Err("slice range bounds must be non-negative".to_string());
    }
    let start = usize::try_from(start).map_err(|_| "slice start does not fit usize".to_string())?;
    let raw_end = usize::try_from(end).map_err(|_| "slice end does not fit usize".to_string())?;
    let end_exclusive = if inclusive {
        raw_end
            .checked_add(1)
            .ok_or_else(|| "inclusive slice end overflow".to_string())?
    } else {
        raw_end
    };
    if start > end_exclusive || end_exclusive > source_len {
        return Err(format!(
            "slice range {start}..{end_exclusive} out of bounds for length {source_len}"
        ));
    }
    Ok((start, end_exclusive))
}

/// Lower verified MIR to the current scalar stack-based LIR.
///
/// The v0.1.4 baseline is intentionally fail-closed: constructs whose native
/// representation is not yet qualified return an error instead of fabricating
/// slot zero, zero-valued operands, NOP semantics, or dangling jump targets.
pub fn lower_mir_to_lir(m: &mir::MirModule) -> Result<LirModule, String> {
    let mut out = LirModule::new();
    let struct_layouts = collect_struct_layouts(m)?;
    let enum_layouts = collect_enum_layouts(m)?;
    let function_abis = collect_function_abis(m, &struct_layouts, &enum_layouts)?;

    for func in &m.functions {
        let func_abi = function_abis
            .get(&func.name)
            .ok_or_else(|| format!("missing ABI metadata for function '{}'", func.name))?;
        let mut var_slots: HashMap<String, u32> = HashMap::new();
        let mut aggregate_slots: HashMap<String, AggregateStorage> = HashMap::new();
        let mut incoming_reference_slots: HashMap<String, u32> = HashMap::new();
        let mut incoming_reference_mutability: HashMap<String, bool> = HashMap::new();
        let mut lir_param_types: Vec<LirType> = Vec::new();

        // Indirect returns use a caller-owned destination pointer as the first
        // machine parameter. This avoids returning pointers into a callee frame.
        let sret_slot = match &func_abi.ret {
            ValueAbi::Indirect { fields, .. } => {
                let cells: u32 = fields.len().try_into().map_err(|_| {
                    format!(
                        "function '{}': return value exceeds ABI cell-count limit",
                        func.name
                    )
                })?;
                if cells == 0 {
                    return Err(format!(
                        "function '{}': zero-sized indirect returns are not yet qualified",
                        func.name
                    ));
                }
                lir_param_types.push(LirType::Ptr(cells));
                Some(0u32)
            }
            ValueAbi::Scalar | ValueAbi::Reference { .. } | ValueAbi::Unit => None,
        };

        for ((param, _annotation), abi) in func
            .params
            .iter()
            .zip(func.param_types.iter())
            .zip(func_abi.params.iter())
        {
            let slot: u32 = lir_param_types
                .len()
                .try_into()
                .map_err(|_| format!("function '{}': parameter slot overflow", func.name))?;
            let lir_ty = abi.lir_param_type()?;
            lir_param_types.push(lir_ty);
            if var_slots.insert(param.clone(), slot).is_some() {
                return Err(format!(
                    "function '{}' contains duplicate parameter '{}'",
                    func.name, param
                ));
            }
            if let ValueAbi::Reference { mutable } = abi {
                incoming_reference_slots.insert(param.clone(), slot);
                incoming_reference_mutability.insert(param.clone(), *mutable);
            }
            if let ValueAbi::Indirect { type_name, fields } = abi {
                aggregate_slots.insert(
                    param.clone(),
                    AggregateStorage {
                        type_name: type_name.clone(),
                        base_slot: None,
                        indirect_slot: Some(slot),
                        fields: fields.clone(),
                    },
                );
            }
        }
        let mut next_slot: u32 = lir_param_types
            .len()
            .try_into()
            .map_err(|_| format!("function '{}': parameter slot overflow", func.name))?;

        // Pre-allocate scalar cells and aggregate frames. Aggregate values use
        // contiguous eight-byte cells in declaration order; the base slot is the
        // deepest cell so positive byte offsets advance through the aggregate.
        let mut const_ints: HashMap<String, i64> = HashMap::new();
        for block in &func.blocks {
            for instr in &block.instrs {
                match instr {
                    mir::Instruction::AggregateInit {
                        dest,
                        type_name,
                        fields,
                    } => {
                        let ordered = ordered_aggregate_fields(type_name, fields, &struct_layouts)?;
                        let base_slot = if ordered.is_empty() {
                            None
                        } else {
                            let width: u32 = ordered.len().try_into().map_err(|_| {
                                format!(
                                    "function '{}': aggregate '{}' is too large",
                                    func.name, type_name
                                )
                            })?;
                            let base = next_slot.checked_add(width - 1).ok_or_else(|| {
                                format!("function '{}': local slot space overflow", func.name)
                            })?;
                            next_slot = next_slot.checked_add(width).ok_or_else(|| {
                                format!("function '{}': local slot space overflow", func.name)
                            })?;
                            Some(base)
                        };
                        aggregate_slots.insert(
                            dest.clone(),
                            AggregateStorage {
                                type_name: type_name.clone(),
                                base_slot,
                                indirect_slot: None,
                                fields: ordered,
                            },
                        );
                    }
                    mir::Instruction::EnumInit {
                        dest,
                        type_name,
                        variant,
                        tag,
                        fields,
                    } => {
                        let payload_width = validate_enum_initializer(
                            type_name,
                            variant,
                            *tag,
                            fields,
                            &enum_layouts,
                        )?;
                        let width = payload_width.checked_add(1).ok_or_else(|| {
                            format!(
                                "function '{}': enum '{}' layout width overflow",
                                func.name, type_name
                            )
                        })?;
                        let width: u32 = width.try_into().map_err(|_| {
                            format!(
                                "function '{}': enum '{}' is too large",
                                func.name, type_name
                            )
                        })?;
                        let base = next_slot.checked_add(width - 1).ok_or_else(|| {
                            format!("function '{}': local slot space overflow", func.name)
                        })?;
                        next_slot = next_slot.checked_add(width).ok_or_else(|| {
                            format!("function '{}': local slot space overflow", func.name)
                        })?;
                        let mut storage_fields = Vec::with_capacity(payload_width + 1);
                        storage_fields.push("@tag".to_string());
                        storage_fields
                            .extend((0..payload_width).map(|index| format!("@payload{index}")));
                        aggregate_slots.insert(
                            dest.clone(),
                            AggregateStorage {
                                type_name: type_name.clone(),
                                base_slot: Some(base),
                                indirect_slot: None,
                                fields: storage_fields,
                            },
                        );
                    }
                    mir::Instruction::SliceAccess {
                        dest,
                        base,
                        start,
                        end,
                        inclusive,
                    } => {
                        let source = aggregate_slots.get(base).cloned().ok_or_else(|| {
                            format!(
                                "function '{}': slice base '{}' has no aggregate storage",
                                func.name, base
                            )
                        })?;
                        if !matches!(source.type_name.as_str(), "Array" | "Slice") {
                            return Err(format!(
                                "function '{}': slicing is qualified only for arrays/slices, got '{}'",
                                func.name, source.type_name
                            ));
                        }
                        let start_value = const_ints.get(start).copied().ok_or_else(|| {
                            format!(
                                "function '{}': dynamic slice starts are not yet qualified in v0.1.4",
                                func.name
                            )
                        })?;
                        let end_value = const_ints.get(end).copied().ok_or_else(|| {
                            format!(
                                "function '{}': dynamic slice ends are not yet qualified in v0.1.4",
                                func.name
                            )
                        })?;
                        let (start_index, end_exclusive) = constant_slice_window(
                            start_value,
                            end_value,
                            *inclusive,
                            source.fields.len(),
                        )?;
                        let slice_len = end_exclusive - start_index;
                        let base_slot = if slice_len == 0 {
                            None
                        } else {
                            let source_base = source.base_slot.ok_or_else(|| {
                                format!(
                                    "function '{}': non-empty slice source '{}' has no base slot",
                                    func.name, base
                                )
                            })?;
                            let start_slot = u32::try_from(start_index).map_err(|_| {
                                format!("function '{}': slice start does not fit u32", func.name)
                            })?;
                            Some(source_base.checked_sub(start_slot).ok_or_else(|| {
                                format!("function '{}': slice base adjustment underflow", func.name)
                            })?)
                        };
                        aggregate_slots.insert(
                            dest.clone(),
                            AggregateStorage {
                                type_name: "Slice".to_string(),
                                base_slot,
                                indirect_slot: None,
                                fields: (0..slice_len).map(|index| index.to_string()).collect(),
                            },
                        );
                    }
                    mir::Instruction::ConstInt { dest, value } => {
                        const_ints.insert(dest.clone(), *value);
                        if !var_slots.contains_key(dest) {
                            var_slots.insert(dest.clone(), next_slot);
                            next_slot += 1;
                        }
                    }
                    mir::Instruction::Move { dest, src }
                    | mir::Instruction::Assign { dest, src }
                    | mir::Instruction::LinearMove { dest, src } => {
                        if let Some(storage) = aggregate_slots.get(src).cloned() {
                            aggregate_slots.insert(dest.clone(), storage);
                        } else if !var_slots.contains_key(dest) {
                            var_slots.insert(dest.clone(), next_slot);
                            next_slot += 1;
                        }
                    }
                    mir::Instruction::Call {
                        dest, func: callee, ..
                    } => {
                        let callee_abi = function_abis.get(callee).ok_or_else(|| {
                            format!(
                                "function '{}': unresolved call target '{}' during ABI allocation",
                                func.name, callee
                            )
                        })?;
                        match &callee_abi.ret {
                            ValueAbi::Indirect { type_name, fields } => {
                                let storage = allocate_local_aggregate(
                                    &mut next_slot,
                                    type_name.clone(),
                                    fields.clone(),
                                    &func.name,
                                )?;
                                aggregate_slots.insert(dest.clone(), storage);
                            }
                            ValueAbi::Reference { .. } => {
                                return Err(format!(
                                    "function '{}': reference call returns require outlives proof",
                                    func.name
                                ));
                            }
                            ValueAbi::Scalar | ValueAbi::Unit => {
                                if !var_slots.contains_key(dest) {
                                    var_slots.insert(dest.clone(), next_slot);
                                    next_slot += 1;
                                }
                            }
                        }
                    }
                    mir::Instruction::ConstStr { dest, .. } => {
                        if !aggregate_slots.contains_key(dest) {
                            let storage = allocate_local_aggregate(
                                &mut next_slot,
                                "String".to_string(),
                                vec!["@data".to_string(), "len".to_string()],
                                &func.name,
                            )?;
                            aggregate_slots.insert(dest.clone(), storage);
                        }
                    }
                    mir::Instruction::ConstBytes { dest, .. } => {
                        if !aggregate_slots.contains_key(dest) {
                            let storage = allocate_local_aggregate(
                                &mut next_slot,
                                "Bytes".to_string(),
                                vec!["@data".to_string(), "len".to_string()],
                                &func.name,
                            )?;
                            aggregate_slots.insert(dest.clone(), storage);
                        }
                    }
                    mir::Instruction::ConstBool { dest, .. }
                    | mir::Instruction::BinaryOp { dest, .. }
                    | mir::Instruction::UnaryOp { dest, .. }
                    | mir::Instruction::Deref { dest, .. }
                    | mir::Instruction::Reborrow { dest, .. }
                    | mir::Instruction::EnumTag { dest, .. }
                    | mir::Instruction::EnumPayloadAccess { dest, .. }
                    | mir::Instruction::FieldAccess { dest, .. }
                    | mir::Instruction::StructAccess { dest, .. }
                    | mir::Instruction::IndexAccess { dest, .. }
                        if !var_slots.contains_key(dest) =>
                    {
                        var_slots.insert(dest.clone(), next_slot);
                        next_slot += 1;
                    }
                    _ => {}
                }
            }
        }

        let mut lir_instrs = Vec::new();
        // Machine parameters (including a hidden indirect-return pointer when
        // present) arrive on the evaluation stack in ABI order.
        for slot in (0..lir_param_types.len()).rev() {
            lir_instrs.push(LirInstr::Store(slot as u32));
        }

        let mut label_map: HashMap<usize, usize> = HashMap::new();
        let mut jump_patches: Vec<(usize, usize)> = Vec::new();
        let mut cond_patches: Vec<(usize, usize)> = Vec::new();
        let mut saw_return = false;
        // Safe references in the first v0.2.0.0 wedge are compiler-known aliases
        // to frame-local scalar places. They are not materialized as forgeable
        // integer pointers in LIR.
        #[derive(Debug, Clone)]
        enum ReferenceStorage {
            LocalPlace(String),
            IncomingPtr(u32),
        }
        let mut reference_places: HashMap<String, ReferenceStorage> = incoming_reference_slots
            .into_iter()
            .map(|(name, slot)| (name, ReferenceStorage::IncomingPtr(slot)))
            .collect();
        let mut reference_mutability = incoming_reference_mutability;

        for block in &func.blocks {
            for instr in &block.instrs {
                match instr {
                    mir::Instruction::ConstInt { dest, value } => {
                        lir_instrs.push(LirInstr::Const(*value));
                        lir_instrs.push(LirInstr::Store(slot_of(&var_slots, dest, &func.name)?));
                    }
                    mir::Instruction::ConstBool { dest, value } => {
                        lir_instrs.push(LirInstr::Const(i64::from(*value)));
                        lir_instrs.push(LirInstr::Store(slot_of(&var_slots, dest, &func.name)?));
                    }
                    mir::Instruction::Borrow {
                        dest,
                        place,
                        mutable,
                    } => {
                        if aggregate_slots.contains_key(place) {
                            return Err(format!(
                                "function '{}': aggregate borrows are not yet qualified for '{}'",
                                func.name, place
                            ));
                        }
                        let _ = slot_of(&var_slots, place, &func.name)?;
                        reference_places
                            .insert(dest.clone(), ReferenceStorage::LocalPlace(place.clone()));
                        reference_mutability.insert(dest.clone(), *mutable);
                    }
                    mir::Instruction::Reborrow {
                        dest,
                        parent,
                        mutable,
                    } => {
                        let storage = reference_places
                            .get(parent)
                            .ok_or_else(|| {
                                format!(
                                    "function '{}': reborrow '{}' has no proven parent origin",
                                    func.name, parent
                                )
                            })?
                            .clone();
                        reference_places.insert(dest.clone(), storage);
                        reference_mutability.insert(dest.clone(), *mutable);
                    }
                    mir::Instruction::Deref { dest, reference } => {
                        let storage = reference_places.get(reference).ok_or_else(|| {
                            format!(
                                "function '{}': dereference '{}' has no proven borrow origin",
                                func.name, reference
                            )
                        })?;
                        match storage {
                            ReferenceStorage::LocalPlace(place) => {
                                emit_scalar_value(&mut lir_instrs, &var_slots, place, &func.name)?;
                            }
                            ReferenceStorage::IncomingPtr(slot) => {
                                lir_instrs.push(LirInstr::LoadPtrOffset(*slot, 0));
                            }
                        }
                        lir_instrs.push(LirInstr::Store(slot_of(&var_slots, dest, &func.name)?));
                    }
                    mir::Instruction::DerefAssign { reference, src } => {
                        let storage = reference_places.get(reference).ok_or_else(|| {
                            format!("function '{}': dereference assignment '{}' has no proven borrow origin", func.name, reference)
                        })?;
                        emit_scalar_value(&mut lir_instrs, &var_slots, src, &func.name)?;
                        match storage {
                            ReferenceStorage::LocalPlace(place) => {
                                lir_instrs
                                    .push(LirInstr::Store(slot_of(&var_slots, place, &func.name)?));
                            }
                            ReferenceStorage::IncomingPtr(slot) => {
                                lir_instrs.push(LirInstr::StorePtrOffset(*slot, 0));
                            }
                        }
                    }
                    mir::Instruction::ConstStr { dest, value } => {
                        let storage = aggregate_slots.get(dest).ok_or_else(|| {
                            format!(
                                "function '{}': String literal '{}' has no descriptor storage",
                                func.name, dest
                            )
                        })?;
                        if storage.type_name != "String" || storage.fields.len() != 2 {
                            return Err(format!(
                                "function '{}': String literal '{}' has invalid descriptor layout",
                                func.name, dest
                            ));
                        }
                        let base = storage.base_slot.ok_or_else(|| {
                            format!(
                                "function '{}': String literal '{}' has no local base",
                                func.name, dest
                            )
                        })?;
                        lir_instrs.push(LirInstr::StringRef(value.clone()));
                        lir_instrs.push(LirInstr::StoreOffset(base, 0));
                        let len = i64::try_from(value.len()).map_err(|_| {
                            format!("function '{}': String literal is too large", func.name)
                        })?;
                        lir_instrs.push(LirInstr::Const(len));
                        lir_instrs.push(LirInstr::StoreOffset(base, 8));
                    }
                    mir::Instruction::ConstBytes { dest, value } => {
                        let storage = aggregate_slots.get(dest).ok_or_else(|| {
                            format!(
                                "function '{}': byte-string literal '{}' has no descriptor storage",
                                func.name, dest
                            )
                        })?;
                        if storage.type_name != "Bytes" || storage.fields.len() != 2 {
                            return Err(format!(
                                "function '{}': byte-string literal '{}' has invalid descriptor layout",
                                func.name, dest
                            ));
                        }
                        let base = storage.base_slot.ok_or_else(|| {
                            format!(
                                "function '{}': byte-string literal '{}' has no local base",
                                func.name, dest
                            )
                        })?;
                        lir_instrs.push(LirInstr::BytesRef(value.clone()));
                        lir_instrs.push(LirInstr::StoreOffset(base, 0));
                        let len = i64::try_from(value.len()).map_err(|_| {
                            format!("function '{}': byte-string literal is too large", func.name)
                        })?;
                        lir_instrs.push(LirInstr::Const(len));
                        lir_instrs.push(LirInstr::StoreOffset(base, 8));
                    }
                    mir::Instruction::Move { dest, src }
                    | mir::Instruction::Assign { dest, src }
                    | mir::Instruction::LinearMove { dest, src } => {
                        if aggregate_slots.contains_key(src) {
                            if !aggregate_slots.contains_key(dest) {
                                return Err(format!(
                                    "function '{}': aggregate alias '{}' lost layout from '{}'",
                                    func.name, dest, src
                                ));
                            }
                        } else {
                            emit_scalar_value(&mut lir_instrs, &var_slots, src, &func.name)?;
                            lir_instrs
                                .push(LirInstr::Store(slot_of(&var_slots, dest, &func.name)?));
                        }
                    }
                    mir::Instruction::AggregateInit { dest, fields, .. } => {
                        let storage = aggregate_slots.get(dest).ok_or_else(|| {
                            format!(
                                "function '{}': aggregate '{}' has no allocated storage",
                                func.name, dest
                            )
                        })?;
                        if let Some(base_slot) = storage.base_slot {
                            for (index, field_name) in storage.fields.iter().enumerate() {
                                let value = fields.iter().find(|(name, _)| name == field_name).map(|(_, value)| value).ok_or_else(|| {
                                    format!("function '{}': aggregate '{}' is missing lowered field '{}'", func.name, dest, field_name)
                                })?;
                                if aggregate_slots.contains_key(value) {
                                    return Err(format!("function '{}': nested/string aggregate field '{}' is not yet qualified in the v0.1.4 scalar-cell layout", func.name, field_name));
                                }
                                emit_scalar_value(&mut lir_instrs, &var_slots, value, &func.name)?;
                                lir_instrs
                                    .push(LirInstr::StoreOffset(base_slot, (index as i64) * 8));
                            }
                        }
                    }
                    mir::Instruction::EnumInit {
                        dest,
                        type_name,
                        variant,
                        tag,
                        fields,
                    } => {
                        let payload_width = validate_enum_initializer(
                            type_name,
                            variant,
                            *tag,
                            fields,
                            &enum_layouts,
                        )?;
                        let storage = aggregate_slots.get(dest).ok_or_else(|| {
                            format!(
                                "function '{}': enum '{}' has no allocated storage",
                                func.name, dest
                            )
                        })?;
                        let base_slot = storage.base_slot.ok_or_else(|| {
                            format!(
                                "function '{}': enum '{}' unexpectedly has no base slot",
                                func.name, dest
                            )
                        })?;

                        lir_instrs.push(LirInstr::Const(i64::from(*tag)));
                        lir_instrs.push(LirInstr::StoreOffset(base_slot, 0));

                        for index in 0..payload_width {
                            lir_instrs.push(LirInstr::Const(0));
                            lir_instrs
                                .push(LirInstr::StoreOffset(base_slot, ((index + 1) as i64) * 8));
                        }

                        for (index, (_, value)) in fields.iter().enumerate() {
                            if aggregate_slots.contains_key(value) {
                                return Err(format!(
                                    "function '{}': nested/string enum payload {} for '{}::{}' is not yet qualified in v0.1.4",
                                    func.name, index, type_name, variant
                                ));
                            }
                            emit_scalar_value(&mut lir_instrs, &var_slots, value, &func.name)?;
                            lir_instrs
                                .push(LirInstr::StoreOffset(base_slot, ((index + 1) as i64) * 8));
                        }
                    }
                    mir::Instruction::EnumTag { dest, base } => {
                        let storage = aggregate_slots.get(base).ok_or_else(|| {
                            format!(
                                "function '{}': enum tag base '{}' is not an aggregate",
                                func.name, base
                            )
                        })?;
                        if !enum_layouts.contains_key(&storage.type_name) {
                            return Err(format!(
                                "function '{}': enum tag access requires enum storage, got '{}'",
                                func.name, storage.type_name
                            ));
                        }
                        emit_aggregate_cell_load(&mut lir_instrs, storage, 0, &func.name)?;
                        lir_instrs.push(LirInstr::Store(slot_of(&var_slots, dest, &func.name)?));
                    }
                    mir::Instruction::EnumPayloadAccess { dest, base, index } => {
                        let storage = aggregate_slots.get(base).ok_or_else(|| {
                            format!(
                                "function '{}': enum payload base '{}' is not an aggregate",
                                func.name, base
                            )
                        })?;
                        if !enum_layouts.contains_key(&storage.type_name) {
                            return Err(format!(
                                "function '{}': enum payload access requires enum storage, got '{}'",
                                func.name, storage.type_name
                            ));
                        }
                        let payload_index = usize::try_from(*index)
                            .map_err(|_| "enum payload index does not fit usize".to_string())?;
                        let payload_width = storage.fields.len().saturating_sub(1);
                        if payload_index >= payload_width {
                            return Err(format!(
                                "function '{}': enum payload index {} out of layout bounds {}",
                                func.name, payload_index, payload_width
                            ));
                        }
                        emit_aggregate_cell_load(
                            &mut lir_instrs,
                            storage,
                            ((payload_index + 1) as i64) * 8,
                            &func.name,
                        )?;
                        lir_instrs.push(LirInstr::Store(slot_of(&var_slots, dest, &func.name)?));
                    }
                    mir::Instruction::BinaryOp {
                        dest,
                        op,
                        left,
                        right,
                    } => {
                        emit_scalar_value(&mut lir_instrs, &var_slots, left, &func.name)?;
                        emit_scalar_value(&mut lir_instrs, &var_slots, right, &func.name)?;
                        use crate::complete_lexer::TokenKind;
                        let lowered = match op {
                            TokenKind::Plus => LirInstr::Add,
                            TokenKind::Minus => LirInstr::Sub,
                            TokenKind::Star => LirInstr::Mul,
                            TokenKind::Slash => LirInstr::Div,
                            TokenKind::Percent => LirInstr::Mod,
                            TokenKind::EqEq => LirInstr::Eq,
                            TokenKind::NotEq => LirInstr::Ne,
                            TokenKind::Lt => LirInstr::Lt,
                            TokenKind::LtEq => LirInstr::Le,
                            TokenKind::Gt => LirInstr::Gt,
                            TokenKind::GtEq => LirInstr::Ge,
                            other => {
                                return Err(format!(
                                    "function '{}': binary operator {:?} is not implemented by the v0.1.4 scalar LIR",
                                    func.name, other
                                ))
                            }
                        };
                        lir_instrs.push(lowered);
                        lir_instrs.push(LirInstr::Store(slot_of(&var_slots, dest, &func.name)?));
                    }
                    mir::Instruction::UnaryOp { dest, op, operand } => {
                        use crate::complete_lexer::TokenKind;
                        match op {
                            TokenKind::Minus => {
                                lir_instrs.push(LirInstr::Const(0));
                                emit_scalar_value(&mut lir_instrs, &var_slots, operand, &func.name)?;
                                lir_instrs.push(LirInstr::Sub);
                            }
                            TokenKind::Bang => {
                                emit_scalar_value(&mut lir_instrs, &var_slots, operand, &func.name)?;
                                lir_instrs.push(LirInstr::Not);
                            }
                            other => {
                                return Err(format!(
                                    "function '{}': unary operator {:?} is not implemented by the v0.1.4 scalar LIR",
                                    func.name, other
                                ))
                            }
                        }
                        lir_instrs.push(LirInstr::Store(slot_of(&var_slots, dest, &func.name)?));
                    }
                    mir::Instruction::Print { src } => {
                        if let Some(storage) = aggregate_slots.get(src) {
                            if !matches!(storage.type_name.as_str(), "String" | "Bytes")
                                || storage.fields.len() != 2
                            {
                                return Err(format!(
                                    "function '{}': printing aggregate '{}' is not qualified; only String and Bytes values are printable",
                                    func.name, storage.type_name
                                ));
                            }
                            emit_aggregate_cell_load(&mut lir_instrs, storage, 0, &func.name)?;
                            emit_aggregate_cell_load(&mut lir_instrs, storage, 8, &func.name)?;
                            lir_instrs.push(LirInstr::PrintBytes);
                        } else {
                            emit_scalar_value(&mut lir_instrs, &var_slots, src, &func.name)?;
                            lir_instrs.push(LirInstr::Call("print".to_string()));
                        }
                    }
                    mir::Instruction::Return { value } => {
                        if !func.returns_value {
                            return Err(format!(
                                "function '{}' has unit/void ABI but MIR returns a value",
                                func.name
                            ));
                        }
                        match &func_abi.ret {
                            ValueAbi::Scalar => {
                                if aggregate_slots.contains_key(value) {
                                    return Err(format!(
                                        "function '{}': scalar return ABI cannot return non-scalar value '{}'",
                                        func.name, value
                                    ));
                                }
                                emit_scalar_value(&mut lir_instrs, &var_slots, value, &func.name)?;
                                lir_instrs.push(LirInstr::Ret);
                            }
                            ValueAbi::Indirect { type_name, fields } => {
                                let storage = aggregate_slots.get(value).ok_or_else(|| {
                                    format!(
                                        "function '{}': indirect return expects aggregate '{}', got scalar '{}'",
                                        func.name, type_name, value
                                    )
                                })?;
                                if &storage.type_name != type_name
                                    || storage.fields.len() != fields.len()
                                {
                                    return Err(format!(
                                        "function '{}': return value '{}' has layout '{}'/{} cells, expected '{}'/{} cells",
                                        func.name,
                                        value,
                                        storage.type_name,
                                        storage.fields.len(),
                                        type_name,
                                        fields.len()
                                    ));
                                }
                                let sret = sret_slot.ok_or_else(|| {
                                    format!(
                                        "function '{}': missing hidden return pointer",
                                        func.name
                                    )
                                })?;
                                for index in 0..fields.len() {
                                    let offset = i64::try_from(index)
                                        .map_err(|_| {
                                            "aggregate return offset overflow".to_string()
                                        })?
                                        .checked_mul(8)
                                        .ok_or_else(|| {
                                            "aggregate return offset overflow".to_string()
                                        })?;
                                    emit_aggregate_cell_load(
                                        &mut lir_instrs,
                                        storage,
                                        offset,
                                        &func.name,
                                    )?;
                                    lir_instrs.push(LirInstr::StorePtrOffset(sret, offset));
                                }
                                lir_instrs.push(LirInstr::Ret);
                            }
                            ValueAbi::Reference { .. } => {
                                return Err(format!(
                                    "function '{}': returning safe references requires outlives proof",
                                    func.name
                                ));
                            }
                            ValueAbi::Unit => {
                                return Err(format!(
                                    "function '{}' has unit ABI but MIR returns a value",
                                    func.name
                                ));
                            }
                        }
                        saw_return = true;
                    }
                    mir::Instruction::Call {
                        dest,
                        func: callee,
                        args,
                    } => {
                        if callee.starts_with("__omni_unsupported_") {
                            return Err(format!(
                                "function '{}': source feature reached unsupported MIR sentinel '{}'",
                                func.name, callee
                            ));
                        }
                        let callee_abi = function_abis.get(callee).ok_or_else(|| {
                            format!(
                                "function '{}': unresolved call target '{}' in LIR lowering",
                                func.name, callee
                            )
                        })?;
                        if args.len() != callee_abi.params.len() {
                            return Err(format!(
                                "function '{}': call '{}' has {} MIR args but ABI expects {}",
                                func.name,
                                callee,
                                args.len(),
                                callee_abi.params.len()
                            ));
                        }

                        if let ValueAbi::Indirect { type_name, fields } = &callee_abi.ret {
                            let storage = aggregate_slots.get(dest).ok_or_else(|| {
                                format!(
                                    "function '{}': aggregate call result '{}' has no caller-owned storage",
                                    func.name, dest
                                )
                            })?;
                            if &storage.type_name != type_name
                                || storage.fields.len() != fields.len()
                            {
                                return Err(format!(
                                    "function '{}': call result '{}' layout does not match '{}',",
                                    func.name, dest, type_name
                                ));
                            }
                            emit_aggregate_pointer(&mut lir_instrs, storage, &func.name)?;
                        }

                        for (arg, expected) in args.iter().zip(callee_abi.params.iter()) {
                            match expected {
                                ValueAbi::Scalar => {
                                    if aggregate_slots.contains_key(arg) {
                                        return Err(format!(
                                            "function '{}': scalar parameter of '{}' cannot receive non-scalar argument '{}'",
                                            func.name, callee, arg
                                        ));
                                    }
                                    emit_scalar_value(
                                        &mut lir_instrs,
                                        &var_slots,
                                        arg,
                                        &func.name,
                                    )?;
                                }
                                ValueAbi::Reference {
                                    mutable: expected_mutable,
                                } => {
                                    let actual_mutable = reference_mutability.get(arg).copied().ok_or_else(|| {
                                        format!(
                                            "function '{}': reference argument '{}' has no mutability proof",
                                            func.name, arg
                                        )
                                    })?;
                                    if *expected_mutable && !actual_mutable {
                                        return Err(format!(
                                            "function '{}': shared reference '{}' cannot satisfy mutable reference parameter of '{}'",
                                            func.name, arg, callee
                                        ));
                                    }
                                    let storage = reference_places.get(arg).ok_or_else(|| {
                                        format!(
                                            "function '{}': reference parameter of '{}' expects a proven safe reference, got '{}'",
                                            func.name, callee, arg
                                        )
                                    })?;
                                    match storage {
                                        ReferenceStorage::LocalPlace(place) => {
                                            lir_instrs.push(LirInstr::GetAddr(slot_of(
                                                &var_slots, place, &func.name,
                                            )?));
                                        }
                                        ReferenceStorage::IncomingPtr(slot) => {
                                            lir_instrs.push(LirInstr::Load(*slot));
                                        }
                                    }
                                }
                                ValueAbi::Indirect { type_name, fields } => {
                                    let storage = aggregate_slots.get(arg).ok_or_else(|| {
                                        format!(
                                            "function '{}': indirect parameter of '{}' expects aggregate '{}', got '{}'",
                                            func.name, callee, type_name, arg
                                        )
                                    })?;
                                    if &storage.type_name != type_name
                                        || storage.fields.len() != fields.len()
                                    {
                                        return Err(format!(
                                            "function '{}': aggregate argument '{}' layout '{}' does not match parameter '{}'",
                                            func.name, arg, storage.type_name, type_name
                                        ));
                                    }
                                    emit_aggregate_pointer(&mut lir_instrs, storage, &func.name)?;
                                }
                                ValueAbi::Unit => {
                                    return Err(format!(
                                        "function '{}': unit cannot be passed as an argument to '{}'",
                                        func.name, callee
                                    ));
                                }
                            }
                        }
                        lir_instrs.push(LirInstr::Call(callee.clone()));
                        match &callee_abi.ret {
                            ValueAbi::Scalar => {
                                lir_instrs
                                    .push(LirInstr::Store(slot_of(&var_slots, dest, &func.name)?));
                            }
                            ValueAbi::Reference { .. }
                            | ValueAbi::Indirect { .. }
                            | ValueAbi::Unit => {}
                        }
                    }
                    mir::Instruction::Jump { target } => {
                        let idx = lir_instrs.len();
                        lir_instrs.push(LirInstr::Jump(0));
                        jump_patches.push((idx, *target));
                    }
                    mir::Instruction::JumpIf { cond, target } => {
                        emit_scalar_value(&mut lir_instrs, &var_slots, cond, &func.name)?;
                        let idx = lir_instrs.len();
                        lir_instrs.push(LirInstr::CondJump {
                            if_true: 0,
                            if_false: 0,
                        });
                        cond_patches.push((idx, *target));
                    }
                    mir::Instruction::Label { id } => {
                        if label_map.insert(*id, lir_instrs.len()).is_some() {
                            return Err(format!(
                                "function '{}': duplicate MIR label {} during LIR lowering",
                                func.name, id
                            ));
                        }
                        lir_instrs.push(LirInstr::Nop);
                    }
                    mir::Instruction::Drop { var } | mir::Instruction::DropLinear { var } => {
                        if aggregate_slots.contains_key(var) || reference_places.contains_key(var) {
                            // Aggregate cells and compiler-known safe-reference aliases have
                            // no standalone scalar slot to destroy in this wedge.
                            continue;
                        }
                        lir_instrs.push(LirInstr::Drop(slot_of(&var_slots, var, &func.name)?));
                    }
                    mir::Instruction::FieldAssign { base, field, src } => {
                        let storage = aggregate_slots.get(base).ok_or_else(|| {
                            format!(
                                "function '{}': field assignment base '{}' is not an aggregate",
                                func.name, base
                            )
                        })?;
                        let offset = storage.offset_of(field).ok_or_else(|| {
                            format!(
                                "function '{}': aggregate '{}' has no field '{}'",
                                func.name, base, field
                            )
                        })?;
                        emit_scalar_value(&mut lir_instrs, &var_slots, src, &func.name)?;
                        if let Some(base_slot) = storage.base_slot {
                            lir_instrs.push(LirInstr::StoreOffset(base_slot, offset));
                        } else if let Some(ptr_slot) = storage.indirect_slot {
                            lir_instrs.push(LirInstr::StorePtrOffset(ptr_slot, offset));
                        } else {
                            return Err(format!(
                                "function '{}': zero-sized aggregate '{}' has no writable storage",
                                func.name, storage.type_name
                            ));
                        }
                    }
                    mir::Instruction::FieldAccess {
                        dest, base, field, ..
                    } => {
                        let storage = aggregate_slots.get(base).ok_or_else(|| {
                            format!(
                                "function '{}': field access base '{}' is not an aggregate",
                                func.name, base
                            )
                        })?;
                        let offset = storage.offset_of(field).ok_or_else(|| {
                            format!(
                                "function '{}': aggregate '{}' has no field '{}'",
                                func.name, base, field
                            )
                        })?;
                        emit_aggregate_cell_load(&mut lir_instrs, storage, offset, &func.name)?;
                        lir_instrs.push(LirInstr::Store(slot_of(&var_slots, dest, &func.name)?));
                    }
                    mir::Instruction::SliceAccess { dest, .. } => {
                        if !aggregate_slots.contains_key(dest) {
                            return Err(format!(
                                "function '{}': slice '{}' lost its validated local view",
                                func.name, dest
                            ));
                        }
                    }
                    mir::Instruction::IndexAccess { dest, base, index } => {
                        let storage = aggregate_slots.get(base).ok_or_else(|| {
                            format!(
                                "function '{}': index access base '{}' is not an aggregate",
                                func.name, base
                            )
                        })?;
                        if storage.type_name == "Bytes" {
                            emit_aggregate_cell_load(&mut lir_instrs, storage, 0, &func.name)?;
                            emit_aggregate_cell_load(&mut lir_instrs, storage, 8, &func.name)?;
                            emit_scalar_value(&mut lir_instrs, &var_slots, index, &func.name)?;
                            lir_instrs.push(LirInstr::LoadByteIndex);
                        } else {
                            let base_slot = storage.base_slot.ok_or_else(|| {
                                format!(
                                    "function '{}': cannot index zero-sized or indirect aggregate '{}' with local-layout indexing",
                                    func.name, storage.type_name
                                )
                            })?;
                            if let Some(raw_index) = const_ints
                                .get(index)
                                .copied()
                                .or_else(|| index.parse::<i64>().ok())
                            {
                                if raw_index < 0 || (raw_index as usize) >= storage.fields.len() {
                                    return Err(format!(
                                        "function '{}': aggregate index {} out of bounds for length {}",
                                        func.name,
                                        raw_index,
                                        storage.fields.len()
                                    ));
                                }
                                lir_instrs.push(LirInstr::LoadOffset(base_slot, raw_index * 8));
                            } else if matches!(storage.type_name.as_str(), "Array" | "Slice") {
                                emit_scalar_value(&mut lir_instrs, &var_slots, index, &func.name)?;
                                let len = u64::try_from(storage.fields.len()).map_err(|_| {
                                    format!(
                                        "function '{}': array length does not fit u64",
                                        func.name
                                    )
                                })?;
                                lir_instrs.push(LirInstr::LoadIndex {
                                    base: base_slot,
                                    len,
                                });
                            } else {
                                return Err(format!(
                                    "function '{}': dynamic indexing is qualified only for arrays/slices/Bytes in v0.1.4",
                                    func.name
                                ));
                            }
                        }
                        lir_instrs.push(LirInstr::Store(slot_of(&var_slots, dest, &func.name)?));
                    }
                    mir::Instruction::StructAccess { .. } => {
                        return Err(format!("function '{}': ownership-sensitive aggregate mutation remains fail-closed until v0.2.0 ownership semantics are qualified", func.name));
                    }
                    mir::Instruction::StructDef { .. } | mir::Instruction::EnumDef { .. } => {
                        // Type metadata itself has no runtime effect; layouts were collected
                        // before function lowering so runtime accesses use deterministic offsets.
                    }
                    mir::Instruction::MatchBranch {
                        cond,
                        then_block,
                        else_block,
                    } => {
                        emit_scalar_value(&mut lir_instrs, &var_slots, cond, &func.name)?;
                        let idx = lir_instrs.len();
                        lir_instrs.push(LirInstr::CondJump {
                            if_true: 0,
                            if_false: 0,
                        });
                        cond_patches.push((idx, *then_block));
                        let else_idx = lir_instrs.len();
                        lir_instrs.push(LirInstr::Jump(0));
                        jump_patches.push((else_idx, *else_block));
                    }
                    mir::Instruction::Spawn { .. } => {
                        return Err(format!(
                            "function '{}': structured concurrency is not implemented in the v0.1.4 native subset",
                            func.name
                        ));
                    }
                    mir::Instruction::Channel { .. } => {
                        return Err(format!(
                            "function '{}': channel runtime is not implemented in the v0.1.4 native subset",
                            func.name
                        ));
                    }
                }
            }
        }

        for (lir_idx, mir_target) in jump_patches {
            let target = label_map.get(&mir_target).copied().ok_or_else(|| {
                format!(
                    "function '{}': jump references missing MIR label {}",
                    func.name, mir_target
                )
            })?;
            lir_instrs[lir_idx] = LirInstr::Jump(target);
        }
        for (lir_idx, mir_target) in cond_patches {
            let target = label_map.get(&mir_target).copied().ok_or_else(|| {
                format!(
                    "function '{}': conditional jump references missing MIR label {}",
                    func.name, mir_target
                )
            })?;
            let fallthrough = lir_idx + 1;
            lir_instrs[lir_idx] = LirInstr::CondJump {
                if_true: target,
                if_false: fallthrough,
            };
        }

        if !saw_return {
            if func.returns_value {
                if func.synthetic && func.name == "main" {
                    // Script/top-level Stage-0 compatibility: a source file without an
                    // explicit user main exits successfully.
                    lir_instrs.push(LirInstr::Const(0));
                    lir_instrs.push(LirInstr::Ret);
                } else {
                    return Err(format!(
                        "function '{}' has a scalar return ABI but no reachable MIR return",
                        func.name
                    ));
                }
            } else {
                lir_instrs.push(LirInstr::Ret);
            }
        }

        let ret = match &func_abi.ret {
            ValueAbi::Scalar => LirType::I64,
            ValueAbi::Unit | ValueAbi::Reference { .. } | ValueAbi::Indirect { .. } => {
                LirType::Void
            }
        };
        out.add_function(LirFunction::new(
            func.name.clone(),
            lir_param_types,
            ret,
            lir_instrs,
            func.effects.clone(),
        ));
    }

    Ok(out)
}

fn slot_of(slots: &HashMap<String, u32>, name: &str, func: &str) -> Result<u32, String> {
    slots.get(name).copied().ok_or_else(|| {
        format!(
            "function '{}': MIR references unknown scalar value '{}'",
            func, name
        )
    })
}

fn emit_scalar_value(
    out: &mut Vec<LirInstr>,
    slots: &HashMap<String, u32>,
    value: &str,
    func: &str,
) -> Result<(), String> {
    if let Ok(v) = value.parse::<i64>() {
        out.push(LirInstr::Const(v));
        Ok(())
    } else {
        out.push(LirInstr::Load(slot_of(slots, value, func)?));
        Ok(())
    }
}

/// Render LIR for diagnostics without pulling a JIT backend into the canonical build.
pub fn compile_lir_module_text(module: &LirModule) -> String {
    let mut out = String::new();
    for func in &module.functions {
        out.push_str(&format!(
            "fn {}({:?}) -> {:?}\n",
            func.name, func.params, func.rets
        ));
        for (ip, instr) in func.body.iter().enumerate() {
            out.push_str(&format!("  {ip}: {instr:?}\n"));
        }
    }
    out
}
