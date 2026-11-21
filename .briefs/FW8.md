# Brief FW8: fix wave, CLI (Sonnet)

Read: AGENTS.md, docs/IMPLEMENTER-CONTRACT.md, and the findings named below in full (each has a reproduction and a
suggested fix). Worktree `../taskapp-wt/FW8`, branch `fix/fw8` from `plan-1`; PR into `plan-1`. Own `target/`;
SQLX_OFFLINE=true. Never `--no-verify`, never `merge -s ours`.

## Findings to fix
<workspace>/taskapp/.scratch/review/agents.md: A1 (CLI snapshots go to the same backups dir as MCP, via one shared helper), A3 (CLI create flags for created_at, completed_at, properties incl. linear_id, parent, cycle, labels, comment created_at, and a `--migration` flag for source Migration, matching MCP), and tests for: snapshot failure aborts the change (MCP and CLI), CLI writes record source Cli, CLI property delete snapshots first.

## Rules
Every finding gets a failing test first, in the owning test file; watch it fail and record the message, then fix.
Smallest fix that closes the finding; no refactors beyond it. Observable UI changes get a harness capture (mock and,
for data paths, real mode with `SQLX_OFFLINE=true pnpm -C ui shot <route> --mode real`).

## Owns
crates/cli/*, crates/mcp/* (snapshot dir helper and the snapshot-failure test only). Anything else: exact lines in your report.

## Done
All listed findings closed with tests, gate green, PR into plan-1 with CI green (`gh run watch <id> --exit-status`).
Do not merge. Reply in the contract format, max 15 lines, listing each finding ID with its test name.
