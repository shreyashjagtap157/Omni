$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

python scripts/audit-baseline.py --worktree
python scripts/verify-source.py --worktree
cargo fmt --all -- --check
cargo clippy --workspace --locked --all-targets -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
cargo build --release --locked -p omni-stage0
cargo install --path crates/omni-stage0 --locked --force
omni --version
omni doctor
omni check examples/native_edition1.omni
python scripts/historical-conformance.py --omni omni
foreach ($seed in 660100,660101,660102,660103) {
    python scripts/fuzz-lexer-parser-smoke.py --omni omni --seconds 15 --seed $seed
}
& scripts/fuzz-qualification.ps1
python scripts/native-conformance.py --omni omni
python scripts/native-conformance.py --omni omni --manifest conformance/native_layout_v0_1_3/manifest.json
python scripts/native-conformance.py --omni omni --manifest conformance/native_value_abi_v0_1_4/manifest.json

Write-Host "Omni v0.1.4.1.1 string/byte/value-ABI + collections-foundation release qualification PASS"
