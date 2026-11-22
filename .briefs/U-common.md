# Common rules for wave-1 UI tasks (U1-U21)

Read AGENTS.md, docs/IMPLEMENTER-CONTRACT.md, .briefs/P-common.md, this file, then your brief.
Then read, on your branch: docs/design-system.md, ui/src/styles/tokens.css, the primitives in ui/src/ui/ (open
`/design` in the harness once), and the header comment of ui/src/lib/commands/index.ts.
Reference for the look (static spike, open its screenshots):
/Users/user1/Ark/Development/active/taskapp-wt/spike-directions/ui/public/directions/b/ (list.html, board.html, palette.html, shots/).

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
