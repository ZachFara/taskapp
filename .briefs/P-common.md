# Common rules for plan-2 tasks

Read AGENTS.md, docs/IMPLEMENTER-CONTRACT.md, this file, then your own brief. Do not read the whole plan.

- Product name is now "Cairn" (was taskapp; crate and package names stay `taskapp-*` for now). It is a generic product:
  no personal names, no "ZF", no real projects, no fixed time zone. Workspace key in examples is ABC.
- Integration branch `plan-2`. Your worktree and branch:
  `git -C <workspace>/taskapp worktree add ../taskapp-wt/<ID> -b task/<ID> origin/plan-2`
  Work ONLY there. Own `target/` (never set CARGO_TARGET_DIR). UI work: `pnpm -C ui install --frozen-lockfile` first.
  UI-only tasks must not run cargo builds unless the brief says so (disk is tight).
- File ownership: edit only the files and folders your brief names. Shared registration points you may ADD lines to
  (never reorder or reformat): ui/src/app/routes.ts, crates/core/src/lib.rs, commands dispatch, mcp tools.rs, cli.
  Anything else outside your files: put the exact lines in your report.
- Design system (once it exists on plan-2): `ui/src/styles/tokens.css` + primitives in `ui/src/ui/`. Every colour, size,
  radius, shadow and duration comes from a token; no hex, rgb(), px font sizes or raw durations in component styles
  (`scripts/check-tokens.sh` enforces it). No browser-default buttons, inputs or selects: use the primitives.
- Commands and keys (once the registry exists): every user action is a command in `ui/src/lib/commands/defs/<id>.ts`
  (one file per command, self-registered). Never add a keydown listener in a component; bind keys through the registry.
- Tests first; watch each fail; record the message. Run only your own tests by name plus `scripts/gate.sh`. CI runs the rest.
  A timing test must take the median of at least 5 runs.
- UI tasks end with harness captures: `pnpm -C ui shot <route> --mode mock` AND `--mode real`, light and dark, at
  1440x900 and 1024x768, zero console errors. OPEN each PNG with the Read tool and fix what looks wrong before reporting.
  Add one line on how the screen follows the design system.
- Never touch `~/Library/Application Support/taskapp`, real Reminders, Google, Keychain, or any network service but this repo's GitHub.
- One PR into `plan-2`; wait for CI with one blocking `gh pr checks <n> --watch` (limit 45 min); rerun once only if the
  sole failure is a timing test. Do not merge. If `gh pr update-branch` is needed later you will be told.
- Costly or spec-contradicting gap: NEEDS_CONTEXT. Cheap gap: choose and list it.
- Report: contract format (max 15 lines) plus start/finish time and PR number.
