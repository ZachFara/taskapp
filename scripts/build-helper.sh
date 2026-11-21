#!/usr/bin/env bash
# Builds the Swift reminders helper (release) and places it where Tauri's externalBin expects it:
# app/src-tauri/binaries/reminders-helper-<target-triple>. Tauri strips the suffix when bundling.
set -euo pipefail
cd "$(dirname "$0")/.."
triple=$(rustc -vV | sed -n 's/^host: //p')
swift build -c release --package-path helpers/reminders-helper --product RemindersHelper
mkdir -p app/src-tauri/binaries
cp helpers/reminders-helper/.build/release/RemindersHelper "app/src-tauri/binaries/reminders-helper-$triple"
echo "built app/src-tauri/binaries/reminders-helper-$triple"
