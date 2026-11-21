#!/usr/bin/env bash
# The fast gate: layout, boundaries, fmt, clippy, build. No tests. Used by the pre-commit hook.
# Rust steps are skipped when no Rust or manifest file is staged.
set -euo pipefail
cd "$(dirname "$0")/.."
step() { printf '== %s\n' "$1"; }
step layout; scripts/check-layout.sh
if [[ ${GATE_ALL:-0} == 1 ]] || git diff --cached --name-only | grep -qE '\.rs$|Cargo\.(toml|lock)$|clippy\.toml$'; then
  step boundaries; scripts/check-boundaries.sh
  step fmt; cargo fmt --all --check
  step clippy; cargo clippy --workspace --all-targets -- -D warnings
  step build; cargo build --workspace
else
  echo "no Rust changes staged: fmt, clippy, build skipped"
fi
