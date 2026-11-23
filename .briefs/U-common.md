# Common rules for wave-1 UI tasks (U1-U21)

Read AGENTS.md, docs/IMPLEMENTER-CONTRACT.md, .briefs/P-common.md, this file, then your brief.
Then read, on your branch: docs/design-system.md, ui/src/styles/tokens.css, the primitives in ui/src/ui/ (open
`/design` in the harness once), and the header comment of ui/src/lib/commands/index.ts.
Reference for the look (static spike, open its screenshots):
<workspace>/taskapp-wt/spike-directions/ui/public/directions/b/ (list.html, board.html, palette.html, shots/).

- UI only: no cargo builds unless your brief says so. Worktree `../taskapp-wt/<ID>`, branch `task/<ID>` from origin/plan-2.
- You REPLACE your own files (some are stubs put there for you). Do not edit any other component, the primitives,
  tokens, the registry engine, api.mock.ts, routes.ts or App.svelte unless your brief names it. Need a new primitive
  or token? Do not add one locally: report the exact need and use the closest existing one meanwhile.
- Look: tokens only, primitives only (`scripts/check-tokens.sh ui/src/<your files>` must pass). Linear's interaction
  patterns (dense rows, inline editing, quiet chrome, fast transitions under 150ms) with the Survey-map identity.
  Every page uses `PageHeader`; lists use `GroupHeader` legend bands; empty and loading states are designed (`EmptyState`, `Skeleton`).
- Keys: no keydown listeners and no `registerShortcut` in components. Actions are commands in
  `ui/src/lib/commands/defs/<area>/<id>.ts` (one file each, label, icon, shortcut or noShortcutReason). Buttons that
  run a command use `CommandButton` (tooltip with name + shortcut). Focus and selection come from `selection.svelte.ts`.
- Data: use the existing api (`ui/src/lib/api.ts`) and stores. Colours of entities: read `color_hex` or the palette
  token for the `color` key, never treat `color` as CSS. Icons: `EntityIcon`.
- Accessibility: every control has a name, visible focus ring, works from the keyboard; lists are real lists/tables with roles.
- Tests first, watched failing: behaviour tests with @testing-library/svelte for what your screen does (not snapshots).
  Keep or port the existing tests of the files you replace; do not delete a behaviour test without replacing it.
- Captures before the PR: your screen in mock AND real mode, light and dark, 1440x900 and 1024x768 (flags
  `--theme`, `--size` if present on your branch; otherwise use the shot script's defaults and say so). OPEN each PNG
  with the Read tool; fix what looks wrong; attach paths in the report with one line on how it follows the design system.
- Report format as in the contract, plus: any primitive/token you were missing, any command ids you added.

## Start and finish order (plan-2 is moving while you work)
- START from the scaffold branch, which has the design system, the registry and your stub files:
  `git -C <workspace>/taskapp fetch && git -C <workspace>/taskapp worktree add ../taskapp-wt/<ID> -b task/<ID> origin/chore/plan2-scaffold`
- Other foundations land on plan-2 during your work: neutral fixtures and a mock that matches the backend (keys become
  ABC-n, data changes), free-port harness with `--theme` and `--size`, undo for every entity, `workspace_status`.
  So in tests NEVER assert on fixture strings or ids you did not create: create the data your test needs through the
  api inside the test (or pass your own seed to createMockApi).
- BEFORE captures and before opening the PR: `git fetch && git merge origin/plan-2` (merge commit, keep both sides in
  shared files, your files win in your own files). If `ui/harness/shot.ts` on origin/plan-2 does not yet have the
  `--theme` flag, wait for it with ONE blocking command (limit 45 min):
  `until git fetch -q && git grep -q -e '--theme' origin/plan-2 -- ui/harness/shot.ts; do sleep 60; done`
  then merge. Never run the harness before that merge (fixed ports collide between parallel agents).
- PR base is `plan-2`.

## Machine load (added 13:00): about 20 agents share 8 cores
- Run tests ONE FILE AT A TIME, in the foreground, with few workers: `pnpm -C ui exec vitest run <file> --maxWorkers=2 --minWorkers=1`.
  Never the whole suite, never in the background, never in a loop. A slow run is load, not a hang: wait for it (timeout 10 min).
- The many `vitest`, `node`, `cargo` and `rustc` processes you see belong to other agents. Never kill processes you did not start.
- No cargo build unless your brief allows it. For real-mode captures the harness builds the devserver once per worktree:
  do captures LAST, once, and only the four required shots per screen.
- Timing tests may fail locally under this load; note it and let CI decide.

## RULING 13:10 (replaces the capture rules above): no harness captures by wave-1 agents
Disk and CPU cannot take 20 devserver builds. Do NOT run the harness or cargo for captures, and do NOT wait for the
`--theme` flag. When your code and tests are done: merge origin/plan-2, run your test files one at a time
(`--maxWorkers=2`; this vitest has no `--minWorkers`), run `scripts/gate.sh`, open the PR into plan-2, wait for CI once,
report. In the report write "captures: not done, visual check is centralised". The orchestrator captures every screen
in one place and sends you a fix list if needed.

## RULING 13:40: do NOT merge plan-2 yourself
Merging plan-2 into a UI worktree stages Rust files, so the pre-commit gate does a full Rust build there (30+ minutes
under load, 7 GB of disk each). So: do NOT `git merge origin/plan-2`. If a merge is in progress, `git merge --abort`.
Just push your branch, open the PR into plan-2, wait for CI once (CI tests the merged result), report. The
orchestrator merges branches together in one place. If you truly need something that landed on plan-2 after you
branched, say exactly what in your report instead of merging.
