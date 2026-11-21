# 0005 Tool installs

Context: the owner's Mac is in real use; installers must add to a setup and never overwrite it.

Decision: agents do not install tools on their own. `scripts/install-tools.sh` prints what it will do, skips tools already
on PATH, asks before running, and is the reproducible record; every install that is run is logged in docs/INSTALLS.md
with date, command, why and how to undo. The owner has since allowed installs; the setup agent still installed nothing.
Until pnpm is installed use `npx -y pnpm@10`; CI installs its own tools.

Consequences: local runs of sqlx-cli, tauri-cli, cargo-machete and shellcheck need that script first.
