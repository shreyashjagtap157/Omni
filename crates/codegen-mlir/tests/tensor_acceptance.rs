use codegen_mlir::{emit_control_flow_demo_mlir_text, TensorAddWorkload};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn mlir_opt_bin() -> Option<String> {
    env::var("MLIR_OPT_BIN").ok().filter(|value| !value.trim().is_empty())
}

fn write_temp_mlir(prefix: &str, text: &str) -> Result<PathBuf, String> {
    let mut path = env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    path.push(format!("omni-{}-{}-{}.mlir", prefix, std::process::id(), stamp));
    fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(path)
}

fn run_mlir_opt(bin: &str, input: &PathBuf) -> Result<String, String> {
    let output = Command::new(bin)
        .arg(input)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(format!(
            "mlir-opt failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    String::from_utf8(output.stdout).map_err(|e| e.to_string())
}

#[test]
#[ignore = "toolchain-backed Step 11 acceptance gate"]
fn mlir_tensor_acceptance_gate() {
    let Some(bin) = mlir_opt_bin() else {
        return;
    };

    let workload = TensorAddWorkload::new(4);
    let text = workload.emit_mlir_text();
    let path = write_temp_mlir("tensor-add", &text).expect("failed to create tensor MLIR fixture");
    let stdout = run_mlir_opt(&bin, &path).expect("mlir-opt tensor workload failed");

    let _ = fs::remove_file(&path);

    assert!(stdout.contains("linalg.generic"));
    assert!(stdout.contains("tensor.empty"));
    assert!(stdout.contains("func.return"));
}

#[test]
#[ignore = "toolchain-backed Step 11 acceptance gate"]
fn mlir_control_flow_acceptance_gate() {
    let Some(bin) = mlir_opt_bin() else {
        return;
    };

    let text = emit_control_flow_demo_mlir_text();
    let path = write_temp_mlir("control-flow", &text).expect("failed to create control-flow MLIR fixture");
    let stdout = run_mlir_opt(&bin, &path).expect("mlir-opt control-flow workload failed");

    let _ = fs::remove_file(&path);

    assert!(stdout.contains("cf.cond_br"));
    assert!(stdout.contains("cf.br"));
    assert!(stdout.contains("func.return"));
}