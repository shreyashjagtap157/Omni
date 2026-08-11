use crate::ast::{Program, Visibility};
use crate::omni_toml_parser::{parse_omni_toml_content, OmniManifest};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Represents a module in the hierarchical module system.
#[derive(Debug, Clone)]
pub struct Module {
    /// Fully qualified module path (e.g., "core::utils::helpers").
    pub path: String,
    /// The module name (last segment of path).
    pub name: String,
    /// Parent module path, if any.
    pub parent: Option<String>,
    /// Whether this module is from a file (true) or inline (false).
    pub is_file_module: bool,
    /// The source file path, if this is a file module.
    pub file_path: Option<PathBuf>,
    /// The parsed program (AST), if loaded.
    pub program: Option<Program>,
    /// Visibility of the module.
    pub visibility: Visibility,
    /// Child modules.
    pub children: Vec<String>,
}

/// Represents a symbol (function, struct, etc.) with its visibility.
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    /// Name of the symbol.
    pub name: String,
    /// Module that owns this symbol.
    pub module_path: String,
    /// Visibility of the symbol.
    pub visibility: Visibility,
    /// Whether this is a type (struct/enum) vs. a function.
    pub is_type: bool,
}

/// The hierarchical module system supporting file modules, inline modules,
/// and all visibility levels.
pub struct ModuleSystem {
    /// All loaded modules indexed by their fully qualified path.
    pub modules: HashMap<String, Module>,
    /// Symbols indexed by name, with module info for visibility checks.
    pub symbols: HashMap<String, Vec<SymbolInfo>>,
    /// The parsed manifest.
    pub manifest: Option<OmniManifest>,
    /// Root directory for resolving file modules.
    pub root_dir: PathBuf,
    /// Module dependency graph: module_path -> list of module_paths it depends on.
    pub dependency_graph: HashMap<String, Vec<String>>,
}

impl ModuleSystem {
    pub fn new() -> Self {
        ModuleSystem {
            modules: HashMap::new(),
            symbols: HashMap::new(),
            manifest: None,
            root_dir: PathBuf::new(),
            dependency_graph: HashMap::new(),
        }
    }

    /// Load modules from an omni.toml manifest file.
    pub fn load_manifest(&mut self, manifest_path: &Path) -> Result<(), String> {
        let content = std::fs::read_to_string(manifest_path)
            .map_err(|e| format!("Failed to read manifest: {}", e))?;

        let manifest = parse_omni_toml_content(&content)?;
        self.root_dir = manifest_path
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf();
        self.manifest = Some(manifest);

        // Load file modules declared in the manifest
        let modules_and_name = if let Some(ref m) = self.manifest {
            m.modules
                .clone()
                .into_iter()
                .map(|mod_name| (mod_name, m.name.clone()))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        for (mod_name, pkg_name) in &modules_and_name {
            self.load_file_module(mod_name, pkg_name)?;
        }

        Ok(())
    }

    /// Load a file module: one file = one module.
    /// Looks for `<mod_name>.omni` in the root directory,
    /// or `<mod_name>/mod.omni` for directory-style modules.
    pub fn load_file_module(&mut self, mod_name: &str, package_name: &str) -> Result<(), String> {
        let module_path = if self.modules.contains_key(package_name) {
            format!("{}::{}", package_name, mod_name)
        } else {
            mod_name.to_string()
        };

        // Check if already loaded
        if self.modules.contains_key(&module_path) {
            return Ok(());
        }

        // Try <mod_name>.omni first, then <mod_name>/mod.omni
        let file_candidate = self.root_dir.join(format!("{}.omni", mod_name));
        let dir_candidate = self.root_dir.join(mod_name).join("mod.omni");

        let source_path = if file_candidate.exists() {
            file_candidate
        } else if dir_candidate.exists() {
            dir_candidate
        } else {
            return Err(format!(
                "Module '{}' not found (looked for '{}' or '{}')",
                mod_name,
                file_candidate.display(),
                dir_candidate.display()
            ));
        };

        let program = crate::parser_utils::parse_file(&source_path).map_err(|e| e.to_string())?;

        // Extract top-level symbols and register them
        let symbols = extract_symbols(&program, &module_path);
        for sym in &symbols {
            self.symbols
                .entry(sym.name.clone())
                .or_default()
                .push(sym.clone());
        }

        let module = Module {
            path: module_path.clone(),
            name: mod_name.to_string(),
            parent: if module_path.contains("::") {
                Some(
                    module_path
                        .rsplit_once("::")
                        .map(|(p, _)| p.to_string())
                        .unwrap_or_default(),
                )
            } else {
                None
            },
            is_file_module: true,
            file_path: Some(source_path.clone()),
            program: Some(program),
            visibility: Visibility::Private,
            children: Vec::new(),
        };

        self.modules.insert(module_path, module);
        Ok(())
    }

    /// Register an inline module: `mod name { ... }`
    pub fn register_inline_module(
        &mut self,
        parent_path: &str,
        name: &str,
        body: &str,
    ) -> Result<(), String> {
        let module_path = if parent_path.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", parent_path, name)
        };

        // Parse the body as a program
        let tokens = crate::complete_lexer::tokenize_complete(body)
            .map_err(|e| format!("Failed to lex inline module '{}': {}", name, e))?;
        let mut parser = crate::parser::Parser::new(tokens);
        let program = parser
            .parse_program()
            .map_err(|e| format!("Failed to parse inline module '{}': {}", name, e))?;

        // Extract symbols
        let symbols = extract_symbols(&program, &module_path);
        for sym in &symbols {
            self.symbols
                .entry(sym.name.clone())
                .or_default()
                .push(sym.clone());
        }

        let module = Module {
            path: module_path.clone(),
            name: name.to_string(),
            parent: if !parent_path.is_empty() {
                Some(parent_path.to_string())
            } else {
                None
            },
            is_file_module: false,
            file_path: None,
            program: Some(program),
            visibility: Visibility::Private,
            children: Vec::new(),
        };

        // Register as child of parent
        if let Some(parent) = self.modules.get_mut(parent_path) {
            parent.children.push(module_path.clone());
        }

        self.modules.insert(module_path, module);
        Ok(())
    }

    /// Check if a symbol is accessible from the given module context.
    /// Implements all visibility levels:
    /// - Private: only accessible within the same module
    /// - PubMod: accessible within the parent module and its children
    /// - PubPkg: accessible within the same package
    /// - Pub: accessible everywhere
    /// - PubCap(cap): accessible only if the requesting module has the capability
    /// - PubFriend(path): accessible only to the specified friend module
    pub fn is_symbol_accessible(
        &self,
        symbol: &SymbolInfo,
        requesting_module: &str,
        _capabilities: &[String],
    ) -> bool {
        match &symbol.visibility {
            Visibility::Private => {
                // Accessible only within the same module
                symbol.module_path == requesting_module
                    || requesting_module.starts_with(&format!("{}::", symbol.module_path))
            }
            Visibility::PubMod => {
                // Accessible within the parent module and its children
                if let Some(parent) = &self
                    .modules
                    .get(&symbol.module_path)
                    .and_then(|m| m.parent.clone())
                {
                    requesting_module == parent
                        || requesting_module.starts_with(&format!("{}::", parent))
                        || symbol.module_path == requesting_module
                } else {
                    // Top-level pub(mod) is same as pub
                    true
                }
            }
            Visibility::PubPkg => {
                // Accessible within the same package (same root module)
                let sym_pkg = symbol.module_path.split("::").next().unwrap_or("");
                let req_pkg = requesting_module.split("::").next().unwrap_or("");
                sym_pkg == req_pkg
            }
            Visibility::Pub => true,
            Visibility::PubCap(cap) => {
                // Accessible only if the requesting module has the capability
                // For now, check if the capability name is in the requesting module's capabilities
                _capabilities.iter().any(|c| c == cap) || cap.is_empty() // Empty cap means public
            }
            Visibility::PubFriend(friend_path) => {
                // Accessible only to the specified friend module
                requesting_module == friend_path
                    || requesting_module.starts_with(&format!("{}::", friend_path))
            }
        }
    }

    /// Resolve a symbol name to its SymbolInfo, checking accessibility.
    pub fn resolve_symbol(
        &self,
        name: &str,
        requesting_module: &str,
        capabilities: &[String],
    ) -> Option<&SymbolInfo> {
        let candidates = self.symbols.get(name)?;
        candidates
            .iter()
            .find(|&sym| self.is_symbol_accessible(sym, requesting_module, capabilities))
            .map(|v| v as _)
    }

    /// Get all programs from loaded modules (for multi-file compilation).
    pub fn get_all_programs(&self) -> Vec<(&String, &Program)> {
        self.modules
            .iter()
            .filter_map(|(path, module)| module.program.as_ref().map(|prog| (path, prog)))
            .collect()
    }

    /// Build a dependency graph from module imports/use declarations.
    pub fn build_dependency_graph(&mut self) {
        self.dependency_graph.clear();

        for (module_path, module) in &self.modules {
            if let Some(ref program) = module.program {
                let deps = extract_module_dependencies(program);
                self.dependency_graph.insert(module_path.clone(), deps);
            }
        }
    }

    /// Get modules in topological order (dependencies first).
    pub fn topological_order(&self) -> Result<Vec<String>, String> {
        let mut visited = HashMap::<String, bool>::new();
        let mut order = Vec::new();

        for module_path in self.modules.keys() {
            if !visited.get(module_path).copied().unwrap_or(false) {
                self.visit_module(module_path, &mut visited, &mut order)?;
            }
        }

        Ok(order)
    }

    fn visit_module(
        &self,
        path: &str,
        visited: &mut HashMap<String, bool>,
        order: &mut Vec<String>,
    ) -> Result<(), String> {
        if visited.get(path).copied().unwrap_or(false) {
            return Ok(());
        }

        // Mark as visiting (for cycle detection)
        visited.insert(path.to_string(), true);

        // Visit dependencies first
        if let Some(deps) = self.dependency_graph.get(path) {
            for dep in deps {
                if self.modules.contains_key(dep) {
                    self.visit_module(dep, visited, order)?;
                }
            }
        }

        order.push(path.to_string());
        Ok(())
    }

    /// Get the module path for a given file path.
    pub fn module_for_file(&self, file_path: &Path) -> Option<String> {
        for (path, module) in &self.modules {
            if let Some(ref fp) = module.file_path {
                if fp == file_path {
                    return Some(path.clone());
                }
            }
        }
        None
    }
}

/// Extract top-level symbols from a program.
fn extract_symbols(program: &Program, module_path: &str) -> Vec<SymbolInfo> {
    let mut symbols = Vec::new();

    for stmt in &program.stmts {
        match stmt {
            crate::ast::Stmt::Fn {
                name, visibility, ..
            } => {
                symbols.push(SymbolInfo {
                    name: name.clone(),
                    module_path: module_path.to_string(),
                    visibility: visibility.clone(),
                    is_type: false,
                });
            }
            crate::ast::Stmt::Struct {
                name, visibility, ..
            }
            | crate::ast::Stmt::Enum {
                name, visibility, ..
            }
            | crate::ast::Stmt::Trait {
                name, visibility, ..
            }
            | crate::ast::Stmt::TypeAlias {
                name, visibility, ..
            } => {
                symbols.push(SymbolInfo {
                    name: name.clone(),
                    module_path: module_path.to_string(),
                    visibility: visibility.clone(),
                    is_type: true,
                });
            }
            _ => {}
        }
    }

    symbols
}

/// Extract module dependencies from use declarations.
fn extract_module_dependencies(program: &Program) -> Vec<String> {
    let mut deps = Vec::new();

    for stmt in &program.stmts {
        if let crate::ast::Stmt::Use { path, .. } = stmt {
            // Extract the first segment as the dependency module
            if let Some(first) = path.split("::").next() {
                let dep = first.to_string();
                if !deps.contains(&dep) {
                    deps.push(dep);
                }
            }
        }
        if let crate::ast::Stmt::UseScoped { path, .. } = stmt {
            if let Some(first) = path.split("::").next() {
                let dep = first.to_string();
                if !deps.contains(&dep) {
                    deps.push(dep);
                }
            }
        }
    }

    deps
}

fn nearest_manifest(source_path: &Path) -> Option<PathBuf> {
    let start = source_path.parent()?;
    for dir in start.ancestors() {
        let candidate = dir.join("omni.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Initialize module system from a source file.
pub fn init_module_system(source_path: &Path) -> Result<ModuleSystem, String> {
    let mut ms = ModuleSystem::new();

    let dir = source_path
        .parent()
        .ok_or_else(|| "Cannot determine parent directory".to_string())?;

    if let Some(manifest_path) = nearest_manifest(source_path) {
        ms.load_manifest(&manifest_path)?;
    } else {
        ms.root_dir = dir.to_path_buf();
    }

    // Also try to load the source file itself as a module
    let source_name = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");

    let program = crate::parser_utils::parse_file(source_path).map_err(|e| e.to_string())?;
    let symbols = extract_symbols(&program, source_name);
    for sym in &symbols {
        ms.symbols
            .entry(sym.name.clone())
            .or_default()
            .push(sym.clone());
    }

    ms.modules.insert(
        source_name.to_string(),
        Module {
            path: source_name.to_string(),
            name: source_name.to_string(),
            parent: None,
            is_file_module: true,
            file_path: Some(source_path.to_path_buf()),
            program: Some(program),
            visibility: Visibility::Private,
            children: Vec::new(),
        },
    );

    Ok(ms)
}

impl Default for ModuleSystem {
    fn default() -> Self {
        ModuleSystem::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Stmt, Visibility};
    use crate::diagnostics::Span;

    fn make_span() -> Span {
        Span::default()
    }

    #[test]
    fn test_module_system_new() {
        let ms = ModuleSystem::new();
        assert!(ms.modules.is_empty());
        assert!(ms.symbols.is_empty());
        assert!(ms.manifest.is_none());
    }

    #[test]
    fn test_extract_symbols() {
        let program = Program {
            stmts: vec![
                Stmt::Fn {
                    name: "pub_fn".to_string(),
                    visibility: Visibility::Pub,
                    is_async: false,
                    type_params: vec![],
                    params: vec![],
                    ret_type: None,
                    effects: vec![],
                    contracts: vec![],
                    body: vec![],
                    span: make_span(),
                },
                Stmt::Struct {
                    name: "MyStruct".to_string(),
                    visibility: Visibility::PubPkg,
                    fields: vec![],
                    is_linear: false,
                    span: make_span(),
                },
                Stmt::Fn {
                    name: "private_fn".to_string(),
                    visibility: Visibility::Private,
                    is_async: false,
                    type_params: vec![],
                    params: vec![],
                    ret_type: None,
                    effects: vec![],
                    contracts: vec![],
                    body: vec![],
                    span: make_span(),
                },
            ],
        };

        let symbols = extract_symbols(&program, "test_mod");
        assert_eq!(symbols.len(), 3);

        assert_eq!(symbols[0].name, "pub_fn");
        assert_eq!(symbols[0].visibility, Visibility::Pub);
        assert!(!symbols[0].is_type);

        assert_eq!(symbols[1].name, "MyStruct");
        assert_eq!(symbols[1].visibility, Visibility::PubPkg);
        assert!(symbols[1].is_type);

        assert_eq!(symbols[2].name, "private_fn");
        assert_eq!(symbols[2].visibility, Visibility::Private);
    }

    #[test]
    fn test_visibility_private_same_module() {
        let ms = ModuleSystem::new();

        let sym = SymbolInfo {
            name: "secret".to_string(),
            module_path: "my_mod".to_string(),
            visibility: Visibility::Private,
            is_type: false,
        };

        // Same module should have access
        assert!(ms.is_symbol_accessible(&sym, "my_mod", &[]));
        // Child module should have access
        assert!(ms.is_symbol_accessible(&sym, "my_mod::child", &[]));
        // Different module should NOT have access
        assert!(!ms.is_symbol_accessible(&sym, "other_mod", &[]));
    }

    #[test]
    fn test_visibility_pub() {
        let ms = ModuleSystem::new();

        let sym = SymbolInfo {
            name: "public_fn".to_string(),
            module_path: "my_mod".to_string(),
            visibility: Visibility::Pub,
            is_type: false,
        };

        assert!(ms.is_symbol_accessible(&sym, "my_mod", &[]));
        assert!(ms.is_symbol_accessible(&sym, "other_mod", &[]));
        assert!(ms.is_symbol_accessible(&sym, "pkg::other", &[]));
    }

    #[test]
    fn test_visibility_pub_pkg() {
        let ms = ModuleSystem::new();

        let sym = SymbolInfo {
            name: "pkg_fn".to_string(),
            module_path: "mypkg::internal".to_string(),
            visibility: Visibility::PubPkg,
            is_type: false,
        };

        // Same package should have access
        assert!(ms.is_symbol_accessible(&sym, "mypkg::other", &[]));
        assert!(ms.is_symbol_accessible(&sym, "mypkg", &[]));
        // Different package should NOT have access
        assert!(!ms.is_symbol_accessible(&sym, "otherpkg::mod", &[]));
    }

    #[test]
    fn test_visibility_pub_cap() {
        let ms = ModuleSystem::new();

        let sym = SymbolInfo {
            name: "cap_fn".to_string(),
            module_path: "secure_mod".to_string(),
            visibility: Visibility::PubCap("network".to_string()),
            is_type: false,
        };

        // With capability should have access
        assert!(ms.is_symbol_accessible(&sym, "client_mod", &["network".to_string()]));
        // Without capability should NOT have access
        assert!(!ms.is_symbol_accessible(&sym, "client_mod", &[]));
        // Empty cap means public
        let sym_empty = SymbolInfo {
            name: "empty_cap".to_string(),
            module_path: "mod".to_string(),
            visibility: Visibility::PubCap("".to_string()),
            is_type: false,
        };
        assert!(ms.is_symbol_accessible(&sym_empty, "any_mod", &[]));
    }

    #[test]
    fn test_visibility_pub_friend() {
        let ms = ModuleSystem::new();

        let sym = SymbolInfo {
            name: "friend_fn".to_string(),
            module_path: "my_mod".to_string(),
            visibility: Visibility::PubFriend("trusted".to_string()),
            is_type: false,
        };

        // Friend should have access
        assert!(ms.is_symbol_accessible(&sym, "trusted", &[]));
        assert!(ms.is_symbol_accessible(&sym, "trusted::child", &[]));
        // Non-friend should NOT have access
        assert!(!ms.is_symbol_accessible(&sym, "stranger", &[]));
    }

    #[test]
    fn test_extract_module_dependencies() {
        let program = Program {
            stmts: vec![
                Stmt::Use {
                    path: "utils::helpers".to_string(),
                    alias: None,
                    span: make_span(),
                },
                Stmt::Use {
                    path: "core::math".to_string(),
                    alias: Some("m".to_string()),
                    span: make_span(),
                },
                Stmt::UseScoped {
                    path: "io::fs".to_string(),
                    aliases: vec![("read_file".to_string(), None)],
                    body: vec![],
                    span: make_span(),
                },
                Stmt::Fn {
                    name: "main".to_string(),
                    visibility: Visibility::Pub,
                    is_async: false,
                    type_params: vec![],
                    params: vec![],
                    ret_type: None,
                    effects: vec![],
                    contracts: vec![],
                    body: vec![],
                    span: make_span(),
                },
            ],
        };

        let deps = extract_module_dependencies(&program);
        assert_eq!(deps.len(), 3);
        assert!(deps.contains(&"utils".to_string()));
        assert!(deps.contains(&"core".to_string()));
        assert!(deps.contains(&"io".to_string()));
    }

    #[test]
    fn test_topological_order_basic() {
        let mut ms = ModuleSystem::new();

        // Create modules manually
        ms.modules.insert(
            "a".to_string(),
            Module {
                path: "a".to_string(),
                name: "a".to_string(),
                parent: None,
                is_file_module: false,
                file_path: None,
                program: None,
                visibility: Visibility::Pub,
                children: vec![],
            },
        );
        ms.modules.insert(
            "b".to_string(),
            Module {
                path: "b".to_string(),
                name: "b".to_string(),
                parent: None,
                is_file_module: false,
                file_path: None,
                program: None,
                visibility: Visibility::Pub,
                children: vec![],
            },
        );

        // b depends on a
        ms.dependency_graph
            .insert("b".to_string(), vec!["a".to_string()]);

        let order = ms.topological_order().unwrap();
        // a should come before b
        let a_idx = order.iter().position(|x| x == "a").unwrap();
        let b_idx = order.iter().position(|x| x == "b").unwrap();
        assert!(a_idx < b_idx, "a should come before b in topological order");
    }

    #[test]
    fn test_resolve_symbol_with_visibility() {
        let mut ms = ModuleSystem::new();

        let pub_sym = SymbolInfo {
            name: "shared".to_string(),
            module_path: "lib".to_string(),
            visibility: Visibility::Pub,
            is_type: false,
        };
        let priv_sym = SymbolInfo {
            name: "hidden".to_string(),
            module_path: "lib".to_string(),
            visibility: Visibility::Private,
            is_type: false,
        };

        ms.symbols.insert("shared".to_string(), vec![pub_sym]);
        ms.symbols.insert("hidden".to_string(), vec![priv_sym]);

        // From another module, should find pub but not priv
        assert!(ms.resolve_symbol("shared", "client", &[]).is_some());
        assert!(ms.resolve_symbol("hidden", "client", &[]).is_none());

        // From same module, should find both
        assert!(ms.resolve_symbol("hidden", "lib", &[]).is_some());
    }

    #[test]
    fn finds_project_manifest_above_src_directory() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("demo");
        let src = project.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            project.join("omni.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
        )
        .unwrap();
        let main = src.join("main.omni");
        std::fs::write(&main, "fn main() -> i64 { return 0; }\n").unwrap();

        assert_eq!(nearest_manifest(&main), Some(project.join("omni.toml")));
        let ms = init_module_system(&main).expect("module system should load project manifest");
        assert_eq!(ms.root_dir, project);
        assert_eq!(ms.manifest.as_ref().map(|m| m.name.as_str()), Some("demo"));
    }
}
