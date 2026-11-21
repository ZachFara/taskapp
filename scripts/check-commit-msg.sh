#!/usr/bin/env bash
# Conventional commit subject, at most 72 chars. Usage: check-commit-msg.sh <message-file>
set -euo pipefail
subject=$(head -n1 "$1")
case "$subject" in Merge\ *|Revert\ \"*) exit 0 ;; esac
re='^(feat|fix|chore|docs|refactor|test|ci|perf|build|style|revert)(\([a-z0-9-]+\))?!?: .+'
if ! [[ $subject =~ $re ]]; then
  echo "commit subject must be 'type(scope): text' (types: feat fix chore docs refactor test ci perf build style revert): $subject" >&2
  exit 1
fi
if ((${#subject} > 72)); then
  echo "commit subject is ${#subject} chars, limit is 72: $subject" >&2
  exit 1
fi
