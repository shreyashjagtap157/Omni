#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

remove_if_present() {
  if [[ -e "$1" ]]; then
    echo "remove $1"
    rm -rf -- "$1"
  fi
}

# Rust/Omni generated build output. Never source.
remove_if_present target
while IFS= read -r -d '' dir; do
  remove_if_present "$dir"
done < <(find crates -mindepth 2 -type d -name target -print0 2>/dev/null || true)

# Generated fuzz/coverage/transient output. Fuzz target source is deliberately untouched.
remove_if_present crates/omni-fuzz/artifacts
remove_if_present .fuzz-cache
remove_if_present coverage
remove_if_present tmp

if [[ "${1:-}" == "--local-llvm" ]]; then
  # Only project-local SDK/build/cache locations covered by .gitignore.
  # System LLVM installations are never touched.
  remove_if_present llvm-build
  remove_if_present toolchains/llvm
  remove_if_present .llvm-cache
fi

echo "Generated-output cleanup complete. Fuzz source and codegen-llvm source were preserved."
