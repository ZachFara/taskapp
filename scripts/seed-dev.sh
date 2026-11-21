#!/usr/bin/env bash
# Seeds a throwaway dev database with the mock fixtures: scripts/seed-dev.sh [path]  (default .scratch/dev-app.db)
set -euo pipefail
cd "$(dirname "$0")/.."
db=${1:-.scratch/dev-app.db}
mkdir -p "$(dirname "$db")"
rm -f "$db" "$db-wal" "$db-shm"
cargo run -q -p taskapp-devserver -- seed --db "$db"
echo "seeded $db"
