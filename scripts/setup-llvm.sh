#!/usr/bin/env bash
set -euo pipefail

# Installs LLVM (example uses LLVM 14 on Debian/Ubuntu) and prints export instruction
if ! command -v sudo >/dev/null 2>&1; then
  echo "sudo not found; run the package install commands yourself as admin."
fi

echo "Installing LLVM 14 packages (may require sudo)..."
sudo apt-get update
sudo apt-get install -y llvm-14 llvm-14-dev libclang-14-dev clang-14

PREFIX=$(llvm-config --prefix)
VER=$(llvm-config --version)
MAJOR=$(echo "$VER" | cut -d. -f1)
MINOR=$(echo "$VER" | cut -d. -f2 | cut -c1)
VAR="LLVM_SYS_${MAJOR}${MINOR}_PREFIX"

cat <<EOF
LLVM installed. Detected prefix: $PREFIX
To export for this shell run:
  export $VAR=$PREFIX
To persist across sessions add that line to your shell profile (e.g. ~/.profile or ~/.bashrc).
EOF
