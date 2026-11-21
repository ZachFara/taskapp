# Final review: UI (Sonnet, read-only)

Repo: <workspace>/taskapp-wt/review-ui (detached at plan-1; do not commit or push). Read AGENTS.md,
docs/DESIGN.md, docs/plans/plan-1.md "Global Constraints" and "Review Focus", and PROGRESS.md rulings. Read the code
as it stands, not the plan.

Rules: reproduce every finding before reporting it (a failing test you wrote in a scratch copy, a command, or exact
input -> wrong output). For each feature in your area, break three safety-relevant behaviours on purpose (edit the
code in place, run the relevant tests by name, then `git checkout -- .`) and record whether a test noticed. Use
SQLX_OFFLINE=true and this worktree's own target dir. Never touch the real database, the owner's Reminders, Google,
Keychain, or any remote. Run only the tests you need, by name.

Write the report AS YOU GO to <workspace>/taskapp/.scratch/review/ui.md (append after each finding): severity (Critical, Important, Minor), file:line,
what is wrong, how you reproduced it, the smallest fix. End with the break-test table (behaviour broken, test that
caught it or "none"). Reply in at most 12 lines: counts by severity and the top three.

## Your area
ui/ (app shell, ViewPage, every component in ui/src/components, stores, keymap, api.ts and api.mock.ts, harness/).

## Questions the ledger set aside for you
- Gantt is hand-built instead of frappe-gantt (owner decision pending): correctness of drag/shift/critical path rendering.
- Gantt bars and Calendar chips do not open the issue panel; panel clipped at 1440px; issue outside the view shows #id.
- Markdown is sanitized (marked + dompurify) everywhere user text renders as HTML?
- Keyboard-first: cmd+k, c, j/k, enter, esc, s, p work across pages; shortcuts ignored while typing.
- Several tasks wrote tests after code: pick their weakest tests and break the code to see.
- Use the harness (docs/harness.md, mock mode) for at least five screens and look at the PNGs.
