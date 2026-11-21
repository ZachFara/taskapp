#!/usr/bin/env bash
# Rebuilds dev.db (the database SQLx checks queries against) from the migrations.
set -euo pipefail
cd "$(dirname "$0")/.."
rm -f dev.db dev.db-wal dev.db-shm
cargo sqlx database setup --source crates/core/migrations
