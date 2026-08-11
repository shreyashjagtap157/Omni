//! Project build-script boundary.
//!
//! Edition 1 requires hermetic, declared-input build actions. The historical
//! `build.omni` evaluator ran arbitrary compile-time source without a qualified
//! capability/input/output contract, so the current qualified Omni release rejects build scripts rather
//! than pretending they are reproducible.

use crate::package::omni_toml_parser::OmniManifest;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub target_os: String,
    pub target_arch: String,
    pub active_features: Vec<String>,
    pub manifest_dir: PathBuf,
}

impl BuildConfig {
    pub fn new(manifest_dir: PathBuf, manifest: &OmniManifest) -> Self {
        let mut active_features: Vec<String> = manifest.features.keys().cloned().collect();
        active_features.sort();
        Self {
            target_os: std::env::consts::OS.to_string(),
            target_arch: std::env::consts::ARCH.to_string(),
            active_features,
            manifest_dir,
        }
    }
}

pub fn run_build_script(config: &BuildConfig) -> Result<(), String> {
    let build_script_path = config.manifest_dir.join("build.omni");
    if !build_script_path.exists() {
        return Ok(());
    }

    Err(format!(
        "build.omni is not qualified in Omni v{}; refusing to execute an unhermetic build script at {} (hermetic declared-input build actions are scheduled for the package/build milestone)",
        crate::version::PROJECT_VERSION,
        build_script_path.display()
    ))
}
