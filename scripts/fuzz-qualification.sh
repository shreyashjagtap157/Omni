#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# The normative v0.1.0 release gate is the deterministic 60-second black-box
# lexer/parser fuzz run executed by qualify-release.sh. cargo-fuzz remains a
# valuable deeper oracle, but it is intentionally optional so an offline Omni
# source release does not depend on an unbundled third-party Cargo subcommand.
if ! command -v cargo >/dev/null 2>&1; then
  echo "SKIP optional cargo-fuzz oracle: cargo is unavailable"
  exit 0
fi
if ! cargo fuzz --help >/dev/null 2>&1; then
  echo "SKIP optional cargo-fuzz oracle: cargo-fuzz is not installed"
  exit 0
fi
if [[ ! -d crates/omni-fuzz ]]; then
  echo "SKIP optional cargo-fuzz oracle: crates/omni-fuzz is not present"
  exit 0
fi

(
  cd crates/omni-fuzz
  cargo fuzz run lexer_parser -- -max_total_time=60
)
echo "Omni optional cargo-fuzz lexer/parser oracle PASS"
