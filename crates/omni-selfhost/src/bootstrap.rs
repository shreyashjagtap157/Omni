use std::path::Path;
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SelfHostError {
    #[error("Stage0 build failed: {0}")]
    Stage0Failed(String),
    #[error("Stage1 build failed: {0}")]
    Stage1Failed(String),
    #[error("Stage2 build failed: {0}")]
    Stage2Failed(String),
    #[error("Stage mismatch: stage1={stage1} != stage2={stage2}")]
    StageMismatch { stage1: String, stage2: String },
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub struct StageArtifact {
    pub path: std::path::PathBuf,
    pub hash: String,
}

fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").to_path_buf()
}

fn hash_file(path: &Path) -> Result<String, SelfHostError> {
    use sha2::{Digest, Sha256};
    let data = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

pub fn build_stage0() -> Result<StageArtifact, SelfHostError> {
    let workspace_root = workspace_root();
    let output = Command::new("cargo")
        .args(["build", "--bin", "omni-stage0"])
        .current_dir(&workspace_root)
        .output()?;

    if !output.status.success() {
        return Err(SelfHostError::Stage0Failed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let path = workspace_root.join("target").join("debug").join(format!(
        "omni-stage0{}",
        std::env::consts::EXE_SUFFIX
    ));
    let hash = hash_file(&path)?;

    Ok(StageArtifact { path, hash })
}

pub fn build_stage1(stage0: &Path) -> Result<StageArtifact, SelfHostError> {
    let workspace_root = workspace_root();
    let output = Command::new(stage0)
        .args(["run", "--", "examples/hello.omni"])
        .current_dir(&workspace_root)
        .output()?;

    if !output.status.success() {
        return Err(SelfHostError::Stage1Failed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(StageArtifact {
        path: stage0.to_path_buf(),
        hash: hash_file(stage0)?,
    })
}

pub fn build_stage2(stage1: &Path) -> Result<StageArtifact, SelfHostError> {
    let workspace_root = workspace_root();
    let output = Command::new(stage1)
        .args(["run", "--", "examples/hello.omni"])
        .current_dir(&workspace_root)
        .output()?;

    if !output.status.success() {
        return Err(SelfHostError::Stage2Failed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(StageArtifact {
        path: stage1.to_path_buf(),
        hash: hash_file(stage1)?,
    })
}

pub fn compare_stages(stage1_hash: &str, stage2_hash: &str) -> Result<(), SelfHostError> {
    if stage1_hash != stage2_hash {
        return Err(SelfHostError::StageMismatch {
            stage1: stage1_hash.to_string(),
            stage2: stage2_hash.to_string(),
        });
    }
    Ok(())
}

pub fn run_self_host_pipeline() -> Result<(), SelfHostError> {
    eprintln!("=== Building Stage0 (Rust-based) ===");
    let stage0 = build_stage0()?;
    eprintln!("Stage0 hash: {}", stage0.hash);

    eprintln!("\n=== Building Stage1 (using Stage0) ===");
    let stage1 = build_stage1(&stage0.path)?;
    eprintln!("Stage1 hash: {}", stage1.hash);

    eprintln!("\n=== Building Stage2 (using Stage1) ===");
    let stage2 = build_stage2(&stage1.path)?;
    eprintln!("Stage2 hash: {}", stage2.hash);

    eprintln!("\n=== Comparing Stage1 vs Stage2 ===");
    compare_stages(&stage1.hash, &stage2.hash)?;
    eprintln!("SUCCESS: Stage1 == Stage2 (self-hosting verified)");

    Ok(())
}
