#!/usr/bin/env sh
set -eu
cd "$(dirname "$0")/.."
python3 scripts/verify-source.py
command -v cargo >/dev/null 2>&1 || { echo "cargo is required" >&2; exit 1; }
cargo install --path crates/omni-stage0 --locked --force
omni --version
