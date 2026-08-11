//! Bootstrap verification infrastructure.
//!
//! v0.1.4 is still Rust-hosted.  This crate can build and smoke-test Stage-0,
//! but it MUST NOT claim Stage-1/Stage-2 self-hosting until the Omni-written
//! compiler exists and emits an executable compiler artifact.

use std::path::Path;
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SelfHostError {
    #[error("Stage0 build failed: {0}")]
    Stage0Failed(String),
    #[error("Self-hosting is not qualified at this milestone: {0}")]
    NotQualified(String),
    #[error("Compilation failed: {0}")]
    CompileFailed(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub struct StageArtifact {
    pub path: std::path::PathBuf,
    pub hash: String,
    pub lir_output: String,
}

fn workspace_root() -> std::path::PathBuf {
    let mut path = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    path.pop();
    path.pop();
    path
}

fn hash_file(path: &Path) -> Result<String, SelfHostError> {
    use sha2::{Digest, Sha256};
    let data = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Build the Rust Stage-0 bootstrap compiler.
pub fn build_stage0() -> Result<StageArtifact, SelfHostError> {
    let workspace_root = workspace_root();
    let output = Command::new("cargo")
        .args(["build", "--locked", "-p", "omni-stage0"])
        .current_dir(&workspace_root)
        .output()?;

    if !output.status.success() {
        return Err(SelfHostError::Stage0Failed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let path = workspace_root
        .join("target")
        .join("debug")
        .join(format!("omni{}", std::env::consts::EXE_SUFFIX));
    if !path.is_file() {
        return Err(SelfHostError::Stage0Failed(format!(
            "cargo succeeded but Stage-0 binary was not found at {}",
            path.display()
        )));
    }
    let hash = hash_file(&path)?;

    Ok(StageArtifact {
        path,
        hash,
        lir_output: String::new(),
    })
}

/// Use Stage-0 to emit LIR for a source file.  This is a compiler smoke helper,
/// not evidence of self-hosting.
pub fn compile_with_stage(stage_exe: &Path, source_file: &str) -> Result<String, SelfHostError> {
    let workspace_root = workspace_root();
    let source_path = workspace_root.join(source_file);
    let output = Command::new(stage_exe)
        .args([
            "emit-lir",
            source_path.to_str().ok_or_else(|| {
                SelfHostError::CompileFailed("source path is not valid UTF-8".to_string())
            })?,
        ])
        .current_dir(&workspace_root)
        .output()?;

    if !output.status.success() {
        return Err(SelfHostError::CompileFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn build_stage1(_stage0: &StageArtifact) -> Result<StageArtifact, SelfHostError> {
    Err(SelfHostError::NotQualified(
        "Stage-1 requires an Omni-written compiler source that Stage-0 can compile into a compiler executable"
            .to_string(),
    ))
}

pub fn build_stage2(_stage1: &StageArtifact) -> Result<StageArtifact, SelfHostError> {
    Err(SelfHostError::NotQualified(
        "Stage-2 requires the Stage-1 Omni compiler to compile the same compiler sources independently"
            .to_string(),
    ))
}

pub fn compare_stages(
    _stage1: &StageArtifact,
    _stage2: &StageArtifact,
) -> Result<(), SelfHostError> {
    Err(SelfHostError::NotQualified(
        "stage convergence cannot be asserted before real Stage-1 and Stage-2 compiler artifacts exist"
            .to_string(),
    ))
}

pub fn run_self_host_pipeline() -> Result<(), SelfHostError> {
    Err(SelfHostError::NotQualified(
        "v0.1.4 is a Rust bootstrap. Self-hosting begins only after the Omni compiler can compile its own implementation; see docs/VERSIONING_AND_BOOTSTRAP_PLAN.md"
            .to_string(),
    ))
}

/// Real Stage-0 smoke verification: build the Rust bootstrap and run the native
/// hello example through the canonical `omni run` AOT path.
pub fn verify_stage0_works() -> Result<(), SelfHostError> {
    let stage0 = build_stage0()?;
    let workspace_root = workspace_root();
    let output = Command::new(&stage0.path)
        .args(["run", "examples/native_hello.omni"])
        .current_dir(&workspace_root)
        .output()?;

    if !output.status.success() {
        return Err(SelfHostError::Stage0Failed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains("Hello, Omni!") {
        Ok(())
    } else {
        Err(SelfHostError::Stage0Failed(format!(
            "unexpected Stage-0 hello output: {stdout:?}"
        )))
    }
}
