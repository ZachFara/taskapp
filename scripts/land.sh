#!/usr/bin/env bash
# The only way a PR is merged. For each PR in order: refresh from its base, wait for CI to start and
# finish, merge only if every check passed, and stop at the first PR that is not green.
# Usage: land.sh [--dry-run] <pr>...   Env: LAND_TIMEOUT seconds (default 3300), LAND_POLL seconds (default 20).
# Exit: 0 all landed, 1 a PR is not green / draft / not open, 2 timed out waiting.
set -euo pipefail
dry=0
[[ ${1:-} == --dry-run ]] && { dry=1; shift; }
(($# > 0)) || { echo "usage: land.sh [--dry-run] <pr>..." >&2; exit 64; }
timeout=${LAND_TIMEOUT:-3300}; poll=${LAND_POLL:-20}
do_or_say() { if ((dry)); then echo "dry-run: would run: $*"; else "$@"; fi; }

for pr in "$@"; do
  view=$(gh pr view "$pr" --json state,isDraft,headRefOid)
  state=$(jq -r .state <<<"$view"); draft=$(jq -r .isDraft <<<"$view"); head=$(jq -r .headRefOid <<<"$view")
  [[ $state == OPEN ]] || { echo "PR $pr is $state, not landing; stopping." >&2; exit 1; }
  [[ $draft == false ]] || { echo "PR $pr is a draft, not merging; stopping." >&2; exit 1; }

  if ((dry)); then echo "dry-run: would run: gh pr update-branch $pr"; else
    out=$(gh pr update-branch "$pr" 2>&1) || { grep -qi 'up to date' <<<"$out" || { echo "update-branch failed: $out" >&2; exit 1; }; }
    for _ in 1 2 3 4 5 6; do # a new head means CI restarts on the refreshed branch
      new=$(gh pr view "$pr" --json headRefOid | jq -r .headRefOid)
      [[ $new != "$head" ]] && { head=$new; break; }
      sleep "$poll"
    done
  fi

  start=$SECONDS
  while :; do
    checks=$(gh pr checks "$pr" --json name,bucket 2>/dev/null || echo '[]')
    total=$(jq length <<<"$checks")
    pending=$(jq '[.[]|select(.bucket=="pending")]|length' <<<"$checks")
    has_check=$(jq '[.[]|select(.name=="check")]|length' <<<"$checks")
    failed=$(jq -r '[.[]|select(.bucket=="fail" or .bucket=="cancel")|.name]|join(", ")' <<<"$checks")
    if [[ -n $failed ]]; then echo "PR $pr is not green, failed: $failed; stopping." >&2; exit 1; fi
    if ((total > 0 && pending == 0 && has_check > 0)); then break; fi
    if ((total == 0)); then echo "PR $pr: no checks yet, waiting"; else echo "PR $pr: $pending pending, waiting"; fi
    if ((SECONDS - start >= timeout)); then echo "PR $pr: timed out after ${timeout}s; checks: $(jq -c . <<<"$checks")" >&2; exit 2; fi
    sleep "$poll"
  done
  echo "PR $pr is green ($total checks)"
  do_or_say gh pr merge "$pr" --squash --match-head-commit "$head"
done
