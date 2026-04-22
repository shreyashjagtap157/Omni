use crate::ast::{Program, Stmt};
use crate::lexer::Lexer;
use crate::parser::Parser;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::Write;
use std::fs;
use std::path::Path;

pub const TYPE_EXPORT_SCHEMA_VERSION: &str = "omni.type-export.v1";
pub const TYPE_EXPORT_ABI_VERSION: &str = "omni-stage0-abi-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeExportFormat {
    Json,
    CHeader,
    Python,
}

impl TypeExportFormat {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "c" | "header" | "cheader" => Ok(Self::CHeader),
            "py" | "python" => Ok(Self::Python),
            other => Err(format!("unknown type export format '{}'", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportDocument {
    pub schema_version: String,
    pub abi_version: String,
    pub items: Vec<ExportedItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExportedItem {
    Function(FunctionExport),
    Struct(StructExport),
    Enum(EnumExport),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionExport {
    pub name: String,
    pub type_params: Vec<String>,
    pub params: Vec<ExportedParam>,
    pub return_type: Option<String>,
    pub effects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructExport {
    pub name: String,
    pub fields: Vec<ExportedField>,
    pub is_linear: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnumExport {
    pub name: String,
    pub variants: Vec<ExportedVariant>,
    pub is_sealed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportedParam {
    pub name: String,
    pub type_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportedField {
    pub name: String,
    pub type_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportedVariant {
    pub name: String,
    pub fields: Vec<ExportedField>,
}

pub fn parse_raw_program(path: &Path) -> Result<Program, String> {
    let source = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

pub fn export_program(program: &Program) -> ExportDocument {
    let mut items = Vec::new();

    for stmt in &program.stmts {
        match stmt {
            Stmt::Fn {
                name,
                type_params,
                params,
                ret_type,
                effects,
                ..
            } => {
                items.push(ExportedItem::Function(FunctionExport {
                    name: name.clone(),
                    type_params: type_params.clone(),
                    params: params
                        .iter()
                        .map(|param_name| ExportedParam {
                            name: param_name.clone(),
                            type_name: None,
                        })
                        .collect(),
                    return_type: normalize_optional_type(ret_type.clone()),
                    effects: normalize_effects(effects),
                }));
            }
            Stmt::Struct {
                name,
                fields,
                is_linear,
            } => {
                items.push(ExportedItem::Struct(StructExport {
                    name: name.clone(),
                    fields: fields
                        .iter()
                        .map(|(field_name, field_type)| ExportedField {
                            name: field_name.clone(),
                            type_name: normalize_type_name(field_type),
                        })
                        .collect(),
                    is_linear: *is_linear,
                }));
            }
            Stmt::Enum {
                name,
                variants,
                is_sealed,
            } => {
                items.push(ExportedItem::Enum(EnumExport {
                    name: name.clone(),
                    variants: variants
                        .iter()
                        .map(|variant| ExportedVariant {
                            name: variant.name.clone(),
                            fields: variant
                                .fields
                                .iter()
                                .map(|(field_name, field_type)| ExportedField {
                                    name: field_name.clone(),
                                    type_name: normalize_type_name(field_type),
                                })
                                .collect(),
                        })
                        .collect(),
                    is_sealed: *is_sealed,
                }));
            }
            _ => {}
        }
    }

    ExportDocument {
        schema_version: TYPE_EXPORT_SCHEMA_VERSION.to_string(),
        abi_version: TYPE_EXPORT_ABI_VERSION.to_string(),
        items,
    }
}

pub fn document_to_json(document: &ExportDocument) -> Result<String, String> {
    serde_json::to_string_pretty(document).map_err(|e| e.to_string())
}

pub fn document_to_c_header(document: &ExportDocument) -> Result<String, String> {
    let mut output = String::new();
    writeln!(output, "#pragma once").map_err(|e| e.to_string())?;
    writeln!(output, "#include <stdbool.h>").map_err(|e| e.to_string())?;
    writeln!(output, "#include <stdint.h>").map_err(|e| e.to_string())?;
    writeln!(output, "").map_err(|e| e.to_string())?;
    writeln!(output, "typedef intptr_t omni_opaque_t;").map_err(|e| e.to_string())?;
    writeln!(output, "").map_err(|e| e.to_string())?;
    writeln!(output, "/* ABI version: {} */", document.abi_version).map_err(|e| e.to_string())?;

    for item in &document.items {
        match item {
            ExportedItem::Function(function) => {
                if !function.effects.is_empty() {
                    writeln!(output, "/* effects: {} */", function.effects.join(", "))
                        .map_err(|e| e.to_string())?;
                }

                let return_type = c_type_for(function.return_type.as_deref(), true);
                let params = if function.params.is_empty() {
                    "void".to_string()
                } else {
                    function
                        .params
                        .iter()
                        .map(|param| {
                            format!(
                                "{} {}",
                                c_type_for(param.type_name.as_deref(), false),
                                sanitize_c_identifier(&param.name)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                };

                writeln!(
                    output,
                    "extern {} {}({});",
                    return_type,
                    sanitize_c_identifier(&function.name),
                    params
                )
                .map_err(|e| e.to_string())?;
                writeln!(output, "").map_err(|e| e.to_string())?;
            }
            ExportedItem::Struct(strukt) => {
                writeln!(
                    output,
                    "typedef struct {} {{",
                    sanitize_c_identifier(&strukt.name)
                )
                .map_err(|e| e.to_string())?;
                for field in &strukt.fields {
                    writeln!(
                        output,
                        "    {} {};",
                        c_type_for(Some(field.type_name.as_str()), false),
                        sanitize_c_identifier(&field.name)
                    )
                    .map_err(|e| e.to_string())?;
                }
                writeln!(output, "}} {};", sanitize_c_identifier(&strukt.name))
                    .map_err(|e| e.to_string())?;
                writeln!(output, "").map_err(|e| e.to_string())?;
            }
            ExportedItem::Enum(enm) => {
                writeln!(
                    output,
                    "/* enum {} is exported as an opaque tag-only type in this Stage0 header */",
                    sanitize_c_identifier(&enm.name)
                )
                .map_err(|e| e.to_string())?;
                if !enm.variants.is_empty() {
                    let variant_names = enm
                        .variants
                        .iter()
                        .map(|variant| sanitize_c_identifier(&variant.name))
                        .collect::<Vec<_>>()
                        .join(", ");
                    writeln!(output, "/* variants: {} */", variant_names)
                        .map_err(|e| e.to_string())?;
                }
                writeln!(
                    output,
                    "typedef struct {} {};",
                    sanitize_c_identifier(&enm.name),
                    sanitize_c_identifier(&enm.name)
                )
                .map_err(|e| e.to_string())?;
                writeln!(output, "").map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(output)
}

pub fn document_to_python_module(document: &ExportDocument) -> Result<String, String> {
    let mut output = String::new();
    writeln!(
        output,
        "\"\"\"Auto-generated Omni Python bindings (Stage0 scaffold).\"\"\""
    )
    .map_err(|e| e.to_string())?;
    writeln!(output, "import ctypes as _ctypes").map_err(|e| e.to_string())?;
    writeln!(output, "").map_err(|e| e.to_string())?;
    writeln!(output, "OMNI_BINDING_ABI = {:?}", document.abi_version).map_err(|e| e.to_string())?;
    writeln!(output, "OPAQUE = _ctypes.c_void_p").map_err(|e| e.to_string())?;
    writeln!(output, "").map_err(|e| e.to_string())?;

    for item in &document.items {
        match item {
            ExportedItem::Struct(strukt) => {
                writeln!(
                    output,
                    "class {}(_ctypes.Structure):",
                    sanitize_c_identifier(&strukt.name)
                )
                .map_err(|e| e.to_string())?;
                if strukt.fields.is_empty() {
                    writeln!(output, "    _fields_ = []").map_err(|e| e.to_string())?;
                } else {
                    writeln!(output, "    _fields_ = [").map_err(|e| e.to_string())?;
                    for field in &strukt.fields {
                        writeln!(
                            output,
                            "        ({:?}, {}),",
                            field.name,
                            python_type_for(Some(field.type_name.as_str()))
                        )
                        .map_err(|e| e.to_string())?;
                    }
                    writeln!(output, "    ]").map_err(|e| e.to_string())?;
                }
                writeln!(output, "").map_err(|e| e.to_string())?;
            }
            ExportedItem::Enum(enm) => {
                writeln!(
                    output,
                    "# enum {} is exported as an opaque handle in this scaffold",
                    sanitize_c_identifier(&enm.name)
                )
                .map_err(|e| e.to_string())?;
                writeln!(output, "{} = OPAQUE", sanitize_c_identifier(&enm.name))
                    .map_err(|e| e.to_string())?;
                writeln!(output, "").map_err(|e| e.to_string())?;
            }
            ExportedItem::Function(function) => {
                writeln!(
                    output,
                    "def configure_{}(lib):",
                    sanitize_c_identifier(&function.name)
                )
                .map_err(|e| e.to_string())?;
                let argtypes = if function.params.is_empty() {
                    "[]".to_string()
                } else {
                    let values = function
                        .params
                        .iter()
                        .map(|param| python_type_for(param.type_name.as_deref()).to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("[{}]", values)
                };
                let restype = python_type_for(function.return_type.as_deref());
                writeln!(
                    output,
                    "    lib.{}.argtypes = {}",
                    sanitize_c_identifier(&function.name),
                    argtypes
                )
                .map_err(|e| e.to_string())?;
                writeln!(
                    output,
                    "    lib.{}.restype = {}",
                    sanitize_c_identifier(&function.name),
                    restype
                )
                .map_err(|e| e.to_string())?;
                writeln!(output, "    return lib").map_err(|e| e.to_string())?;
                writeln!(output, "").map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(output)
}

fn normalize_optional_type(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(normalize_type_name(trimmed))
        }
    })
}

fn normalize_type_name(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "unknown".to_string();
    }
    trimmed.to_string()
}

fn normalize_effects(effects: &[String]) -> Vec<String> {
    let mut set = BTreeSet::new();
    for effect in effects {
        let trimmed = effect.trim().to_ascii_lowercase();
        if !trimmed.is_empty() {
            set.insert(trimmed);
        }
    }
    set.into_iter().collect()
}

fn c_type_for(type_name: Option<&str>, is_return_type: bool) -> &'static str {
    match type_name.map(|value| value.trim().to_ascii_lowercase()) {
        Some(ref value) if value == "bool" => "bool",
        Some(ref value)
            if value == "int" || value == "i64" || value == "isize" || value == "usize" =>
        {
            "int64_t"
        }
        Some(ref value) if value == "string" || value == "str" => "const char *",
        Some(ref value) if value == "void" || value == "unit" || value == "()" => {
            if is_return_type {
                "void"
            } else {
                "omni_opaque_t"
            }
        }
        Some(ref value) if value.starts_with('*') => "omni_opaque_t",
        Some(_) => "omni_opaque_t",
        None => {
            if is_return_type {
                "omni_opaque_t"
            } else {
                "omni_opaque_t"
            }
        }
    }
}

fn python_type_for(type_name: Option<&str>) -> &'static str {
    match type_name.map(|value| value.trim().to_ascii_lowercase()) {
        Some(ref value) if value == "bool" => "_ctypes.c_bool",
        Some(ref value)
            if value == "int" || value == "i64" || value == "isize" || value == "usize" =>
        {
            "_ctypes.c_longlong"
        }
        Some(ref value) if value == "string" || value == "str" => "_ctypes.c_char_p",
        Some(ref value) if value == "void" || value == "unit" || value == "()" => "None",
        Some(_) => "OPAQUE",
        None => "OPAQUE",
    }
}

fn sanitize_c_identifier(value: &str) -> String {
    let mut sanitized = String::new();
    for (idx, ch) in value.chars().enumerate() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            if idx == 0 && ch.is_ascii_digit() {
                sanitized.push('_');
            }
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }
    if sanitized.is_empty() {
        "_omni".to_string()
    } else {
        sanitized
    }
}
