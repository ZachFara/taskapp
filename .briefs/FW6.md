# Brief FW6: fix wave, Gantt (Sonnet)

Read: AGENTS.md, docs/IMPLEMENTER-CONTRACT.md, and the findings named below in full (each has a reproduction and a
suggested fix). Worktree `../taskapp-wt/FW6`, branch `fix/fw6` from `plan-1`; PR into `plan-1`. Own `target/`;
SQLX_OFFLINE=true. Never `--no-verify`, never `merge -s ours`.

## Findings to fix
<workspace>/taskapp/.scratch/review/ui.md Important 1 and 2 (a drag keeps a DateTime due's time and zone and never invents a due for estimate-only bars; apply() is all-or-nothing from the user's view: on any failure reload, show the error, no unhandled rejection), plus the two unnoticed break tests (rollup rows not draggable).

## Rules
Every finding gets a failing test first, in the owning test file; watch it fail and record the message, then fix.
Smallest fix that closes the finding; no refactors beyond it. Observable UI changes get a harness capture (mock and,
for data paths, real mode with `SQLX_OFFLINE=true pnpm -C ui shot <route> --mode real`).

## Owns
ui/src/components/views/GanttView.svelte, ui/src/lib/gantt.ts and their tests. Anything else: exact lines in your report.

## Done
All listed findings closed with tests, gate green, PR into plan-1 with CI green (`gh run watch <id> --exit-status`).
Do not merge. Reply in the contract format, max 15 lines, listing each finding ID with its test name.
