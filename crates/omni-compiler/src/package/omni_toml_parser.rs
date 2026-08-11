use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OmniManifest {
    pub name: String,
    pub version: String,
    pub edition: Option<String>,
    pub dependencies: HashMap<String, String>,
    pub modules: Vec<String>,
    pub capabilities: HashMap<String, Vec<String>>,
    pub features: HashMap<String, Vec<String>>,
    pub build_targets: Vec<String>,
}

pub fn parse_manifest(content: &str) -> Result<OmniManifest, String> {
    let mut manifest = OmniManifest::default();
    let mut current_section = String::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].trim().to_string();
            continue;
        }

        if let Some(idx) = line.find('=') {
            let key = line[..idx].trim().to_string();
            let value_str = line[idx + 1..].trim();

            match current_section.as_str() {
                "package" | "" => {
                    if key == "name" {
                        manifest.name = parse_string_value(value_str);
                    } else if key == "version" {
                        manifest.version = parse_string_value(value_str);
                    } else if key == "edition" {
                        manifest.edition = Some(parse_string_value(value_str));
                    }
                }
                "dependencies" => {
                    manifest
                        .dependencies
                        .insert(key, parse_string_value(value_str));
                }
                "modules"
                    // Using module = "name"
                    if key == "module" => {
                        manifest.modules.push(parse_string_value(value_str));
                    }
                "capabilities" => {
                    manifest
                        .capabilities
                        .insert(key, parse_array_value(value_str));
                }
                "features" => {
                    manifest.features.insert(key, parse_array_value(value_str));
                }
                "build_targets"
                    if key == "target" => {
                        manifest.build_targets.push(parse_string_value(value_str));
                    }
                _ => {}
            }
        }
    }

    Ok(manifest)
}

fn parse_string_value(val: &str) -> String {
    val.trim_matches('"').trim_matches('\'').to_string()
}

fn parse_array_value(val: &str) -> Vec<String> {
    if val == "false" {
        return vec![];
    }
    if val == "true" {
        return vec!["true".to_string()];
    }
    if val.starts_with('[') && val.ends_with(']') {
        let inner = &val[1..val.len() - 1];
        if inner.trim().is_empty() {
            return vec![];
        }
        return inner
            .split(',')
            .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    vec![parse_string_value(val)]
}

pub fn load_manifest(path: &Path) -> Result<OmniManifest, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read manifest: {}", e))?;
    parse_manifest(&content)
}
