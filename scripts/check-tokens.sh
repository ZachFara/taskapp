#!/usr/bin/env bash
# Fails on raw design values in UI source outside ui/src/styles/: hex colours, rgb()/hsl(), px font sizes,
# raw durations. Everything visual comes from a token in ui/src/styles/tokens.css.
# Usage: check-tokens.sh [file...]  (default: every .svelte and .css under ui/src except ui/src/styles)
set -euo pipefail
cd "$(dirname "$0")/.."
if (($#)); then files=("$@"); else
  files=(); while IFS= read -r f; do files+=("$f"); done < <(find ui/src \( -name '*.svelte' -o -name '*.css' \) -not -path 'ui/src/styles/*' | sort)
fi
((${#files[@]})) || exit 0
bad=0
scan() { # scan <label> <regex>
  if out=$(grep -nE "$2" "${files[@]}" 2>/dev/null); then
    while IFS= read -r l; do printf 'tokens: %s: %s\n' "$1" "$l" >&2; done <<<"$out"; bad=1
  fi
}
scan 'hex colour' '(^|[^&{[:alnum:]_])#[0-9a-fA-F]{3,8}\b'
scan 'rgb/hsl colour' '\b(rgba?|hsla?|hwb|oklch|oklab|lab|lch)\('
scan 'px font size' 'font-size:[[:space:]]*[0-9.]+px|font:[^;]*[0-9.]+px'
scan 'raw duration' '(transition|animation|duration|delay)[^;]*[^-[:alnum:]_.][0-9.]+m?s\b'
exit $bad
