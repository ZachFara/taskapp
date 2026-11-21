# Brief FW7: fix wave, keyboard (Sonnet)

Read: AGENTS.md, docs/IMPLEMENTER-CONTRACT.md, and the findings named below in full (each has a reproduction and a
suggested fix). Worktree `../taskapp-wt/FW7`, branch `fix/fw7` from `plan-1`; PR into `plan-1`. Own `target/`;
SQLX_OFFLINE=true. Never `--no-verify`, never `merge -s ours`.

## Findings to fix
<workspace>/taskapp/.scratch/review/ui.md Important 3 and 4: Enter navigates only when focus is on the list body, not on buttons/links; register `l`, `/`, `g i`, `g v` from the design; j/k/enter/s/p work in Table and Board too (not Gantt/Calendar).

## Rules
Every finding gets a failing test first, in the owning test file; watch it fail and record the message, then fix.
Smallest fix that closes the finding; no refactors beyond it. Observable UI changes get a harness capture (mock and,
for data paths, real mode with `SQLX_OFFLINE=true pnpm -C ui shot <route> --mode real`).

## Owns
ui/src/lib/keymap.ts, ui/src/components/views/{ListView,TableView,BoardView}.svelte, ui/src/components/{Shell,CommandPalette}.svelte and their tests. Anything else: exact lines in your report.

## Done
All listed findings closed with tests, gate green, PR into plan-1 with CI green (`gh run watch <id> --exit-status`).
Do not merge. Reply in the contract format, max 15 lines, listing each finding ID with its test name.
