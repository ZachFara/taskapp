#!/usr/bin/env bash
# Self-test for check-tokens.sh: each rule must fail on a bad input, and a token-only file must pass.
set -uo pipefail
cd "$(dirname "$0")/../.." || exit 1
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
fails=0
expect() { # expect <0|nonzero> <name> <css-line>
  printf '%s\n' "$3" >"$tmp/t.css"
  scripts/check-tokens.sh "$tmp/t.css" >/dev/null 2>&1; rc=$?
  if { [[ $1 == 0 ]] && ((rc == 0)); } || { [[ $1 == nonzero ]] && ((rc != 0)); }; then echo "ok   $2"; else echo "FAIL $2 (rc=$rc)"; fails=1; fi
}
expect nonzero "hex colour fails" 'a { color: #1a2b3c; }'
expect nonzero "short hex fails" 'a { color: #fff; }'
expect nonzero "rgb() fails" 'a { color: rgb(0 0 0 / 0.5); }'
expect nonzero "hsl() fails" 'a { color: hsl(10, 20%, 30%); }'
expect nonzero "px font-size fails" 'a { font-size: 13px; }'
expect nonzero "px in font shorthand fails" 'a { font: 600 13px/1 sans-serif; }'
expect nonzero "raw ms duration fails" 'a { transition: color 120ms ease; }'
expect nonzero "raw s duration fails" 'a { animation: spin 0.9s linear; }'
expect 0 "tokens pass" 'a { color: var(--ink); font-size: var(--fs-md); transition: color var(--dur-1) var(--ease); border: 1px solid var(--border); }'
expect 0 "svelte blocks and anchors pass" '{#each items as i} <a href="#top">x</a> {/each}'
# Old components are not restyled yet; the design system's own files must already pass.
if scripts/check-tokens.sh ui/src/ui/*.svelte ui/src/app/DesignGallery.svelte >/dev/null 2>&1; then echo "ok   design system files pass"; else echo "FAIL design system files"; fails=1; fi
exit $fails
