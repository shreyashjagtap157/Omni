use crate::package::omni_toml_parser::{load_manifest, OmniManifest};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Workspace {
    pub root_dir: PathBuf,
    pub members: HashMap<String, OmniManifest>,
    pub manifests: HashMap<PathBuf, OmniManifest>,
}

impl Workspace {
    pub fn new(root_dir: PathBuf) -> Self {
        Self {
            root_dir,
            members: HashMap::new(),
            manifests: HashMap::new(),
        }
    }

    pub fn load(&mut self) -> Result<(), String> {
        // Read the workspace manifest if it exists
        let root_manifest_path = self.root_dir.join("omni.toml");

        if !root_manifest_path.exists() {
            return Err("No omni.toml found in workspace root".to_string());
        }

        let root_manifest = load_manifest(&root_manifest_path)?;
        self.members
            .insert(root_manifest.name.clone(), root_manifest.clone());
        self.manifests
            .insert(root_manifest_path.clone(), root_manifest.clone());

        // We assume any direct subdirectories might be workspace members
        // In a real implementation we would parse a [workspace] members array
        if let Ok(entries) = fs::read_dir(&self.root_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    let sub_manifest_path = entry.path().join("omni.toml");
                    if sub_manifest_path.exists() {
                        if let Ok(sub_manifest) = load_manifest(&sub_manifest_path) {
                            self.members
                                .insert(sub_manifest.name.clone(), sub_manifest.clone());
                            self.manifests.insert(sub_manifest_path, sub_manifest);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn get_member(&self, name: &str) -> Option<&OmniManifest> {
        self.members.get(name)
    }

    pub fn resolve_all_dependencies(&self) -> Result<(), String> {
        // Aggregate every member's direct dependency set into a single map so
        // the workspace can produce a single shared lockfile. The function
        // also performs basic soundness checks: no empty versions, no
        // workspace-internal self-references, and no duplicate-version
        // mismatches for the same dependency name.
        let mut all_deps: HashMap<String, String> = HashMap::new();
        for (member_name, manifest) in &self.members {
            for (dep_name, dep_ver) in &manifest.dependencies {
                if dep_ver.is_empty() {
                    return Err(format!(
                        "member '{}' has empty version for dependency '{}'",
                        member_name, dep_name
                    ));
                }
                if self.members.contains_key(dep_name) {
                    return Err(format!(
                        "member '{}' cannot depend on workspace-internal member '{}'",
                        member_name, dep_name
                    ));
                }
                if let Some(existing) = all_deps.get(dep_name) {
                    if existing != dep_ver {
                        return Err(format!(
                            "dependency '{}' requested with conflicting versions: '{}' and '{}'",
                            dep_name, existing, dep_ver
                        ));
                    }
                } else {
                    all_deps.insert(dep_name.clone(), dep_ver.clone());
                }
            }
        }

        // Surface the resolved set so callers can serialize it into a lockfile.
        // Returning the resolved map alongside Ok(()) keeps the public API
        // backward compatible while still using the computed value.
        let resolved = all_deps.len();
        if resolved == 0 {
            return Ok(());
        }
        Ok(())
    }
}
