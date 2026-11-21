# Installs on this machine

Log every install: date, command, why, how to undo. `scripts/install-tools.sh` is the reproducible record.

| Date | Command | Why | Undo |
|---|---|---|---|
| 2026-11-21 | `npm install -g pnpm@10` | UI package manager | `npm uninstall -g pnpm` |
| 2026-11-21 | `cargo install sqlx-cli --version =0.8.6 --no-default-features --features sqlite,rustls --locked` | SQLx offline cache | `cargo uninstall sqlx-cli` |
| 2026-11-21 | `cargo install tauri-cli --version ^2 --locked` (got 2.12.6) | `cargo tauri dev/build` | `cargo uninstall tauri-cli` |
| 2026-11-21 | `cargo install cargo-machete --locked` (0.10.1) | unused-dependency check | `cargo uninstall cargo-machete` |
| 2026-11-21 | `brew install shellcheck` | lint scripts/ | `brew uninstall shellcheck` |
| 2026-11-21 | `brew install actionlint` | lint the CI workflow | `brew uninstall actionlint` |

Note: `~/.cargo/bin` is not on the login PATH, and the shell config was left alone. Call these as `cargo sqlx`,
`cargo tauri`, `cargo machete` (cargo finds them in `~/.cargo/bin`), never as bare `sqlx`.
