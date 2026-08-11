use crate::type_export::{ExportDocument, ExportedItem};

pub fn compare_documents(
    old_document: &ExportDocument,
    new_document: &ExportDocument,
) -> Vec<String> {
    let mut diffs = Vec::new();

    for old_item in &old_document.items {
        match find_matching_item(&new_document.items, old_item) {
            Some(new_item) => compare_item(old_item, new_item, &mut diffs),
            None => diffs.push(format!(
                "{} '{}' was removed",
                item_kind_name(old_item),
                item_name(old_item)
            )),
        }
    }

    diffs
}

fn find_matching_item<'a>(
    items: &'a [ExportedItem],
    probe: &ExportedItem,
) -> Option<&'a ExportedItem> {
    items.iter().find(|item| item_key(item) == item_key(probe))
}

fn compare_item(old_item: &ExportedItem, new_item: &ExportedItem, diffs: &mut Vec<String>) {
    match (old_item, new_item) {
        (ExportedItem::Function(old_fn), ExportedItem::Function(new_fn)) => {
            if old_fn.type_params != new_fn.type_params {
                diffs.push(format!(
                    "function '{}' type parameters changed from {:?} to {:?}",
                    old_fn.name, old_fn.type_params, new_fn.type_params
                ));
            }

            if old_fn.params.len() != new_fn.params.len() {
                diffs.push(format!(
                    "function '{}' parameter count changed from {} to {}",
                    old_fn.name,
                    old_fn.params.len(),
                    new_fn.params.len()
                ));
            }

            for (index, (old_param, new_param)) in
                old_fn.params.iter().zip(new_fn.params.iter()).enumerate()
            {
                if old_param.type_name != new_param.type_name {
                    diffs.push(format!(
                        "function '{}' parameter {} type changed from {:?} to {:?}",
                        old_fn.name, index, old_param.type_name, new_param.type_name
                    ));
                }
            }

            if old_fn.return_type != new_fn.return_type {
                diffs.push(format!(
                    "function '{}' return type changed from {:?} to {:?}",
                    old_fn.name, old_fn.return_type, new_fn.return_type
                ));
            }

            if old_fn.effects != new_fn.effects {
                diffs.push(format!(
                    "function '{}' effects changed from {:?} to {:?}",
                    old_fn.name, old_fn.effects, new_fn.effects
                ));
            }
        }
        (ExportedItem::Struct(old_struct), ExportedItem::Struct(new_struct)) => {
            if old_struct.is_linear != new_struct.is_linear {
                diffs.push(format!(
                    "struct '{}' linearity changed from {} to {}",
                    old_struct.name, old_struct.is_linear, new_struct.is_linear
                ));
            }

            if old_struct.fields.len() != new_struct.fields.len() {
                diffs.push(format!(
                    "struct '{}' field count changed from {} to {}",
                    old_struct.name,
                    old_struct.fields.len(),
                    new_struct.fields.len()
                ));
            }

            for (index, (old_field, new_field)) in old_struct
                .fields
                .iter()
                .zip(new_struct.fields.iter())
                .enumerate()
            {
                if old_field.name != new_field.name {
                    diffs.push(format!(
                        "struct '{}' field {} name changed from '{}' to '{}'",
                        old_struct.name, index, old_field.name, new_field.name
                    ));
                }

                if old_field.type_name != new_field.type_name {
                    diffs.push(format!(
                        "struct '{}' field '{}' type changed from '{}' to '{}'",
                        old_struct.name, old_field.name, old_field.type_name, new_field.type_name
                    ));
                }
            }
        }
        (ExportedItem::Enum(old_enum), ExportedItem::Enum(new_enum)) => {
            if old_enum.is_sealed != new_enum.is_sealed {
                diffs.push(format!(
                    "enum '{}' sealed flag changed from {} to {}",
                    old_enum.name, old_enum.is_sealed, new_enum.is_sealed
                ));
            }

            if old_enum.variants.len() != new_enum.variants.len() {
                diffs.push(format!(
                    "enum '{}' variant count changed from {} to {}",
                    old_enum.name,
                    old_enum.variants.len(),
                    new_enum.variants.len()
                ));
            }

            for (index, (old_variant, new_variant)) in old_enum
                .variants
                .iter()
                .zip(new_enum.variants.iter())
                .enumerate()
            {
                if old_variant.name != new_variant.name {
                    diffs.push(format!(
                        "enum '{}' variant {} name changed from '{}' to '{}'",
                        old_enum.name, index, old_variant.name, new_variant.name
                    ));
                }

                if old_variant.fields.len() != new_variant.fields.len() {
                    diffs.push(format!(
                        "enum '{}' variant '{}' field count changed from {} to {}",
                        old_enum.name,
                        old_variant.name,
                        old_variant.fields.len(),
                        new_variant.fields.len()
                    ));
                }

                for (field_index, (old_field, new_field)) in old_variant
                    .fields
                    .iter()
                    .zip(new_variant.fields.iter())
                    .enumerate()
                {
                    if old_field.name != new_field.name {
                        diffs.push(format!(
                            "enum '{}' variant '{}' field {} name changed from '{}' to '{}'",
                            old_enum.name,
                            old_variant.name,
                            field_index,
                            old_field.name,
                            new_field.name
                        ));
                    }

                    if old_field.type_name != new_field.type_name {
                        diffs.push(format!(
                            "enum '{}' variant '{}' field '{}' type changed from '{}' to '{}'",
                            old_enum.name,
                            old_variant.name,
                            old_field.name,
                            old_field.type_name,
                            new_field.type_name
                        ));
                    }
                }
            }
        }
        _ => {
            diffs.push(format!(
                "{} '{}' changed kind from {} to {}",
                item_kind_name(old_item),
                item_name(old_item),
                item_kind_name(old_item),
                item_kind_name(new_item)
            ));
        }
    }
}

fn item_key(item: &ExportedItem) -> (&'static str, &str) {
    (item_kind_name(item), item_name(item))
}

fn item_name(item: &ExportedItem) -> &str {
    match item {
        ExportedItem::Function(function) => &function.name,
        ExportedItem::Struct(strukt) => &strukt.name,
        ExportedItem::Enum(enm) => &enm.name,
    }
}

fn item_kind_name(item: &ExportedItem) -> &'static str {
    match item {
        ExportedItem::Function(_) => "function",
        ExportedItem::Struct(_) => "struct",
        ExportedItem::Enum(_) => "enum",
    }
}
