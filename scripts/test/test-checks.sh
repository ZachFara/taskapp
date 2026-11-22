#!/usr/bin/env bash
# Self-tests for the checks: each one must fail on a bad input, not only pass on a good one.
set -uo pipefail
cd "$(dirname "$0")/../.." || exit 1
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
printf '%s\n' "feat(core): add issues" >"$tmp/m-ok"
printf '%s\n' "add issues" >"$tmp/m-notype"
printf '%s\n' "feat: $(printf 'x%.0s' {1..70})" >"$tmp/m-long"
expect 0 "commit: conventional subject passes" scripts/check-commit-msg.sh "$tmp/m-ok"
expect nonzero "commit: missing type fails" scripts/check-commit-msg.sh "$tmp/m-notype"
expect nonzero "commit: 73 chars fails" scripts/check-commit-msg.sh "$tmp/m-long"
# docs-only
d() { printf '%s\n' "$@" | scripts/docs-only.sh; }
check_d() { # check_d <want> <name> <paths...>
  if [[ $(d "${@:3}") == "$1" ]]; then echo "ok   $2"; else echo "FAIL $2"; fails=1; fi
}
check_d true "docs-only true" docs/a.md README.md
check_d false "docs-only false on code" docs/a.md crates/core/src/lib.rs
check_d false "docs-only false on fixture" docs/fixtures/x.md
scripts/test/test-land.sh || fails=1
scripts/test/test-check-tokens.sh || fails=1
exit $fails
