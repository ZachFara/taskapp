#!/usr/bin/env bash
# land.sh against a fake gh: draft and non-green PRs must not be merged.
set -uo pipefail
cd "$(dirname "$0")/../.." || exit 1
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
cat >"$tmp/gh" <<'FAKE'
#!/usr/bin/env bash
echo "$*" >>"$FAKE_LOG"
case "$1 $2" in
  "pr view") echo "{\"state\":\"${FAKE_STATE:-OPEN}\",\"isDraft\":${FAKE_DRAFT:-false},\"headRefOid\":\"abc\"}" ;;
  "pr checks") echo "${FAKE_CHECKS:-[]}" ;;
  "pr update-branch") echo "already up to date"; exit 1 ;;
  "pr merge") ;;
esac
FAKE
chmod +x "$tmp/gh"
fails=0
run() { # run <name> <want-rc> <FAKE_* assignments...> -- <land args>
  name=$1; want=$2; shift 2; envs=(); while [[ $1 != -- ]]; do envs+=("$1"); shift; done; shift
  : >"$tmp/log"
  env PATH="$tmp:$PATH" FAKE_LOG="$tmp/log" LAND_TIMEOUT=2 LAND_POLL=1 "${envs[@]}" scripts/land.sh "$@" >/dev/null 2>&1; rc=$?
  merged=$(grep -c 'pr merge' "$tmp/log")
  if ((rc == want)) && { { ((want == 0)) && [[ ${EXPECT_MERGE:-1} == "$merged" ]]; } || { ((want != 0)) && ((merged == 0)); }; }; then echo "ok   $name"; else echo "FAIL $name (rc=$rc merges=$merged)"; fails=1; fi
}
green='[{"name":"lint","bucket":"pass"},{"name":"check","bucket":"pass"}]'
run "land refuses a draft PR" 1 FAKE_DRAFT=true "FAKE_CHECKS=$green" -- 5
run "land refuses a non-green PR" 1 'FAKE_CHECKS=[{"name":"lint","bucket":"fail"},{"name":"check","bucket":"fail"}]' -- 5
run "land refuses a closed PR" 1 FAKE_STATE=MERGED "FAKE_CHECKS=$green" -- 5
run "land waits on 'no checks yet' and times out without merging" 2 'FAKE_CHECKS=[]' -- 5
run "land waits while pending" 2 'FAKE_CHECKS=[{"name":"check","bucket":"pending"}]' -- 5
run "land refuses when the aggregate check is missing" 2 'FAKE_CHECKS=[{"name":"lint","bucket":"pass"}]' -- 5
EXPECT_MERGE=1 run "land merges a green PR" 0 "FAKE_CHECKS=$green" -- 5
EXPECT_MERGE=0 run "land --dry-run never merges or updates" 0 "FAKE_CHECKS=$green" -- --dry-run 5
grep -q update-branch "$tmp/log" && { echo "FAIL dry-run called update-branch"; fails=1; }
exit $fails
