# Common rules for the plan-2 backend features (F1-F4)

Read AGENTS.md, docs/IMPLEMENTER-CONTRACT.md, this file, then your own brief. Do not read the whole plan.

- Integration branch: `plan-2`. Make your own worktree and branch:
  `git -C /Users/user1/Ark/Development/active/taskapp worktree add ../taskapp-wt/<ID> -b task/<ID> origin/plan-2`
  Work ONLY inside that worktree. Use its own `target/` (do not set CARGO_TARGET_DIR). Run `pnpm -C ui install --frozen-lockfile` there if you touch ui/.
- Backend only: core logic, a schema migration, the command in crates/commands (dispatch), Tauri command, MCP tool,
  CLI subcommand, mock api support (ui/src/lib/api.mock.ts, api.ts, bindings.ts) and tests.
  Touch NO file under ui/src/components, ui/src/app, or any styling. No screens.
- Migrations: use exactly the file name your brief gives (crates/core/migrations/). Never edit 0001_init.sql.
  After changing queries run `cargo sqlx prepare --workspace` against a scratch db (see scripts/ and AGENTS.md) and commit `.sqlx/`.
  `.env` sets SQLX_OFFLINE=true; override it for prepare only.
- Shared registration points (crates/core/src/lib.rs mod lines, model.rs, commands dispatch, mcp tools.rs, cli commands.rs,
  api.mock.ts, bindings): you MAY add your lines there for this task. Add, do not reorder or reformat neighbours;
  three other features edit the same files in parallel. Prefer a new file per feature (`crates/core/src/<feature>.rs`).
- Every write goes through the existing change-history recording (crates/core/src/history.rs) and uses BEGIN IMMEDIATE like the other writers.
- Product is becoming generic: no personal names, no "ZF", no real projects in new tests or fixtures. Use prefix ABC and made-up data.
- Tests first; watch each fail; record the message. Run only your own tests by name plus `scripts/gate.sh`. CI runs the rest.
- Never touch `~/Library/Application Support/taskapp`, real Reminders, Google, Keychain, or any network service other than this repo's GitHub.
- Open one PR into `plan-2`, wait for CI with one blocking `gh pr checks <n> --watch` (limit 45 min), then report. Do not merge.
- Costly or spec-contradicting gap: stop with NEEDS_CONTEXT. Cheap gap: choose, and list the choice in the report.
- Report: the format in docs/IMPLEMENTER-CONTRACT.md, plus start and finish time and the PR number.
