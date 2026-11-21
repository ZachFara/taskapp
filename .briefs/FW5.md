# Brief FW5: fix wave, reminders import (Haiku)

Read: AGENTS.md, docs/IMPLEMENTER-CONTRACT.md, and the findings named below in full (each has a reproduction and a
suggested fix). Worktree `../taskapp-wt/FW5`, branch `fix/fw5` from `plan-1`; PR into `plan-1`. Own `target/`;
SQLX_OFFLINE=true. Never `--no-verify`, never `merge -s ours`.

## Findings to fix
<workspace>/taskapp/.scratch/review/sync.md F3: notes and due go into the `NewIssue` passed to `create_in` in the same transaction as the import_log row, so nothing is patched after commit; the reminder is deleted only after that commit.

## Rules
Every finding gets a failing test first, in the owning test file; watch it fail and record the message, then fix.
Smallest fix that closes the finding; no refactors beyond it. Observable UI changes get a harness capture (mock and,
for data paths, real mode with `SQLX_OFFLINE=true pnpm -C ui shot <route> --mode real`).

## Owns
crates/reminders/*, crates/core/src/quickadd.rs (only if `new_issue` needs a notes/due parameter). Anything else: exact lines in your report.

## Done
All listed findings closed with tests, gate green, PR into plan-1 with CI green (`gh run watch <id> --exit-status`).
Do not merge. Reply in the contract format, max 15 lines, listing each finding ID with its test name.
