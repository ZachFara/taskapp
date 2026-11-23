# FX6 Backend sweep: what the backend fixer deferred   (owns crates/**, ui/src/lib/api.mock*.ts, ui/src/lib/contract/**, ui/src/lib/bindings.ts generated)
START FROM `origin/task/FX1` (PR #118, green, lands before you): `git worktree add ../taskapp-wt/FX6 -b task/FX6 origin/task/FX1`. Cargo `-j 4`. Rules: .briefs/FX-common.md, .briefs/F-common.md. Read each finding's text in <workspace>/taskapp/.scratch/review2/R1.md and R2.md. Failing test first for each (say the message).
1. R1-2 `settings_set` must validate known keys (issue_prefix, timezone, estimation, point_scale, theme, week_start, default_view_template) the same way their dedicated commands do, and reject unknown keys unless they match an allowed prefix (list the prefixes the UI uses, e.g. `insights.`, `sidebar_`, `cairn.`).
2. R1-3 the last status of a category (at least Unstarted, Started, Completed) cannot be deleted; clear error.
3. R1-4 key lookup accepts only the forms the app produces (`ABC-12`, case-insensitive prefix), not `ABC-+1` or `ABC-01`.
4. R1-5 a change written outside a history step must clear the redo stack (or be impossible: find the writers and put them in a step).
5. R2-8 CSV: duplicate headers (clear error or numbered names), CR-only line endings, row numbers consistent with the file (header = line 1), a size cap with a clear message.
6. R2-9 MCP pin/unpin record the agent source and reject a target that does not exist.
7. R2-10 tool text and error prefixes consistent (read the finding).
8. R2-5 remainder: scalar-argument MCP tools reject unknown argument names too (one shared check on every tool's argument object).
9. Contract cases for the six reachable commands still uncovered: blocks_between, blocks_create, blocks_delete, blocks_move, properties_reorder, properties_set_scope (mock must match).
10. Unused workspace dependency `schemars` in the root Cargo.toml: remove it if nothing uses it (cargo machete must stay clean).
Update <workspace>/taskapp/.scratch/review2/TRACKER.md lines you close (state FIXED (FX6)). One PR into plan-2 (it shows FX1's commits until #118 lands), CI once, report per item, max 15 lines. Do not merge.
