#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

python3 scripts/audit-baseline.py --worktree
python3 scripts/verify-source.py --worktree
cargo fmt --all -- --check
cargo clippy --workspace --locked --all-targets -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
cargo build --release --locked -p omni-stage0
cargo install --path crates/omni-stage0 --locked --force
omni --version
omni doctor
omni check examples/native_edition1.omni
python3 scripts/historical-conformance.py --omni omni

# Normative lexer/parser fuzz gate: 60 cumulative seconds. Independent shards are
# deterministic, fit constrained qualification hosts, and diversify seeds.
for seed in 660100 660101 660102 660103; do
  python3 scripts/fuzz-lexer-parser-smoke.py --omni omni --seconds 15 --seed "$seed"
done
./scripts/fuzz-qualification.sh

python3 scripts/native-conformance.py --omni omni
python3 scripts/native-conformance.py --omni omni --manifest conformance/native_layout_v0_1_3/manifest.json
python3 scripts/native-conformance.py --omni omni --manifest conformance/native_value_abi_v0_1_4/manifest.json

echo "Omni v0.1.4.1.1 string/byte/value-ABI + collections-foundation release qualification PASS"
