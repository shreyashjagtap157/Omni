use crate::ast::Program;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A simple module system that uses `omni.toml` manifest.
/// Modules are declared in `omni.toml` under `[modules]` section.
pub struct ModuleSystem {
    /// All loaded modules: path -> parsed Program
    pub modules: HashMap<PathBuf, Program>,
    /// Declared modules from omni.toml
    pub module_names: Vec<String>,
}

impl ModuleSystem {
    pub fn new() -> Self {
        ModuleSystem {
            modules: HashMap::new(),
            module_names: Vec::new(),
        }
    }

    /// Load modules declared in the manifest.
    pub fn load_manifest_modules(&mut self, source_path: &Path) -> Result<(), String> {
        let dir = source_path
            .parent()
            .ok_or_else(|| "Cannot determine parent directory".to_string())?;

        // Look for omni.toml in the same directory
        let manifest_path = dir.join("omni.toml");
        if !manifest_path.exists() {
            return Ok(()); // No manifest, no modules
        }

        // Parse the manifest (simple key=value format)
        let content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read omni.toml: {}", e))?;

        // Very simple parser: look for lines like `module = "foo"`
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("module") && line.contains('=') {
                let parts: Vec<&str> = line.split('=').collect();
                if parts.len() == 2 {
                    let name = parts[1].trim().trim_matches('"').trim_matches('\'');
                    self.module_names.push(name.to_string());
                }
            }
        }

        // Load each module
        for name in &self.module_names.clone() {
            self.load_module(source_path, name)?;
        }

        Ok(())
    }

    /// Load a single module file.
    fn load_module(&mut self, source_path: &Path, mod_name: &str) -> Result<PathBuf, String> {
        let dir = source_path
            .parent()
            .ok_or_else(|| "Cannot determine parent directory".to_string())?;

        let candidate = dir.join(format!("{}.omni", mod_name));

        if !candidate.exists() {
            return Err(format!(
                "Module '{}' not found (looked for '{}')",
                mod_name,
                candidate.display()
            ));
        }

        if self.modules.contains_key(&candidate) {
            return Ok(candidate);
        }

        let program = crate::parse_file(&candidate)?;
        self.modules.insert(candidate.clone(), program);

        Ok(candidate)
    }

    /// Get all loaded module programs.
    pub fn get_all_programs(&self) -> Vec<Program> {
        self.modules.values().cloned().collect()
    }
}

/// Initialize module system from a source file.
pub fn init_module_system(source_path: &Path) -> Result<ModuleSystem, String> {
    let mut ms = ModuleSystem::new();
    ms.load_manifest_modules(source_path)?;
    Ok(ms)
}
