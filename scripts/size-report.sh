#!/usr/bin/env bash
# Report-only: files over 800 lines and packages over 5000 lines. Always exits 0.
cd "$(dirname "$0")/.." || exit 0
files=$(git ls-files -co --exclude-standard | grep -E '\.(rs|ts|svelte|swift)$' || true)
echo "== largest files"
[[ -n $files ]] && echo "$files" | xargs wc -l | grep -v ' total$' | sort -rn | head -10
echo "== files over 800 lines"
[[ -n $files ]] && echo "$files" | xargs wc -l | grep -v ' total$' | awk '$1 > 800 {print "FLAG " $0}'
echo "== lines per package (flag at 5000)"
for p in crates/* app/src-tauri helpers/reminders-helper ui; do
  [[ -d $p ]] || continue
  n=$(echo "$files" | grep "^$p/" | xargs cat 2>/dev/null | wc -l | tr -d ' ')
  flag=""; ((n > 5000)) && flag=" FLAG"
  echo "$n $p$flag"
done
exit 0
