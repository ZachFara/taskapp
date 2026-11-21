#!/usr/bin/env bash
# Rejects files outside scripts/layout-allowed.txt.
# Usage: check-layout.sh [file-list] [allowed-list]  (default: git's tracked + unignored files)
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
allowed=${2:-$here/layout-allowed.txt}
if [[ -n ${1:-} ]]; then files=$(cat "$1"); else files=$(cd "$here/.." && git ls-files -co --exclude-standard); fi
regex=()
while IFS= read -r p; do
  [[ -z $p || $p == \#* ]] && continue
  p=${p//./\\.}; p=${p//\*\*/$'\001'}; p=${p//\*/[^/]*}; p=${p//$'\001'/.*}
  regex+=("^${p#/}\$")
done <"$allowed"
bad=0
while IFS= read -r f; do
  [[ -z $f ]] && continue
  ok=0
  for r in "${regex[@]}"; do [[ $f =~ $r ]] && { ok=1; break; }; done
  ((ok)) || { echo "layout: file not in allowed list: $f" >&2; bad=1; }
done <<<"$files"
exit $bad
