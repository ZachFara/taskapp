# Brief FW2: fix wave, backup safety (Haiku)

Read: AGENTS.md, docs/IMPLEMENTER-CONTRACT.md, and the findings named below in full (each has a reproduction and a
suggested fix). Worktree `../taskapp-wt/FW2`, branch `fix/fw2` from `plan-1`; PR into `plan-1`. Own `target/`;
SQLX_OFFLINE=true. Never `--no-verify`, never `merge -s ours`.

## Findings to fix
<workspace>/taskapp/.scratch/review/core.md F2, F3 and <workspace>/taskapp/.scratch/review/sync.md F7. F2: remove `restore_live` and its command/mock/binding (Settings already shows the CLI instruction). F3: restore refuses a file that is not a taskapp database (needs the `issues` and `settings` tables and an `issue_prefix` setting) and first copies the current target to `<target>.pre-restore-<UTC timestamp>`. F7: add `backup: SyncState` to `SyncStatus`, set it from the sync loop's backup step, show it in Shell's status indicator.

## Rules
Every finding gets a failing test first, in the owning test file; watch it fail and record the message, then fix.
Smallest fix that closes the finding; no refactors beyond it. Observable UI changes get a harness capture (mock and,
for data paths, real mode with `SQLX_OFFLINE=true pnpm -C ui shot <route> --mode real`).

## Owns
crates/core/src/{backup,model}.rs and tests, crates/commands, app/src-tauri/src/{commands,sync_loop}.rs (backup status lines only), ui/src/lib/{bindings,api.mock}.ts, ui/src/components/Shell.svelte. Anything else: exact lines in your report.

## Done
All listed findings closed with tests, gate green, PR into plan-1 with CI green (`gh run watch <id> --exit-status`).
Do not merge. Reply in the contract format, max 15 lines, listing each finding ID with its test name.
