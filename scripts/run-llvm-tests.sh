#!/usr/bin/env bash
set -euo pipefail

if ! command -v llvm-config >/dev/null 2>&1; then
  echo "llvm-config not found. Please install LLVM or run scripts/setup-llvm.sh"
  exit 2
fi

PREFIX=$(llvm-config --prefix)
VER=$(llvm-config --version)
MAJOR=$(echo "$VER" | cut -d. -f1)
MINOR=$(echo "$VER" | cut -d. -f2 | cut -c1)
if [ "$MAJOR" -ne 14 ]; then
  echo "Expected LLVM 14.x, found $VER"
  exit 3
fi
VAR="LLVM_SYS_${MAJOR}${MINOR}_PREFIX"

# export for this process
export ${VAR}="$PREFIX"

echo "Exported ${VAR}=${PREFIX}"

echo "Running tests: cargo test -p codegen-llvm --release --features with_inkwell,real_llvm"
cargo test -p codegen-llvm --release --features with_inkwell,real_llvm -- --nocapture

echo "Running release MIR optimization tests: cargo test -p omni-compiler --release --test mir_optimize"
cargo test -p omni-compiler --release --test mir_optimize
