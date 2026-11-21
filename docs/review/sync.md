# Final review: sync and the app shell (read-only, worktree review-sync @ plan-1 e1b15e1)

Findings are appended as reproduced.

## F1 Important: work blocks never reach Google Calendar (and existing block events are deleted)
- crates/gcal/src/reconcile.rs:196 `blocks_between(db, MIN_UTC, MAX_UTC)`; crates/core/src/blocks.rs:109 compares `julianday(?)`.
- SQLite julianday() cannot parse chrono's MIN/MAX years (-262143 / +262142), returns NULL, so the WHERE is never true.
- Repro (scratch test crates/gcal/tests/scratch.rs, deleted after): one block tomorrow; `blocks_between(MIN_UTC, MAX_UTC)` -> 0 rows;
  same block with a sane range (yesterday..+2d) -> 1 row. So `desired()` gets no blocks, a `block:<id>` event is never created, and
  the planner's Delete branch removes any that exist. DESIGN 5.4 / line 199 / 272 require work blocks as events. No reconcile test uses a block.
- Fix: in reconcile.rs use a real window (e.g. now-30d .. now+365d) or make blocks_between treat MIN/MAX as unbounded; add a reconcile test with one block.

## F2 Important: digest alert fires at 08:00 UTC, not 08:00 local, for this owner
- crates/gcal/src/reconcile.rs:206-209 reads setting `timezone`, default UTC. Nothing in the app ever writes `timezone` (grep: only docs/gcal.md says "optionally set it"), no Settings UI.
- The same app already knows the zone: app/src-tauri/src/lib.rs:44 `system_tz()` feeds reminders import; /etc/localtime on this Mac is America/New_York.
- Effect: `digest-alert:<date>` is created at 08:00Z = 03:00 EST / 04:00 EDT, so the phone buzzes in the middle of the night. All-day digest dates are unaffected (date-only, no shift); timed-due events are absolute UTC, fine; only the alert time and the "Due ..." description zone are wrong.
- Repro by arithmetic through planner::desired (zone.from_local_datetime(date 08:00)); with tz="UTC" start = 08:00Z.
- Fix: in `Real` pass `self.tz` into reconcile (or fall back to system_tz() when the setting is unset) instead of "UTC".

## F3 Important: reminder notes and due date are lost when the second step fails, yet the reminder is deleted
- crates/reminders/src/import.rs:107-121 (issue + log row committed in one tx, then notes/due applied by a separate `issues::update`), and :147-165 `delete_pending` runs regardless.
- Two ways to lose phone-captured data: (a) `issues::update` fails (SQLITE_BUSY past 5 s while MCP/CLI writes, or any error) -> only reported in `report.failed`, which `sync_loop` discards (`.await?` on a value it ignores); (b) the app dies between the commit and the update. In both cases the log row exists, so the next run skips the reminder and deletes it. The notes/due are gone from Reminders and never reach the issue.
- Repro (scratch test, removed): fake helper returns notes "IMPORTANT NOTES" + due 2026-12-01; trigger `BEFORE UPDATE ON issues RAISE(ABORT)`. Result: `failed=["a: notes or due not applied: ... upd down"]`, helper calls `["fetch --list L1","delete --list L1 a"]`, issue description "" and due NULL.
- Fix: pass description and due in the `NewIssue` given to `issues::create_in` so they land in the same transaction (drop the second update).

## F4 Important: export commits and pushes the enclosing git repo when `export_dir` is a subfolder of one
- crates/export/src/git.rs:15 `git -C <dir> add -A` then `commit`; git resolves the repo upward, so a plain subfolder of any repo is not rejected.
- Repro (scratch test, removed): outer repo with an uncommitted `unrelated-wip.txt`; `export_dir = outer/exports`; `commit_and_push` -> HEAD contains `exports/*.yaml` AND `unrelated-wip.txt`. With a remote set the push publishes it. Owner keeps everything under ~/Ark (dotfiles are a chezmoi repo), so an export folder inside a repo is likely.
- Fix: before staging, require `git -C dir rev-parse --show-toplevel` to equal `dir` (else Invalid "export folder must be the root of its own git repo"), or stage with `git add -A -- .` and commit with `-- .` pathspec.

## F5 Important: an export file round-trips wrongly; descriptions are silently truncated on restore
- crates/export/src/files.rs:19 `SECTION_COMMENTS`, :158-162 `parse_issue` splits at the FIRST "\n## Comments\n" in the body (already noted as a `shortcut:` in the source).
- Repro (scratch test, removed): description "Plan\n\n## Comments\nkeep this text and more\n\nsecond para" plus one comment, export, `rebuild` -> description "Plan\n" (everything after the heading dropped), 1 comment. Same with no comments. A markdown heading "## Comments" is a natural thing in notes, and restore is the documented recovery path (DESIGN 5.6), so this is data loss exactly when it is needed. The existing `rebuild_roundtrip` re-exports the rebuilt DB and compares trees, so it cannot see loss that is stable across both.
- Fix: put comments in their own file section that cannot occur in a description, e.g. split on the LAST "\n## Comments\n" only when what follows parses as comment headings, or escape a leading "## Comments"/"### <time> <time>" line in bodies on write. Add a roundtrip test comparing the rebuilt DB rows to the original.

## F6 Important: a packaged build contains no reminders helper
- app/src-tauri/tauri.conf.json has no `bundle.externalBin`/`resources`; lib.rs:105 expects `Contents/MacOS/reminders-helper`; sync_loop.rs:150 falls back to the repo build only under `debug_assertions`. No script builds/copies/signs the Swift binary (grep). In a release build every import step fails ("No such file") and the watcher logs to stderr every 60 s. Plan T27 builds the signed app, so it may find this, but no task owns the wiring. The Reminders permission prompt also needs the helper signed inside the bundle.
- Not reproduced by running a bundle (no signing/Xcode here); established from config and code.
- Fix: add `bundle.externalBin: ["../../helpers/reminders-helper/reminders-helper"]`, a build step that renders `swift build -c release` to that triple-suffixed name, and a QA line.

## F7 Important: backup failures are invisible
- app/src-tauri/src/sync_loop.rs:93-94 `set_state` returns Ok for `Step::Backup` without recording; `run_step` only `eprintln!`s. A GUI app has no stderr the owner reads. If backups fail (disk full, bad `backup_dir`), nothing in the UI changes and the "nightly backup" promise (DESIGN 5.8) is silently broken.
- Confirmed by reading; no test for a failing backup step. Fix: add `backup: SyncState` to `SyncStatus` (or a `backup_error` setting) and show it next to the other three.

## F8 Minor: calendar fingerprint cannot see a moved work block
- sync_loop.rs:160-165 CALENDAR_FINGERPRINT uses `count/max(start)/max(end)` of work_blocks; blocks write no history (ruling 19).
- Repro (scratch test, removed): blocks A (tomorrow 00:00) and B (+6 h); `move_block(A, +2 h)` -> poller reports `calendar=false` (a project rename correctly reports true, because it writes history). The change waits for the 15-minute tick. Only matters once F1 is fixed. Fix: add `sum(julianday(start_utc)+julianday(end_utc)+issue_id)` or `max(id)||total_changes`-style hash of work_blocks to the fingerprint.
- Ledger claim "own writes can hide an external write from the poller for one tick": NOT reproduced. The poller holds one dedicated connection, the app's own pool connections count as "other" writers to it, and `print` is only advanced when `data_version` moves, so a relevant change is signalled on the first tick that sees it (I walked through both orders of the two reads). Several commits inside one tick merge into one signal, which is correct.

## F9 Minor: the loop's export path and the command path disagree on an empty setting
- sync_loop.rs:166-180 `export()` takes `export_dir` as is; commands.rs:119-124 filters empty strings. An empty `export_dir` makes the loop write `settings.yaml`, `issues/` etc. into the process cwd and `git -C "" add -A` (empty -C is "no change") there. In a dev build that is the repo. Mutation `export_now accepts empty dir` is also uncaught (test only covers "unset"). Fix: reuse the filtered getter from commands.rs in both places.
- Also `export_now` runs outside the Syncer lock, so it can race the hourly export on git's index.lock (not reproduced).

## F10 Minor: export settings filter is a substring denylist
- crates/export/src/write.rs:96-99. Reproduced: settings named `github_api_key`, `gcal_password`, `export_credential`, `google_auth` all land in `settings.yaml` (pushed to a remote). Today the only real secrets live in the Keychain so nothing leaks now; the next key stored in settings will. `gcal_calendar_id` (often the Google account e-mail) and `reminders_list_id` are exported by design. Over-exclusion: any key containing "dir"/"path" (e.g. a future `direction`) is dropped and then not restored.
- Fix: allowlist (`issue_prefix`, `gcal_enabled`, `export_enabled`, `timezone`) instead.

## F11 Minor: restore drops work blocks, import log and change history
- Repro (scratch test, removed): issue + work block + import_log row, export, rebuild -> work_blocks 0, import_log 0, changes 0. DESIGN 5.6 lists issues/projects/properties/views only, so this is within spec, but a restore silently loses every scheduled work block and the reminder dedupe memory (reminders captured but not yet deleted would be re-imported). Fix: add `work_blocks.yaml` and `import_log.yaml`, or document the loss in docs.

## F12 Minor: OAuth listener serves one connection, so any stray connection aborts sign-in
- crates/gcal/src/auth.rs:107-135. Repro (scratch test, removed): connect to the loopback redirect port and send nothing -> `Google sign-in failed: state mismatch`. Browsers do open speculative/preconnect sockets to loopback. Fails safe (no token accepted), but the owner sees a confusing failure. Fix: loop `accept()` until a request carries `state` or `error`, within the existing 300 s timeout.
- Verified fine: PKCE S256 + random state (checked before the code is used), refresh token only handed to Keychain (`taskapp.gcal`/`refresh_token`) and never put in a log, error or DB (errors use `without_url`, status codes only); 400/401 on refresh and 401 on API calls map to DISCONNECTED and `flag()` writes the visible error state; a missing Keychain item does too.

## F13 Minor: hardening gaps in the shell
- tauri.conf.json `"csp": null`; DOMPurify is the only barrier between agent/Reminders-supplied markdown (Description.svelte, Comments.svelte use `{@html}`) and full IPC access (gcal_connect, export_now, every command). Set a CSP (`default-src 'self'; style-src 'self' 'unsafe-inline'; connect-src ipc: http://ipc.localhost`).
- `shell:default` and `tauri_plugin_shell` are enabled but nothing in ui/src imports the shell plugin; the opener plugin already covers links. Remove both.
- sync_loop.rs: every Reminders change line (EventKit emits bursts, and the helper's own delete triggers one) queues a separate full import step; there is no debounce like the calendar's. Cheap per run but unbounded in a burst.
- gcal: changing `gcal_calendar_id` leaves the app's events in the old calendar forever (rows are kept, never deleted).
- gcal PKCE test gap: replacing `code_verifier` with a wrong value is not caught (mock token endpoint does not check it); real Google would reject at the owner's first sign-in.
- Verified fine: deeplink `parse_key` (strict, uppercase prefix, <= 9 digits, test catches loosening); capabilities scope opener to obsidian/http(s); sync steps are serialized by one mutex; failed steps become visible status except Backup (F7).

## Answers to the ledger questions
- Review Focus 2: reimport after crash is safe for duplicates (issue + log row in one tx; test and mutation agree). Not safe for notes/due: F3.
- Focus 3: deleted/edited Google events are recreated/overwritten and completed/archived events removed (tests + mutations agree); revoked token sets a visible error (mutation caught). Work-block events never exist: F1.
- Focus 5: external write -> `db-changed` within one 1 s tick; the 2 s test passes and the mutation that disables the poller is caught. Moved-block blind spot: F8.
- Timezone: the all-day digest lands on the right local date (date-only, never shifted). The 08:00 alert lands at 03:00/04:00 New York time: F2.
- Reminders helper delete scope (ruling 14): verified in the real store, helpers/reminders-helper/Sources/RemindersCore/EventKitStore.swift:27-35 filters `calendarItems(withExternalIdentifier:)` by `calendar.calendarIdentifier == list`, and Commands.swift rechecks that the list exists. `swift build` compiles it. But the only tests use `FakeStore`, so nothing guards the real filter (table).
- Swift tests could not be run here: `swift test` fails with "no such module 'Testing'" (Command Line Tools only). Swift break-tests below are by inspection.

## Break-test table
| Area | Behaviour broken on purpose | Test that caught it |
|---|---|---|
| reminders | skip the already-imported check | reimport_after_crash_is_idempotent |
| reminders | delete without `--list` | imports_and_deletes |
| reminders | helper error not written to sync_status | helper_error_sets_status, invalid_json_sets_error_and_imports_nothing, imports_and_deletes |
| reminders | mark every pending id deleted whatever the helper reported | none (all 9 pass) |
| reminders | empty-title guard removed | none (quickadd rejects the same input, same report) |
| helper (Swift, by inspection) | EventKitStore.delete drops the list filter | none (tests use FakeStore) |
| helper (Swift, by inspection) | delete skips the "list exists" check | none (no unknown-list delete test) |
| helper (Swift, by inspection) | fetch returns completed reminders | fetch_excludes_completed (expected; not run) |
| gcal | completed issues keep their events | completed_issue_event_deleted |
| gcal | revoked token not flagged | revoked_token_sets_error_state |
| gcal | rows of other calendars not filtered | other_calendars_are_never_touched |
| gcal | OAuth state not checked | connect_rejects_a_forged_state |
| gcal | PKCE challenge method dropped | connect_runs_pkce_and_returns_the_refresh_token (+ forged_state) |
| gcal | wrong code_verifier at token exchange | none |
| gcal | work blocks reach the calendar (F1, not a mutation) | none, no test uses a block |
| export | "secret" removed from settings denylist | secrets_never_exported |
| export | safe_name allows "/" and "." | none |
| export | rebuild no longer refuses an existing DB | none (test only asserts an error; the later UNIQUE conflict satisfies it) |
| sync loop | step() takes no lock | syncs_never_overlap |
| sync loop | failed step not recorded in status | sync_now_reports_each_state |
| sync loop | poller never signals external commits | app_refreshes_on_external_change |
| deeplink | accepts lowercase prefix / long numbers | deeplink_parses_key |
| commands | export_now accepts an empty dir | none |
| sync loop | backup failure recorded in status (F7, not a mutation) | none, no such behaviour |

Worktree restored with `git checkout -- .`; scratch tests deleted; `git status` clean. Nothing contacted Google, the Keychain, Reminders or the real DB.
