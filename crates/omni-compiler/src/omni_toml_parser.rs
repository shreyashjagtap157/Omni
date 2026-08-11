use std::path::Path;

/// Represents a capability declaration in omni.toml.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDecl {
    /// The capability kind (e.g., "network", "filesystem", "subprocess").
    pub kind: String,
    /// Permissions granted. For boolean capabilities, empty vec means `true`,
    /// non-empty means specific permissions (e.g., ["read", "write", "/tmp"]).
    pub permissions: Vec<String>,
    /// If false, the capability is explicitly denied.
    pub enabled: bool,
}

/// A build target configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildTarget {
    /// Target triple (e.g., "x86_64-unknown-linux-gnu").
    pub triple: String,
    /// Optimization level (0, 1, 2, 3, s, z).
    pub opt_level: String,
    /// Whether debug info is enabled.
    pub debug: bool,
    /// Features enabled for this target.
    pub features: Vec<String>,
}

/// An edition of the Omni language.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Edition {
    #[default]
    E2026,
    E2027,
    Custom(String),
}

impl std::str::FromStr for Edition {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "2026" => Edition::E2026,
            "2027" => Edition::E2027,
            other => Edition::Custom(other.to_string()),
        })
    }
}

impl Edition {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "2026" => Edition::E2026,
            "2027" => Edition::E2027,
            other => Edition::Custom(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Edition::E2026 => "2026",
            Edition::E2027 => "2027",
            Edition::Custom(s) => s,
        }
    }
}

/// A feature flag declaration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FeatureDecl {
    /// Name of the feature.
    pub name: String,
    /// Features this feature enables (transitive).
    pub enables: Vec<String>,
    /// Optional description.
    pub description: Option<String>,
}

#[derive(Debug, Default)]
pub struct OmniManifest {
    pub name: String,
    pub version: String,
    pub edition: Edition,
    pub dependencies: Vec<Dependency>,
    pub modules: Vec<String>,
    pub capabilities: Vec<CapabilityDecl>,
    pub features: Vec<FeatureDecl>,
    pub build_targets: Vec<BuildTarget>,
}

#[derive(Debug)]
pub struct Dependency {
    pub name: String,
    pub version_req: String,
    pub path: Option<String>,
}

/// Parse a full omni.toml manifest with all Phase 4 fields.
/// Supports:
/// ```toml
/// name = "my-project"
/// version = "0.1.0"
/// edition = "2026"
///
/// [dependencies]
/// foo = "0.1.0"
/// bar = { version = "0.2.0", path = "../bar" }
///
/// [modules]
/// module = "utils"
/// module = "core"
///
/// [capabilities]
/// network = ["read"]
/// filesystem = ["read", "write", "/tmp"]
/// subprocess = false
///
/// [features]
/// feature = "async"
/// feature = "simd"
///
/// [build_targets]
/// target = "x86_64-unknown-linux-gnu"
/// target = "wasm32-unknown-unknown"
/// ```
pub fn parse_omni_toml(path: &Path) -> Result<OmniManifest, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    parse_omni_toml_content(&content)
}

/// Parse omni.toml from a string (useful for testing).
pub fn parse_omni_toml_content(content: &str) -> Result<OmniManifest, String> {
    let mut manifest = OmniManifest::default();
    let mut in_capabilities = false;
    let mut in_modules = false;
    let mut in_dependencies = false;
    let mut in_features = false;
    let mut in_build_targets = false;

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Check for section headers
        if line.starts_with('[') && line.ends_with(']') {
            let section = line[1..line.len() - 1].trim();
            in_dependencies = section == "dependencies";
            in_modules = section == "modules";
            in_capabilities = section == "capabilities";
            in_features = section == "features";
            in_build_targets = section == "build_targets";
            continue;
        }

        if in_dependencies {
            parse_dependency_line(line, &mut manifest.dependencies, line_num)?;
        } else if in_modules {
            parse_module_line(line, &mut manifest.modules, line_num)?;
        } else if in_capabilities {
            parse_capability_line(line, &mut manifest.capabilities, line_num)?;
        } else if in_features {
            parse_feature_line(line, &mut manifest.features, line_num)?;
        } else if in_build_targets {
            parse_build_target_line(line, &mut manifest.build_targets, line_num)?;
        } else {
            // Top-level key=value
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim();
                let value = line[eq_pos + 1..].trim();

                match key {
                    "name" => manifest.name = unquote(value),
                    "version" => manifest.version = unquote(value),
                    "edition" => manifest.edition = Edition::from_str(&unquote(value)),
                    _ => {}
                }
            }
        }
    }

    Ok(manifest)
}

fn unquote(s: &str) -> String {
    s.trim_matches('"').trim_matches('\'').to_string()
}

fn parse_dependency_line(
    line: &str,
    deps: &mut Vec<Dependency>,
    _line_num: usize,
) -> Result<(), String> {
    if let Some(eq_pos) = line.find('=') {
        let name = line[..eq_pos].trim();
        let value = line[eq_pos + 1..].trim();

        if value.starts_with('{') {
            let mut dep = Dependency {
                name: name.to_string(),
                version_req: String::new(),
                path: None,
            };

            let inner = &value[1..value.len().saturating_sub(1)];
            for part in inner.split(',') {
                let part = part.trim();
                if let Some(eq_pos) = part.find('=') {
                    let key = part[..eq_pos].trim();
                    let val = unquote(part[eq_pos + 1..].trim());
                    match key {
                        "version" => dep.version_req = val,
                        "path" => dep.path = Some(val),
                        _ => {}
                    }
                }
            }
            deps.push(dep);
        } else {
            let version = unquote(value);
            deps.push(Dependency {
                name: name.to_string(),
                version_req: version,
                path: None,
            });
        }
    }
    Ok(())
}

fn parse_module_line(
    line: &str,
    modules: &mut Vec<String>,
    _line_num: usize,
) -> Result<(), String> {
    if line.starts_with("module") && line.contains('=') {
        let parts: Vec<&str> = line.split('=').collect();
        if parts.len() == 2 {
            let name = unquote(parts[1].trim());
            modules.push(name);
        }
    }
    Ok(())
}

fn parse_capability_line(
    line: &str,
    caps: &mut Vec<CapabilityDecl>,
    _line_num: usize,
) -> Result<(), String> {
    if let Some(eq_pos) = line.find('=') {
        let kind = line[..eq_pos].trim();
        let value = line[eq_pos + 1..].trim();

        if value == "true" {
            caps.push(CapabilityDecl {
                kind: kind.to_string(),
                permissions: vec![],
                enabled: true,
            });
        } else if value == "false" {
            caps.push(CapabilityDecl {
                kind: kind.to_string(),
                permissions: vec![],
                enabled: false,
            });
        } else if value.starts_with('[') && value.ends_with(']') {
            let inner = &value[1..value.len().saturating_sub(1)];
            let permissions: Vec<String> = if inner.trim().is_empty() {
                vec![]
            } else {
                inner.split(',').map(|p| unquote(p.trim())).collect()
            };
            caps.push(CapabilityDecl {
                kind: kind.to_string(),
                permissions,
                enabled: true,
            });
        } else {
            // Bare string value treated as a single permission
            caps.push(CapabilityDecl {
                kind: kind.to_string(),
                permissions: vec![unquote(value)],
                enabled: true,
            });
        }
    }
    Ok(())
}

fn parse_feature_line(
    line: &str,
    features: &mut Vec<FeatureDecl>,
    _line_num: usize,
) -> Result<(), String> {
    if line.starts_with("feature") && line.contains('=') {
        let parts: Vec<&str> = line.split('=').collect();
        if parts.len() == 2 {
            let name = unquote(parts[1].trim());
            features.push(FeatureDecl {
                name,
                enables: vec![],
                description: None,
            });
        }
    }
    Ok(())
}

fn parse_build_target_line(
    line: &str,
    targets: &mut Vec<BuildTarget>,
    _line_num: usize,
) -> Result<(), String> {
    if line.starts_with("target") && line.contains('=') {
        let parts: Vec<&str> = line.split('=').collect();
        if parts.len() == 2 {
            let triple = unquote(parts[1].trim());
            targets.push(BuildTarget {
                triple,
                opt_level: "2".to_string(),
                debug: false,
                features: vec![],
            });
        }
    }
    Ok(())
}

/// Parse an inline module declaration: `mod name { ... }`
/// Returns the module name and the body content.
pub fn parse_inline_module(source: &str) -> Option<(String, String)> {
    let source = source.trim();
    if !source.starts_with("mod ") {
        return None;
    }

    let rest = &source[4..];
    let name_end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')?;
    let name = rest[..name_end].trim().to_string();

    let after_name = rest[name_end..].trim_start();
    if !after_name.starts_with('{') {
        return None;
    }

    let body_start = 1; // skip '{'
    let mut depth = 1;
    let mut body_end = None;

    for (i, c) in after_name[body_start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    body_end = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }

    let body_end = body_end?;
    let body = after_name[body_start..body_end].trim().to_string();
    Some((name, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_manifest() {
        let toml = r#"
name = "my-project"
version = "0.1.0"

[modules]
module = "utils"
module = "core"

[dependencies]
foo = "0.1.0"
bar = { version = "0.2.0", path = "../bar" }
"#;

        let manifest = parse_omni_toml_content(toml).unwrap();

        assert_eq!(manifest.name, "my-project");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.modules.len(), 2);
        assert_eq!(manifest.modules[0], "utils");
        assert_eq!(manifest.modules[1], "core");
        assert_eq!(manifest.dependencies.len(), 2);
        assert_eq!(manifest.dependencies[0].name, "foo");
        assert_eq!(manifest.dependencies[0].version_req, "0.1.0");
        assert_eq!(manifest.dependencies[1].name, "bar");
        assert_eq!(manifest.dependencies[1].version_req, "0.2.0");
        assert_eq!(manifest.dependencies[1].path, Some("../bar".to_string()));
    }

    #[test]
    fn test_parse_capabilities() {
        let toml = r#"
name = "my-app"
version = "0.1.0"

[capabilities]
network = ["read"]
filesystem = ["read", "write", "/tmp"]
subprocess = false
"#;

        let manifest = parse_omni_toml_content(toml).unwrap();

        assert_eq!(manifest.capabilities.len(), 3);

        let net = &manifest.capabilities[0];
        assert_eq!(net.kind, "network");
        assert!(net.enabled);
        assert_eq!(net.permissions, vec!["read"]);

        let fs = &manifest.capabilities[1];
        assert_eq!(fs.kind, "filesystem");
        assert!(fs.enabled);
        assert_eq!(fs.permissions, vec!["read", "write", "/tmp"]);

        let sub = &manifest.capabilities[2];
        assert_eq!(sub.kind, "subprocess");
        assert!(!sub.enabled);
        assert!(sub.permissions.is_empty());
    }

    #[test]
    fn test_parse_edition() {
        let toml = r#"
name = "my-app"
version = "0.1.0"
edition = "2026"
"#;

        let manifest = parse_omni_toml_content(toml).unwrap();
        assert_eq!(manifest.edition, Edition::E2026);
        assert_eq!(manifest.edition.as_str(), "2026");
    }

    #[test]
    fn test_parse_features() {
        let toml = r#"
name = "my-app"
version = "0.1.0"

[features]
feature = "async"
feature = "simd"
"#;

        let manifest = parse_omni_toml_content(toml).unwrap();
        assert_eq!(manifest.features.len(), 2);
        assert_eq!(manifest.features[0].name, "async");
        assert_eq!(manifest.features[1].name, "simd");
    }

    #[test]
    fn test_parse_build_targets() {
        let toml = r#"
name = "my-app"
version = "0.1.0"

[build_targets]
target = "x86_64-unknown-linux-gnu"
target = "wasm32-unknown-unknown"
"#;

        let manifest = parse_omni_toml_content(toml).unwrap();
        assert_eq!(manifest.build_targets.len(), 2);
        assert_eq!(manifest.build_targets[0].triple, "x86_64-unknown-linux-gnu");
        assert_eq!(manifest.build_targets[1].triple, "wasm32-unknown-unknown");
    }

    #[test]
    fn test_parse_inline_module() {
        let source = "mod utils { fn helper() { } }";
        let (name, body) = parse_inline_module(source).unwrap();
        assert_eq!(name, "utils");
        assert_eq!(body, "fn helper() { }");
    }

    #[test]
    fn test_parse_inline_module_nested() {
        let source = "mod outer { mod inner { fn foo() { } } }";
        let (name, body) = parse_inline_module(source).unwrap();
        assert_eq!(name, "outer");
        assert!(body.contains("mod inner"));
    }

    #[test]
    fn test_parse_inline_module_invalid() {
        assert!(parse_inline_module("fn foo() { }").is_none());
        assert!(parse_inline_module("mod foo").is_none());
    }

    #[test]
    fn test_edition_from_str() {
        assert_eq!(Edition::from_str("2026"), Edition::E2026);
        assert_eq!(Edition::from_str("2027"), Edition::E2027);
        assert_eq!(
            Edition::from_str("2030"),
            Edition::Custom("2030".to_string())
        );
    }
}
