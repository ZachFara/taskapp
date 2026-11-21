# 0002 Import boundaries

Context: core owns the database and every domain operation; everything else is an adapter or a sync crate.

Decision: enforced by `scripts/check-boundaries.sh` from `cargo metadata`. core depends on no workspace crate.
mcp, cli, commands, gcal, reminders and export depend only on core. devserver depends on core and commands.
Only `app/src-tauri` may depend on the sync crates. Consequence for the plan: `gcal_*`, `reminders_*` and `sync_*`
commands cannot live in `crates/commands`; they are Tauri wrappers in `app/src-tauri` and the devserver does not
serve them (it never runs sync anyway).

Consequences: a new edge needs a change to the allowed table in the script and an owner decision. The check reads
manifests, so a `use` of a crate that is not declared fails to compile instead.
