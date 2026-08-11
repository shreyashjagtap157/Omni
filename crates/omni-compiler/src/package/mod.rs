pub mod build_graph;
pub mod build_script;
pub mod lockfile;
pub mod omni_toml_parser;
pub mod omni_workspace;
pub mod solver;

use lockfile::Lockfile;
use omni_toml_parser::OmniManifest;
// The PubGrub solver and registry abstraction remain available in `solver`,
// but v0.1.4 intentionally has no trusted remote/local registry provider.
// Project preparation therefore fails closed when external dependencies exist.
use std::path::Path;

pub fn resolve_and_write_lockfile(manifest: &OmniManifest, out_path: &Path) -> Result<(), String> {
    if !manifest.dependencies.is_empty() {
        let mut names: Vec<_> = manifest.dependencies.keys().cloned().collect();
        names.sort();
        return Err(format!(
            "dependency resolution is not qualified in Omni v0.1.4; refusing to invent registry versions for: {}",
            names.join(", ")
        ));
    }

    let lockfile = Lockfile::new(Default::default());
    lockfile
        .write_to_file(out_path)
        .map_err(|e| format!("Failed to write lockfile: {}", e))?;
    Ok(())
}
