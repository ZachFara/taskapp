# Final review: sync and the app shell (Sonnet, read-only)

Repo: <workspace>/taskapp-wt/review-sync (detached at plan-1; do not commit or push). Read AGENTS.md,
docs/DESIGN.md, docs/plans/plan-1.md "Global Constraints" and "Review Focus", and PROGRESS.md rulings. Read the code
as it stands, not the plan.

Rules: reproduce every finding before reporting it (a failing test you wrote in a scratch copy, a command, or exact
input -> wrong output). For each feature in your area, break three safety-relevant behaviours on purpose (edit the
code in place, run the relevant tests by name, then `git checkout -- .`) and record whether a test noticed. Use
SQLX_OFFLINE=true and this worktree's own target dir. Never touch the real database, the owner's Reminders, Google,
Keychain, or any remote. Run only the tests you need, by name.

Write the report AS YOU GO to <workspace>/taskapp/.scratch/review/sync.md (append after each finding): severity (Critical, Important, Minor), file:line,
what is wrong, how you reproduced it, the smallest fix. End with the break-test table (behaviour broken, test that
caught it or "none"). Reply in at most 12 lines: counts by severity and the top three.

## Your area
crates/reminders, crates/gcal, crates/export, helpers/reminders-helper (Swift), app/src-tauri (sync_loop, deeplink, commands, lib.rs, capabilities, tauri.conf.json).

## Questions the ledger set aside for you
- Review Focus 2 (reimport after crash), 3 (edited/deleted Google events, revoked token visible), 5 (MCP/CLI writes while app open, UI refresh within 2 s).
- gcal uses a new `timezone` setting (default UTC): does the digest land on the right local day for the owner?
- export: description lines like '## Comments' round-trip wrongly; settings excluded by substring. Data-loss risk?
- sync loop: own writes can hide an external write from the poller for one tick; backup errors only reach stderr.
- reminders helper delete is scoped to the list (ruling 14): verify in the EventKit store code, not only the fake.
- OAuth: PKCE, state check, refresh token only in Keychain, never logged.
