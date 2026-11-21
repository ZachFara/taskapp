# Final review: core data and time (Sonnet, read-only)

Repo: <workspace>/taskapp-wt/review-core (detached at plan-1; do not commit or push). Read AGENTS.md,
docs/DESIGN.md, docs/plans/plan-1.md "Global Constraints" and "Review Focus", and PROGRESS.md rulings. Read the code
as it stands, not the plan.

Rules: reproduce every finding before reporting it (a failing test you wrote in a scratch copy, a command, or exact
input -> wrong output). For each feature in your area, break three safety-relevant behaviours on purpose (edit the
code in place, run the relevant tests by name, then `git checkout -- .`) and record whether a test noticed. Use
SQLX_OFFLINE=true and this worktree's own target dir. Never touch the real database, the owner's Reminders, Google,
Keychain, or any remote. Run only the tests you need, by name.

Write the report AS YOU GO to <workspace>/taskapp/.scratch/review/core.md (append after each finding): severity (Critical, Important, Minor), file:line,
what is wrong, how you reproduced it, the smallest fix. End with the break-test table (behaviour broken, test that
caught it or "none"). Reply in at most 12 lines: counts by severity and the top three.

## Your area
crates/core (issues, statuses, properties, projects, cycles, labels, comments, blocks, relations, schedule, views, search, quickadd, pins, backup, history, settings, migrations).

## Questions the ledger set aside for you
- Review Focus 1 (DST, date-only never shifts) and 4 (property rename/kind change/delete) hold?
- views: group_by and columns are not validated against defs; label sort is Invalid. Real problem?
- pins accept only obsidian:// for notes while project doc_link also allows vault-relative .md: consistent with the design?
- restore guard relies on SQLite refusing to leave WAL while another connection is open; is that sound?
- quickadd grammar rulings (bare date words, last token wins) vs the design examples.
- work blocks write no history (ruling 19): acceptable against 'every mutation writes change history'?
