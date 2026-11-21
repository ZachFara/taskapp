# Brief FW3: fix wave, calendar (Sonnet)

Read: AGENTS.md, docs/IMPLEMENTER-CONTRACT.md, and the findings named below in full (each has a reproduction and a
suggested fix). Worktree `../taskapp-wt/FW3`, branch `fix/fw3` from `plan-1`; PR into `plan-1`. Own `target/`;
SQLX_OFFLINE=true. Never `--no-verify`, never `merge -s ours`.

## Findings to fix
<workspace>/taskapp/.scratch/review/sync.md F1, F2 (F2: the app writes the `timezone` setting from the system zone at startup when unset; reconcile and planner use it), plus the untested PKCE verifier from its break table.

## Rules
Every finding gets a failing test first, in the owning test file; watch it fail and record the message, then fix.
Smallest fix that closes the finding; no refactors beyond it. Observable UI changes get a harness capture (mock and,
for data paths, real mode with `SQLX_OFFLINE=true pnpm -C ui shot <route> --mode real`).

## Owns
crates/gcal/* , app/src-tauri/src/lib.rs (timezone line only). Anything else: exact lines in your report.

## Done
All listed findings closed with tests, gate green, PR into plan-1 with CI green (`gh run watch <id> --exit-status`).
Do not merge. Reply in the contract format, max 15 lines, listing each finding ID with its test name.
