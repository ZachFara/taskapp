# Development harness

Open a screen, capture it, read its console, without the Tauri app. Never touches
`~/Library/Application Support/taskapp`; the devserver refuses any `--db` under that directory
(symlinks and relative paths are resolved first).

## Modes
- **mock**: Vite with `VITE_API_MODE=mock` on http://localhost:5174, fixtures from `ui/src/lib/api.mock.ts`.
  No Rust needed. Use this first.
- **real**: Vite with `VITE_API_MODE=http` on http://localhost:5173 talking to `taskapp-devserver` on
  127.0.0.1:7878 (CORS allows only `http://localhost:5173`). `shot --mode real` seeds a temp db and starts both.
  By hand: `cargo run -p taskapp-devserver -- serve --db .scratch/dev-app.db` after `scripts/seed-dev.sh`.
  `serve --port 0` picks a free port; the first stdout line is `listening on <addr>`.

The devserver serves only `taskapp_commands::dispatch`. Sync commands (`gcal_*`, `reminders_*`, `sync_*`)
are absent and return a `NotFound` error; it never runs any sync. Responses are `{ok: value}` or
`{error: {kind, message}}`.

## Capture
`pnpm -C ui shot <route> [--mode mock|real] [--width 1440] [--height 900] [--keys "c,renew lease,Enter"] [--click "<css>"] [--out dir]`

Writes `ui/harness-out/<route-slug>.png`, `.aria.txt` (accessibility tree) and `.console.json`.
Exits 1 on any console error, page error, failed request or HTTP status >= 400 (favicon excluded).
`--keys`: comma-separated; a token like `Enter` or `Meta+k` is a key press, anything else is typed text.
A full `http://` URL as the route is opened as is, with no dev server started. Servers the script started
are stopped at the end; ones it found already running are left alone.

## After a UI change
1. `pnpm -C ui shot <your route>`, open the PNG, check it looks right.
2. Read the `.aria.txt` for the expected text and `.console.json` for `[]`.
3. Repeat for each state you changed (use `--keys` / `--click`), one capture per behaviour.
4. Attach the PNG path in your report.

Needs Playwright Chromium: `pnpm -C ui exec playwright install chromium` once.
