#!/usr/bin/env bash
set -euo pipefail

# Prefer an existing LLVM on PATH; otherwise install a compatible LLVM package.
if command -v llvm-config >/dev/null 2>&1; then
  PREFIX=$(llvm-config --prefix)
  VER=$(llvm-config --version)
else
  if ! command -v sudo >/dev/null 2>&1; then
    echo "sudo not found; run the package install commands yourself as admin."
  fi

  echo "Installing compatible LLVM packages (may require sudo)..."
  sudo apt-get update
  sudo apt-get install -y llvm-19 llvm-19-dev libclang-19-dev clang-19
  PREFIX=$(llvm-config --prefix)
  VER=$(llvm-config --version)
fi

MAJOR=$(echo "$VER" | cut -d. -f1)
MINOR=$(echo "$VER" | cut -d. -f2 | cut -c1)
VAR="LLVM_SYS_${MAJOR}${MINOR}_PREFIX"

cat <<EOF
LLVM installed. Detected prefix: $PREFIX
To export for this shell run:
  export $VAR=$PREFIX
To persist across sessions add that line to your shell profile (e.g. ~/.profile or ~/.bashrc).
EOF
