#!/usr/bin/env bash
# Import boundaries between workspace crates, read from cargo metadata.
#   core: no workspace deps. mcp, commands: only core. cli: core + export (the restore command). gcal, reminders, export: only core.
#   devserver: core + commands. app: anything.
# Usage: check-boundaries.sh [metadata.json]  (default: cargo metadata)
set -euo pipefail
meta=$(if [[ -n ${1:-} ]]; then cat "$1"; else cargo metadata --format-version 1 --no-deps; fi)
jq -r '
  def allowed: {
    "taskapp-core": [], "taskapp-gcal": ["taskapp-core"], "taskapp-reminders": ["taskapp-core"],
    "taskapp-export": ["taskapp-core"], "taskapp-mcp": ["taskapp-core"], "taskapp-cli": ["taskapp-core", "taskapp-export"],
    "taskapp-commands": ["taskapp-core"], "taskapp-devserver": ["taskapp-core", "taskapp-commands"] };
  .packages[] | select(.name | startswith("taskapp-")) | select(allowed[.name] != null) |
  .name as $n | .dependencies[] | select(.name | startswith("taskapp-")) |
  select(.name as $d | (allowed[$n] | index($d)) == null) | "boundary: \($n) must not depend on \(.name)"' <<<"$meta" |
  { out=$(cat); [[ -z $out ]] || { echo "$out" >&2; exit 1; }; }
