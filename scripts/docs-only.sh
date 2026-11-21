#!/usr/bin/env bash
# Reads changed file paths on stdin; prints "true" if every one is prose (docs/, .briefs/, or a root .md/.txt)
# and none is a fixture. Empty input prints "false".
n=0
while IFS= read -r f; do
  [[ -z $f ]] && continue
  n=$((n + 1))
  [[ $f == *fixtures/* ]] && { echo false; exit 0; }
  [[ $f == docs/* || $f == .briefs/* || $f =~ ^[^/]+\.(md|txt)$ ]] || { echo false; exit 0; }
done
((n > 0)) && echo true || echo false
