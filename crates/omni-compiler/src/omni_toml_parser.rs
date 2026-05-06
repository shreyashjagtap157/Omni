use std::path::Path;

#[derive(Debug, Default)]
pub struct OmniManifest {
    pub name: String,
    pub version: String,
    pub dependencies: Vec<Dependency>,
    pub modules: Vec<String>,
}

#[derive(Debug)]
pub struct Dependency {
    pub name: String,
    pub version_req: String,
    pub path: Option<String>,
}

/// Parse a simple omni.toml manifest.
/// Supports:
/// ```toml
/// name = "my-project"
/// version = "0.1.0"
/// 
/// [dependencies]
/// foo = "0.1.0"
/// bar = { version = "0.2.0", path = "../bar" }
/// 
/// [modules]
/// module = "utils"
/// module = "core"
/// ```
pub fn parse_omni_toml(path: &Path) -> Result<OmniManifest, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    
    let mut manifest = OmniManifest::default();
    let mut in_dependencies = false;
    let mut in_modules = false;
    
    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        
        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        // Check for section headers
        if line.starts_with('[') && line.ends_with(']') {
            let section = &line[1..line.len()-1].trim();
            in_dependencies = *section == "dependencies";
            in_modules = *section == "modules";
            continue;
        }
        
        if in_dependencies {
            parse_dependency_line(line, &mut manifest.dependencies, line_num)?;
        } else if in_modules {
            parse_module_line(line, &mut manifest.modules, line_num)?;
        } else {
            // Top-level key=value
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim();
                let value = line[eq_pos+1..].trim().trim_matches('"').trim_matches('\'');
                
                match key {
                    "name" => manifest.name = value.to_string(),
                    "version" => manifest.version = value.to_string(),
                    _ => {}
                }
            }
        }
    }
    
    Ok(manifest)
}

fn parse_dependency_line(
    line: &str,
    deps: &mut Vec<Dependency>,
    _line_num: usize,
) -> Result<(), String> {
    // Simple format: name = "version" or name = { version = "...", path = "..." }
    if let Some(eq_pos) = line.find('=') {
        let name = line[..eq_pos].trim();
        let value = line[eq_pos+1..].trim();
        
        if value.starts_with('{') {
            // Structured format: name = { version = "...", path = "..." }
            let mut dep = Dependency {
                name: name.to_string(),
                version_req: String::new(),
                path: None,
            };
            
            // Very simple parser for { version = "...", path = "..." }
            let inner = &value[1..value.len()-1];
            for part in inner.split(',') {
                let part = part.trim();
                if let Some(eq_pos) = part.find('=') {
                    let key = part[..eq_pos].trim();
                    let val = part[eq_pos+1..].trim().trim_matches('"').trim_matches('\'');
                    match key {
                        "version" => dep.version_req = val.to_string(),
                        "path" => dep.path = Some(val.to_string()),
                        _ => {}
                    }
                }
            }
            deps.push(dep);
        } else {
            // Simple format: name = "version"
            let version = value.trim_matches('"').trim_matches('\'');
            deps.push(Dependency {
                name: name.to_string(),
                version_req: version.to_string(),
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
    // Format: module = "name"
    if line.starts_with("module") && line.contains('=') {
        let parts: Vec<&str> = line.split('=').collect();
        if parts.len() == 2 {
            let name = parts[1].trim().trim_matches('"').trim_matches('\'');
            modules.push(name.to_string());
        }
    }
    Ok(())
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
        
        let mut manifest = OmniManifest::default();
        let mut in_deps = false;
        let mut in_mods = false;
        
        for line in toml.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { continue; }
            
            if line.starts_with('[') && line.ends_with(']') {
                let section = &line[1..line.len()-1].trim();
                in_deps = *section == "dependencies";
                in_mods = *section == "modules";
                continue;
            }
            
            if in_mods {
                parse_module_line(line, &mut manifest.modules, 0).unwrap();
            } else if in_deps {
                parse_dependency_line(line, &mut manifest.dependencies, 0).unwrap();
            } else {
                if let Some(eq_pos) = line.find('=') {
                    let key = line[..eq_pos].trim();
                    let value = line[eq_pos+1..].trim().trim_matches('"').trim_matches('\'');
                    match key {
                        "name" => manifest.name = value.to_string(),
                        "version" => manifest.version = value.to_string(),
                        _ => {}
                    }
                }
            }
        }
        
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
}
