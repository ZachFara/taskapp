# Cairn

Cairn is a single-user, Linear-like task manager for the Mac. It has custom properties on every task, a Gantt chart, capture from Apple Reminders, notifications through Google Calendar, optional export to a private GitHub repo, and an MCP server and CLI for agents. The data lives in a local SQLite database that you own.

Status: pre-release. There is no signed or notarized build, and no licence yet. See [docs/DESIGN.md](docs/DESIGN.md) for the spec.

How to use the app: [docs/GUIDE.md](docs/GUIDE.md)

## Build and run

Needs stable Rust, Node with pnpm 10 (`npx -y pnpm@10` works), the Tauri CLI, and the Xcode command line tools for the Swift Reminders helper.

```sh
git config core.hooksPath scripts/hooks   # once per clone
scripts/build-helper.sh                   # Swift helper for Apple Reminders
pnpm -C ui install --frozen-lockfile
cd app/src-tauri && cargo tauri dev
```

On first launch, set the workspace name, issue key prefix (2 to 5 capital letters, default `ABC`) and time zone. The same choices can also be made from the CLI or MCP (below).

## Dev harness

The UI runs in a browser with fake data, or against the real core through a local development server that never touches the real database:

```sh
pnpm -C ui dev:mock                       # fake data on http://localhost:5173
pnpm -C ui shot /view/1 --mode mock       # screenshot, page text and console errors
cargo run -p taskapp-devserver -- seed --db .scratch/dev.db
```

## CLI quick start

```sh
cargo run -p taskapp-cli -- --db .scratch/dev.db setup "Cairn" ABC --tz Europe/Berlin
cargo run -p taskapp-cli -- --db .scratch/dev.db add "Pay rent friday p1"
cargo run -p taskapp-cli -- --db .scratch/dev.db issue get ABC-1
```

Add `--json` for machine-readable output.

## MCP quick start

`taskapp-mcp` is a stdio MCP server over the same database. Point an MCP client at it:

```json
{
  "mcpServers": {
    "cairn": {
      "command": "/path/to/target/debug/taskapp-mcp",
      "args": ["--db", "/path/to/cairn.db"]
    }
  }
}
```

Tools include `workspace_status`, `workspace_setup`, `search_issues`, `create_issue`, `update_issue`, `bulk_update`, `get_history`, `undo` and `redo`.

## Data location

- Database and backups: `~/Library/Application Support/taskapp/` (the directory name and bundle id `app.taskapp.desktop` are kept from the earlier name).
- Google Calendar and GitHub export are off until you connect them in settings.

## What is not verified

- A signed, notarized or installed build.
- Apple Reminders import against a real list.
- Google Calendar sync against a real account.
- GitHub export to a real repository, and restoring from it.

## Development

`AGENTS.md` describes the repo layout, the gate (`scripts/gate.sh`), and the rules for contributors and agents.
