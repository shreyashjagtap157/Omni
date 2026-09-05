#!/usr/bin/env bash
# Convenience wrapper to bump Omni version and sync to git / GitHub remote.
# Usage:
#   ./scripts/sync-github.sh patch [number]
#   ./scripts/sync-github.sh minor
#   ./scripts/sync-github.sh major
#   ./scripts/sync-github.sh stable
#   ./scripts/sync-github.sh set <x.y.z.w>

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$ROOT_DIR"

TYPE="${1:-patch}"
EXTRA="${2:-}"
REMOTE="${REMOTE:-origin}"

ARGS=("scripts/bump-version.py" "$TYPE")

if [ "$TYPE" = "patch" ] && [ -n "$EXTRA" ]; then
    ARGS+=("--number" "$EXTRA")
elif [ "$TYPE" = "set" ]; then
    if [ -z "$EXTRA" ]; then
        echo "Error: 'set' requires version argument (e.g. 0.2.0.1111)"
        exit 1
    fi
    ARGS+=("$EXTRA")
fi

ARGS+=("--sync" "--remote" "$REMOTE")

echo "Running: python3 ${ARGS[*]}"
python3 "${ARGS[@]}"

echo "Validating repository gates..."
python3 scripts/verify-source.py --worktree
python3 scripts/audit-baseline.py --worktree

echo "All source and baseline audit gates passed!"
