$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")
if (Get-Command py -ErrorAction SilentlyContinue) {
    py scripts/verify-source.py
} elseif (Get-Command python -ErrorAction SilentlyContinue) {
    python scripts/verify-source.py
} else {
    throw "Python 3 is required for the offline source verifier."
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "Cargo is required. Install the stable Rust toolchain first."
}
cargo install --path crates/omni-stage0 --locked --force
omni --version
