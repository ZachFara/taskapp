#!/usr/bin/env bash
# Installs the dev tools this project needs. Prints the plan, asks, skips anything already present,
# never overwrites an existing install. Log what you ran in docs/INSTALLS.md.
set -euo pipefail
plan=(
  "pnpm|npm install -g pnpm@10"
  "sqlx|cargo install sqlx-cli --version =0.8.6 --no-default-features --features sqlite,rustls --locked"
  "cargo-tauri|cargo install tauri-cli --version ^2 --locked"
  "cargo-machete|cargo install cargo-machete --locked"
  "shellcheck|brew install shellcheck"
)
present() { command -v "$1" >/dev/null 2>&1; }
echo "This will run (only for tools not already on PATH):"
for e in "${plan[@]}"; do present "${e%%|*}" && echo "  skip ${e%%|*} (present)" || echo "  ${e#*|}"; done
read -r -p "Continue? [y/N] " ans
[[ $ans == y ]] || { echo "aborted"; exit 1; }
for e in "${plan[@]}"; do present "${e%%|*}" || eval "${e#*|}"; done
for t in pnpm sqlx cargo-tauri cargo-machete shellcheck; do present "$t" && echo "ok: $t at $(command -v "$t")" || echo "MISSING: $t"; done
