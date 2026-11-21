#!/usr/bin/env bash
# Self-tests for the checks: each one must fail on a bad input, not only pass on a good one.
set -uo pipefail
cd "$(dirname "$0")/../.."
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
fails=0
expect() { # expect <0|nonzero> <name> <cmd...>
  want=$1; name=$2; shift 2
  "$@" >/dev/null 2>&1; rc=$?
  if { [[ $want == 0 ]] && ((rc == 0)); } || { [[ $want == nonzero ]] && ((rc != 0)); }; then echo "ok   $name"; else echo "FAIL $name (rc=$rc)"; fails=1; fi
}
# layout
printf '%s\n' Cargo.toml crates/core/src/lib.rs ui/src/lib/api.ts >"$tmp/good"
printf '%s\n' Cargo.toml crates/core/src/stray.txt >"$tmp/stray"
printf '%s\n' crates/core/src/deep/x.rs >"$tmp/deep"
expect 0 "layout accepts allowed files" scripts/check-layout.sh "$tmp/good"
expect nonzero "layout rejects a stray file" scripts/check-layout.sh "$tmp/stray"
expect nonzero "layout rejects a nested file under a single-level pattern" scripts/check-layout.sh "$tmp/deep"
expect 0 "layout passes on the real tree" scripts/check-layout.sh
# boundaries
mk() { echo "{\"packages\":[{\"name\":\"$1\",\"dependencies\":[{\"name\":\"$2\"}]}]}"; }
mk taskapp-core taskapp-gcal >"$tmp/core-gcal.json"
mk taskapp-gcal taskapp-core >"$tmp/gcal-core.json"
mk taskapp-mcp taskapp-export >"$tmp/mcp-export.json"
mk taskapp-commands taskapp-gcal >"$tmp/cmd-gcal.json"
expect nonzero "boundaries: core importing a sync crate fails" scripts/check-boundaries.sh "$tmp/core-gcal.json"
expect nonzero "boundaries: mcp importing a sync crate fails" scripts/check-boundaries.sh "$tmp/mcp-export.json"
expect nonzero "boundaries: commands importing a sync crate fails" scripts/check-boundaries.sh "$tmp/cmd-gcal.json"
expect 0 "boundaries: sync crate on core passes" scripts/check-boundaries.sh "$tmp/gcal-core.json"
expect 0 "boundaries pass on the real workspace" scripts/check-boundaries.sh
# commit messages
msg() { printf '%s\n' "$1" >"$tmp/msg"; scripts/check-commit-msg.sh "$tmp/msg"; }
expect 0 "commit: conventional subject passes" msg "feat(core): add issues"
expect nonzero "commit: missing type fails" msg "add issues"
expect nonzero "commit: 73 chars fails" msg "feat: $(printf 'x%.0s' {1..70})"
# docs-only
d() { printf '%s\n' "$@" | scripts/docs-only.sh; }
[[ $(d docs/a.md README.md) == true ]] && echo "ok   docs-only true" || { echo "FAIL docs-only true"; fails=1; }
[[ $(d docs/a.md crates/core/src/lib.rs) == false ]] && echo "ok   docs-only false on code" || { echo "FAIL docs-only code"; fails=1; }
[[ $(d docs/fixtures/x.md) == false ]] && echo "ok   docs-only false on fixture" || { echo "FAIL docs-only fixture"; fails=1; }
scripts/test/test-land.sh || fails=1
exit $fails
