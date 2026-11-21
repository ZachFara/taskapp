# Final review: agent and CLI surface (Sonnet, read-only)
Worktree <workspace>/taskapp-wt/review-agents (plan-1), target dir ../review-agents-target. Findings are appended as found.

## Findings

### 1. Important: CLI snapshots go to a folder the app never lists or prunes
crates/cli/src/main.rs:258-261 (`dir` = db parent) and crates/cli/src/commands.rs:246-248; MCP uses `<db dir>/backups` (crates/mcp/src/main.rs:66-68); the app uses `backup_dir` setting or `<data dir>/backups` (app/src-tauri/src/lib.rs:108-110).
`backup::snapshot` writes `<dir>/snapshots/`, so the CLI's pre-change snapshot lands in `<data dir>/snapshots/`, not `<data dir>/backups/snapshots/`. The app's backup list, `backup::prune` and the snapshot-watching export all look only under `backups/`.
Repro: `taskapp --db D/taskapp.db property create linear_id X text; taskapp --db D/taskapp.db property delete linear_id` -> file `D/snapshots/<ts>-delete-property.db`; `taskapp --db D/taskapp.db backup list --dir D/backups` prints `(none)`. Each CLI snapshot is a full DB copy that is never pruned; the user will not find it in Settings. The `backup_dir` setting is also ignored by both MCP and CLI.
Fix: CLI uses `<db dir>/backups` like MCP (one shared helper in core would be better: read the `backup_dir` setting, else `<db dir>/backups`). The CLI test `property_change_kind_snapshots_first` asserts the wrong folder (`dir/snapshots`) and must change with it.

### 2. Important: CLI cannot do the Linear migration (spec 5.7 says MCP and CLI both must)
crates/cli/src/main.rs IssueCmd/CommentCmd, crates/cli/src/commands.rs:78-100, 112.
Spec 5.7: "the MCP server and CLI must be able to create every kind of record ... and must accept original created and completed timestamps. The original Linear ID is kept as a property." CLI `issue create` has no created_at, completed_at, properties/linear_id, parent, cycle, milestone, labels or start date; `comment add` passes `None` for created_at; no `status create`, `milestone create`, `project update` (doc_link, dates), and no way to record the Migration source (SRC is fixed to Cli). Repro: `taskapp issue create --help` lists only title, description, priority, status, project, due, estimate.
MCP covers issues (created_at, completed_at, properties incl. linear_id, parent_id, cycle_id, milestone_id, label_ids, migration:true) and comments (created_at). I confirmed create_issue + migration:true records source Migration (existing test `create_issue_with_migration_timestamps`).
Fix: either state in PROGRESS that migration is MCP-only (the runbook already says MCP), or add `--created-at --completed-at --property k=v --parent --cycle --milestone --label --migration` to `issue create`, and `--created-at` to `comment add`.

### 3. Important: CI `changes` job treats a rename into docs/ as docs-only, so test jobs are skipped
.github/workflows/ci.yml:31 (`git diff --name-only`), scripts/docs-only.sh.
`git diff --name-only` reports only the new path of a rename. Repro in a scratch repo: `git mv crates/core/src/lib.rs docs/lib.md`, commit; `git diff --name-only HEAD~1...HEAD` prints `docs/lib.md` only; `docs-only.sh` prints `true`. With docs_only=true test-core, test-rest, test-ui and test-swift are skipped and `check` counts skipped as pass. (`lint` still builds, so only test-only breakage slips through, e.g. moving a test file into docs/ or `.briefs/`.)
Fix: `git diff --no-renames --name-only`. Add a docs-only self-test for it is not possible in docs-only.sh itself (it sees only paths); document the flag.

### 4. Minor: devserver real-db refusal is bypassed by a hard link
crates/devserver/src/main.rs:51-58 (`refuse_real_db` compares resolved paths with `starts_with`).
Repro (fake HOME): `ln "$HOME/Library/Application Support/taskapp/taskapp.db" other/hard.db; taskapp-devserver seed --db other/hard.db` -> seeded; `sqlite3 other/hard.db 'select count(*) from issues'` = 20; the real inode got a second WAL (`hard.db-wal`), which corrupts a running app's DB. Symlink, `..`, relative, and case variants (`library/application support/TASKAPP`) are all refused (verified). Needs the owner to create a hard link, hence Minor.
Fix: also refuse when the file's (dev, ino) equals the real db's (MetadataExt), or refuse any db with st_nlink > 1.

### 5. Minor: `shell:default` capability and plugin are unused and widen what the webview can open
app/src-tauri/capabilities/default.json:7, app/src-tauri/src/lib.rs:97, app/src-tauri/Cargo.toml:27. tauri.conf.json has `"csp": null`.
No UI code imports @tauri-apps/plugin-shell (ui/package.json has only plugin-opener), but `shell:default` grants `allow-open` for http(s), tel: and mailto: links, a second route that skips the scoped `opener:allow-open-url` list (the plugin's own default.toml says so). With `csp: null` any injected script can call it.
Fix: remove the shell plugin, its permission and the Cargo line. (Set a CSP: owner decision, UI reviewer's area.)

### 6. Minor: `create_status`, `pin`, `unpin` (MCP) and their CLI twins write no change history, so no source is recorded
crates/mcp/src/tools.rs:416-439 (create_status, pin, unpin); core `statuses::create` and `pins::pin/unpin` take no ChangeSource.
Repro: MCP `create_status {"name":"Rev","category":"Started","color":"#ff0000"}` then `select * from changes` shows no row (project, milestone, cycle, label, issue, comment, property_def all show `"Agent"`). Global Constraint: every mutation writes history with a ChangeSource; the migration creates statuses, so those are untraceable. Fix: add a ChangeSource arg to statuses::create (core owner) or put it on the follow-up list.

### 7. Minor: snapshot rule is per call, so a loop of `update_issue` / `archive_issue` is unprotected
crates/mcp/src/tools.rs:320 (`items.len() > 10` only in bulk_update). Repro: 50 sequential `archive_issue` calls take no snapshot (verified by absence of `backups/snapshots`). This is the plan's stated rule (ruling: "bulk_update > 10"), so it is a known limit; cheapest hardening is one snapshot per MCP session before the first write that follows 10 writes. Also MCP `bulk_update` snapshots before validating items (test `..._applies_nothing_but_snapshots` pins this, harmless).

### 8. Minor: CLI snapshots before validating its arguments
crates/cli/src/commands.rs:166-170: `property change-kind K badkind` takes a snapshot, then errors on the kind. MCP orders it correctly (unknown key fails before the snapshot). Fix: parse kind first.

### 9. Important (test gap): nothing tests that a failed snapshot aborts the destructive change
crates/mcp/src/tools.rs:213-216 (`snapshot`) and crates/cli/src/commands.rs:246-249.
Current code is correct: with `<db dir>/backups` being a file, MCP `delete_property` returns `Io: Not a directory` and the property survives (verified by hand with a rebuilt binary). But mutating `snapshot` to ignore the error (`let _ = ...; Ok(())`) is caught by NO test in either crate (see break table). A later edit could silently turn "snapshot then delete" into "delete even when the snapshot failed".
Fix: one test per adapter: make the backup folder unwritable (a file named `backups` for MCP; for the CLI a file named `snapshots` in the db dir), call delete_property, assert error and that the property still exists.

### 10. Minor (test gaps in CLI): source and delete-property snapshot are not asserted
crates/cli/tests/cli.rs. Changing `SRC` to `ChangeSource::Ui` (every CLI write then records the wrong source) and removing the snapshot from `property delete` both pass all 11 CLI tests (see break table). Only `change-kind` is tested for its snapshot, and nothing reads the `changes.source` column for CLI writes (the MCP suite has `writes_record_agent_source`).
Fix: add `cli_writes_record_cli_source` (run `add`, then `history ZF-1 --json`, assert source "Cli") and extend the snapshot test to `property delete`.

### 11. Minor: `backup restore --yes` overwrites the current database without keeping a copy of it
crates/cli/src/main.rs:~95-105 -> core backup::restore. Verified in a fake HOME: restore replaces the file in place; the previous contents are gone. A snapshot of the current db before restoring would make a wrong restore recoverable. Restore itself is correct otherwise: refuses without --yes (error names the real target path), default target is `~/Library/Application Support/taskapp/taskapp.db`, refuses while another connection has it open, fails when the target directory does not exist.

### 12. Minor (test gaps, scripts and devserver)
- scripts/land.sh: no test for a `cancel` bucket check (land.sh:~29), for `--match-head-commit`, for `--admin` being absent, or for update-branch failing for a reason other than "up to date" (the fake gh always says "already up to date"). Removing any of them keeps test-land.sh green; each one is a way for a red or stale PR to merge. Fix: add fake-gh cases `check` bucket cancel -> rc 1 no merge; update-branch error text "conflict" -> rc 1; assert the logged merge line contains `--match-head-commit abc` and not `--admin`.
- crates/devserver/tests/server.rs: refusal of the real db is only tested for `serve`, not `seed`; making `seed` skip `refuse_real_db` passes every test, and `seed --db <real>` would write 20 fixture issues into the owner's database. Add `seed_refuses_real_app_db`. Failing refusal tests hang instead of failing (the child server keeps running, `command().output()` waits forever); give `refused` a timeout.
- ui/src/lib/openExternal.test.ts: removing the `^` anchor from the allow-list regex passes all tests, and `javascript:alert(1)//https://x` then returns true. The Tauri opener scope (`https://*` etc.) is a second layer that would still refuse it, but no test reads app/src-tauri/capabilities/default.json, so widening that scope (e.g. adding `file://*`) is also unnoticed. Add a test that parses default.json and asserts the exact scope list, and one unanchored-bypass string.
- Allow-list notes (no defect): `http://` is allowed in both layers (pins accept it, project doc_link only obsidian/https, the UI header accepts only those two). `obsidian://*` is any Obsidian URI, so an agent-written pin or doc_link can make one click run an Obsidian URI action; accepted by design, worth a line in docs.
- CI aggregator (`check` job) was checked by hand: it fails on `failure` and `cancelled`, treats `skipped` as pass, and lists all nine jobs. No test exists for ci.yml itself.

## Questions the ledger set aside: answers
- Snapshot rule: MCP bulk_update > 10 (exactly 10 takes none), change_property_kind, delete_property snapshot first, fail closed, and an unknown key fails before the snapshot for the property tools (bulk_update snapshots first, tested as intended). CLI snapshots for change-kind and delete but into the wrong folder (finding 1). Sources: MCP Agent/Migration, CLI Cli, except statuses and pins (finding 6).
- Migration: MCP yes for issues and comments with timestamps and linear_id; CLI no (finding 2).
- Devserver: Host check, 415 on non-JSON, CORS only localhost:5173, loopback bind, symlink/relative/case variants refused, sync commands absent; hard link bypass (finding 4); seed refusal untested (12).
- openExternal and capability: see 5 and 12.
- backup restore: needs --yes, default target is the real db (verified under a fake HOME), no copy of the replaced db (finding 11).
- CI/scripts red-PR paths: finding 3 (rename into docs/ skips tests) and the land.sh test gaps in 12. land.sh logic itself stops on fail/cancel/pending, needs the `check` job, rejects drafts and non-open PRs, and merges with `--match-head-commit`.

## Break-test table (code edited in place, tests run by name or file, then `git checkout -- .`)
| Behaviour broken | Test that caught it |
|---|---|
| MCP bulk_update snapshot threshold removed | bulk_update_over_10_snapshots_first, bulk_update_with_unknown_key_applies_nothing_but_snapshots |
| MCP delete_property snapshot removed | delete_property_snapshots_first |
| MCP change_property_kind snapshot removed | change_property_kind_snapshots_first |
| MCP snapshot failure swallowed (change proceeds) | none |
| MCP unknown-key check removed before snapshot | unknown_property_snapshots_nothing_and_errors |
| MCP source always Ui | writes_record_agent_source |
| MCP migration flag ignored | create_issue_with_migration_timestamps |
| MCP bulk_update not atomic | bulk_update_failing_item_leaves_earlier_items_unchanged |
| CLI source Ui instead of Cli | none |
| CLI delete-property snapshot removed | none |
| CLI change-kind snapshot removed | property_change_kind_snapshots_first (+3 backup tests) |
| CLI snapshot failure swallowed | none |
| CLI restore --yes check removed | backup_restore_without_yes_changes_nothing |
| CLI restore ignores --db (uses default target) | backup_restore_without_yes_changes_nothing, backup_restore_replaces_db (this run aborted on a missing dir; the real app dir does not exist on this Mac, nothing was written) |
| devserver Host check allows any host | rejects_foreign_host |
| devserver refusal never matches | all refusal tests hang (no clean failure) |
| devserver symlink resolution removed | refuses_real_app_db_via_symlink, refuses_real_app_db_via_relative_path |
| devserver CORS any origin | cors_allows_only_the_dev_origin |
| devserver binds 0.0.0.0 | binds_localhost_only (+4 others) |
| devserver seed skips refusal | none |
| openExternal check removed | refuses javascript/file/data/vbscript/empty cases (6 failures) |
| openExternal check skipped inside Tauri | refuses in Tauri too, before the plugin is reached |
| openExternal http dropped | hands obsidian and http links to window.open outside Tauri |
| openExternal regex anchor removed | none |
| Tauri opener scope widened | none (no test reads capabilities) |
| land.sh failed-check stop removed | land refuses a non-green PR |
| land.sh draft check removed | land refuses a draft PR |
| land.sh open-state check removed | land refuses a closed PR |
| land.sh `check` job requirement removed | land refuses when the aggregate check is missing |
| land.sh pending ignored | land waits while pending |
| land.sh timeout merges instead of exiting 2 | three timeout tests |
| land.sh cancel bucket not a failure | none |
| land.sh --match-head-commit removed | none |
| land.sh --admin added | none |
| land.sh update-branch failure ignored | none |

Not run: full suites (only the named test files), real gh, real PRs, CI. The worktree is clean (`git status` empty). Scratch material: <scratch>
