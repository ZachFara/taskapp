# Design system "Survey map"

Review everything at `/design` (every token and primitive, every state, light and dark, a theme switch).

## Files
- `ui/src/styles/tokens.css`: all values. `base.css`: fonts, reset, focus ring, selection, scrollbars, reduced motion.
  Both are imported by `ui/src/app/main.ts`. `fonts/`: Bricolage Grotesque (OFL, `OFL.txt`), self-hosted woff2.
- `ui/src/ui/`: primitives, `icons.ts` (the icon set). `ui/src/app/DesignGallery.svelte`: the gallery.
- `scripts/check-tokens.sh`: fails on hex, rgb()/hsl(), px font sizes and raw durations outside `ui/src/styles/`.

## Tokens
- Colour roles: `frame*` (dark green chrome), `sheet`, `raised`, `active`, `selection`, `card`, `band-1..4` with
  `band-ink` and `band-count`, `ink`, `muted`, `border`, `control`, `focus`, `accent`, `stone`, `signal`, `positive`,
  `warning`, `overlay`, `tip-*`. Label colours `--c-<key>` for the 12 backend palette keys; use them only through
  `EntityIcon` and `Pill`.
- Type `--fs-*` (11, 12, 13, 15, 20, 28). Space `--s-1..10` (2 to 48). Radius `--r-*`. One shadow, `--shadow-float`,
  for floating things only. Motion `--dur-1/2/3` (80/140/220ms) and one `--ease`. Layers `--z-*`. Sizes `--row-h`
  (32), `--row-h-compact` (28), `--ctl-h`, `--sidebar-w`, `--panel-w`.

## Rules
- Do: colour, size, radius, shadow and duration from a token; role tokens only in components; `Button`, `IconButton`,
  `TextInput`, `Select` and the other primitives for every control; an icon-only control always has a label (it feeds
  the tooltip); tooltips carry the action name and its shortcut.
- Do not: hex, rgb(), px font sizes, raw ms; a bare `<button>`, `<input>`, `<select>`; a shadow on anything that does
  not float; a keydown listener in a component (bind keys through the command registry); a monospace font.
- Status is a `StatusGlyph` (stack of three stones: how many are solid), priority a `PriorityGlyph` (rings; urgent
  has a core and the signal colour). Group headers use `GroupHeader`, whose band step follows the status category.
- Frame surfaces set `data-on-frame` so the focus ring uses `--focus-frame`.

## Adding a theme
1. Copy the `[data-theme="light"]` block in `tokens.css`, rename the selector (for example `[data-theme="dusk"]`), and
   change values only. Keep every role and all 12 `--c-*` keys; `tokens.test.ts` checks that every theme block defines
   the same names and that palette colours keep 3:1 on the sheet and band steps 1 and 2.
2. Check contrast with that test, then open `/design?theme=dusk` and look at every section.
3. If it should also follow the system setting, mirror it in a `@media (prefers-color-scheme)` block the way dark does
   (the two dark blocks must stay identical).
No component changes.
