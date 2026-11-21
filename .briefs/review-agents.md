# Final review: agent and CLI surface, security (Sonnet, read-only)

Repo: <workspace>/taskapp-wt/review-agents (detached at plan-1; do not commit or push). Read AGENTS.md,
docs/DESIGN.md, docs/plans/plan-1.md "Global Constraints" and "Review Focus", and PROGRESS.md rulings. Read the code
as it stands, not the plan.

Rules: reproduce every finding before reporting it (a failing test you wrote in a scratch copy, a command, or exact
input -> wrong output). For each feature in your area, break three safety-relevant behaviours on purpose (edit the
code in place, run the relevant tests by name, then `git checkout -- .`) and record whether a test noticed. Use
SQLX_OFFLINE=true and this worktree's own target dir. Never touch the real database, the owner's Reminders, Google,
Keychain, or any remote. Run only the tests you need, by name.

Write the report AS YOU GO to <workspace>/taskapp/.scratch/review/agents.md (append after each finding): severity (Critical, Important, Minor), file:line,
what is wrong, how you reproduced it, the smallest fix. End with the break-test table (behaviour broken, test that
caught it or "none"). Reply in at most 12 lines: counts by severity and the top three.

## Your area
crates/mcp, crates/cli, crates/commands, crates/devserver, ui/src/lib/openExternal.ts, scripts/ (land.sh, gate, checks), .github/workflows/ci.yml.

## Questions the ledger set aside for you
- Snapshot-before-bulk rule (bulk_update > 10, change_property_kind, delete_property) in both MCP and CLI; every agent write records source Agent/Migration/Cli.
- Linear migration needs: can MCP and CLI create every record kind with original timestamps and linear_id?
- devserver: Host check, JSON-only, refuses the real db path (symlink/relative), never runs sync.
- openExternal allow-list and the Tauri opener capability scope.
- `taskapp backup restore` requires --yes; default target is the real db.
- Anything in CI or scripts that could let a red PR merge.
