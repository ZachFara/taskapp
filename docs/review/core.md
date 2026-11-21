# Core review (crates/core), worktree review-core @ plan-1 e1b15e1
Scratch repro tests: crates/core/tests/zz_review.rs in the review worktree (untracked, removed at the end; copy kept in .scratch/review/core-repro.rs).
All commands: SQLX_OFFLINE=true CARGO_TARGET_DIR=<worktree>/target cargo test -p taskapp-core --test zz_review <name>.

## F1 Important: writes that read first fail with "database is locked" under a second writer
Where: db.pool.begin() (deferred) then SELECT then write, in properties.rs:88,154,270,305,365,440,504 (create_def, rename_def,
reorder, set_scope, change_kind, delete_def, set_value), labels.rs:58,95, cycles.rs:142,191, comments.rs:81,122,
projects.rs:178,233,331,370, views.rs:420, pins.rs:114. issues.rs and statuses.rs use BEGIN IMMEDIATE and are fine.
Why: in WAL a deferred transaction that read and then tries to write after another connection committed gets SQLITE_BUSY at
once; busy_timeout does not apply. Review Focus 5 ("writes must wait rather than fail") is not met for these functions.
Repro (r1_deferred_tx_busy_under_two_writers): two Db handles on one file, one task each calling properties::set_value
300 times on different issues. Result: 163 and 199 of 300 calls failed with "(code: 5) database is locked"
(second run: 0 and 300). tests/issues.rs concurrent_writers_do_not_error only covers issues::create, which is IMMEDIATE.
Smallest fix: replace db.pool.begin() with db.pool.begin_with("BEGIN IMMEDIATE") in those 21 places (or one Db::begin_write()).

## F2 Important: backup::restore_live can never succeed from the app
Where: backup.rs:85-94 with 136-165. restore_live asks "no other connection has the file open", but the db it is given is the
app's own pool, which holds (at least) the connection it just used for the pragma query. Commands crate backup_restore
(commands/src/lib.rs:256) calls it with &s.db.
Repro (r4_restore_live_with_own_pool): open file Db, snapshot, restore_live(&db, &snap) -> Err(Conflict("the database is open
elsewhere; close the app before restoring")). No test calls restore_live (tests/backup.rs only covers restore()).
So the Settings "restore" button is dead; only the CLI (backup::restore with the app closed) works.
Smallest fix: restore_live should be removed from the app path and the UI told to use the CLI with the app closed, or the app
must close its pool and exit/reopen around the call; add a test that calls restore_live.

## F3 Important: restore accepts any SQLite file and keeps no copy of what it overwrites
Where: backup.rs:232-251 (check_backup only runs integrity_check), 102-132.
Repro (r5_restore_accepts_foreign_sqlite): restore(<sqlite file with one table "notes">, <live.db with an issue>) -> Ok(());
the live database is replaced and the issue is gone; Db::open of the result also returns Ok (migrations run onto the foreign
file). Also no pre-restore copy of the current database, so a mistaken restore is unrecoverable (anti-goal "no way back").
Smallest fix: in check_backup also require the _sqlx_migrations table and an issues table; before the rename, rename the
current `to` to `to.before-restore` (rename, not delete).

## F4 Important: pins are deleted by a read, and survive only while their name-based target is unchanged
Where: pins.rs:140-226 (resolve by project slug and issue key; list_resolved DELETEs unresolved pins).
Repro (r3_pins_vanish_on_slug_rename_and_prefix_change): pin project "home" and issue ZF-1; rename the project slug to
"house" via projects::update -> next list_resolved returns 1 pin; set issue_prefix to "AB" -> next list_resolved returns 0,
and the pins table is empty (rows deleted). Order and titles are lost with them.
Design 5.9 says a pin to something deleted disappears; a renamed slug or changed prefix is not a deletion.
Smallest fix: pin projects by id and issues by number (route built at resolve time); if keeping names, make projects::update
rewrite pins.target on slug change and do not delete on prefix mismatch.

## F5 Important: views on the fixed field DueDate ignore every timed due issue
Where: issues.rs:164-172 (due_cols writes due_date only for Due::Date), views.rs:230 (FixedField::DueDate = issues.due_date).
Repro (r2_due_date_filter_misses_timed_due): one issue due 2026-12-01 14:00 UTC (timed) and one due Date 2026-12-01;
filter DueDate Lte "2026-12-31" returns only the date-only one. Sorting by DueDate puts timed issues in the NULL group, and
IsEmpty on DueDate matches them. Overdue/due-this-week views silently omit timed items.
Smallest fix: in due_cols also set date = utc converted to tz (fall back to the UTC date), keep due_time_utc/due_tz as they are.

## F6 Important: issues::update replaces the whole properties map and never validates it
Where: issues.rs:320 (apply_patch), 248 (create). IssuePatch.properties is "replace", not merge, and properties::validate is not called.
Repro (r13_patch_properties_replaces_whole_map): issue with {a:1,b:2}; patch properties {a:9} -> properties is {a:9}; b is gone.
(r8b) a key with no definition and any JSON shape ({"any":["thing"]}) is accepted through update. MCP update_issue/CLI pass this
through, so an agent that sends one key wipes the rest, and typed/option/scope rules (Review Focus 4) hold only on set_value.
History keeps the old map, so it is recoverable by hand. Unknown keys are wanted for migration (linear_id), so do not reject them.
Smallest fix: merge the patch map into the old one (null value removes a key) and run validate() for keys that have a definition.

## F7 Important: parent cycles are accepted
Where: issues.rs:394 only rejects parent == self. Repro (r8a): a.parent=b then b.parent=a both Ok. Relations::add has a cycle
check for Blocks; parent has none. Any roll-up over parent_id (Gantt roll-up bars, children trees) can loop.
Smallest fix: same recursive-CTE reach check as relations.rs:39 before the UPDATE.

## F8 Minor: issue_prefix (all settings) accepted without validation
Where: settings.rs:20, commands settings_set(any key, any JSON). Repro (r9): settings::set issue_prefix 5 -> every issues::get
returns Invalid("invalid type: integer `5`, expected a string") (app unusable until the row is fixed); prefix "" -> keys "-1".
Smallest fix: in settings::set reject issue_prefix unless it matches ^[A-Z][A-Z0-9]{0,9}$ (and mention that a prefix change orphans issue pins, see F4).

## F9 Minor: statuses::delete leaves completed_at wrong and always records source System
Where: statuses.rs:146-183. Repro (r6): issue in In Progress, delete that status moving to Done -> issue is in Done with
completed_at = None (the reverse leaves a stale value). update_in has the logic; this bypasses it. updated_at is not touched
either. statuses create/update/delete take no ChangeSource and create/update write no history (see F15).
Smallest fix: when categories differ set/clear completed_at in the same UPDATE; take a ChangeSource argument.

## F10 Minor: views::save validates nothing (ledger question on group_by, columns, label sort)
Where: views.rs:369. Repro (r7): save a view sorted by Label -> Ok, then views::query -> Err("invalid: cannot sort by label"), so the
saved view can never open until edited. A view with filter/group_by/columns on an unknown property also saves Ok (compile only
checks filter and sorts at query time; group_by and columns are never checked by core).
Verdict: label sort being Invalid is fine (no sensible order) but it must be refused at save time, not at open. group_by Label
cannot be compiled either, grouping is done in the UI. Smallest fix: call compile(&def, &property_defs) in save() and also check
group_by/columns property keys against defs.

## F11 Minor: milestone is not checked against the issue's project
Where: issues.rs create_in/update_in (no check). Repro (r11): issue in project B with a milestone of project A is accepted;
projects::delete(A) then sets that issue's project_id (B) to NULL as well (projects.rs:216-223 clears project_id for the
milestone match). History does record it. Smallest fix: reject milestone whose project != issue project, or limit the UPDATE's
project_id clear to project_id = pid.

## F12 Minor: Due::DateTime tz is not validated
Where: issues.rs due_cols / check_fields. Repro (r8c): tz "Not/AZone" accepted via update. schedule.rs:14 already carries a
"shortcut: ... reject it at write time" comment, so the limit is live. Smallest fix: parse tz with chrono_tz in check_fields.

## F13 Minor: three different "note link" rules (ledger question on pins vs doc_link)
properties::validate NoteLink (properties.rs:578): obsidian:// OR any path ending .md, so "javascript:alert(1)//x.md" and
"/etc/x.md" pass (r8d). projects::valid_doc_link: obsidian://, https://, or vault-relative .md without ':' '/' '..'.
pins Note: obsidian:// only. Design 5.9 pins "Obsidian notes and web links" and 5.1 says the project link is to an Obsidian
note, so pins being strictest is consistent with the design; a vault-relative path cannot be opened or pinned without a vault
root, so refusing it in pins is right. The loose one is NoteLink (not opened by the UI today, so not exploitable yet).
Smallest fix: reuse projects::valid_doc_link in NoteLink (or require obsidian:// and https://).

## F14 Minor: quick-add bare date words eat title words (ledger question on grammar rulings)
Where: quickadd.rs:167 (bare date_at), 152-159 (last wins). Repro (r10, parse with today = Fri 2026-11-20):
"fix sun glare" -> title "fix glare", due 2026-11-22; "email sat about mon" -> due 2026-11-23, title "email about";
"buy 2 mar" -> title "buy", due 2027-03-02; "sort 3 dec files" -> due 12-03. "by fri by mon" and "p2 p3" take the last.
The design example "renew lease by fri p1 #ohio-move" parses correctly (existing test full_example). Timed due and DST are fine:
(r15) 02:30 on 2026-03-08 New York (nonexistent) keeps "at 2:30am" in the title with a date-only due; 3:30am gives 07:30Z.
Verdict: last-token-wins is fine; bare weekday and month words are the risk (silent, no undo besides editing). Smallest fix:
require by/due/on before weekday and month-day phrases, keep today, tomorrow, ISO date and "in N days" bare.

## F15 Minor: "every mutation writes change history" is not literally true (ledger question on ruling 19)
No history, no ChangeSource: blocks.rs (create/move/delete work block), statuses create/update, views save/delete/reorder,
pins pin/unpin/reorder, properties::reorder, settings::set. Ruling 19 is defensible for work blocks (calendar placement, not an
issue field, UI-only today: no mcp/cli caller) but the Global Constraint and design 5.1 ("every change to issues and properties")
differ in scope. Either amend the constraint to "every change to issues, properties and the entities that take a ChangeSource"
or add rows. Risk today is low; the one that matters is statuses (F9) because an agent can change workflow.

## F16 Minor: unique and foreign-key violations surface as Error::Db, not Conflict/NotFound
Repro (r16): duplicate project slug, duplicate label name, work block for a missing issue, comment on a missing issue all give
kind Db with the raw SQLite text. statuses has unique_to_conflict; projects, labels, comments, blocks, milestones, create_def
(race), set_scope (unknown project) do not. Smallest fix: map is_unique_violation/is_foreign_key_violation in one helper.

## Answers to the other ledger questions
- Review Focus 1 holds. Date-only is stored as TEXT date and round-trips (date_only_due_never_shifts); a timed due stores utc
  plus IANA tz so wall-clock survives DST by construction, and quickadd computes local time with chrono-tz (r15: gap and
  normal case behave). Caveat: due_datetime_survives_dst builds the UTC value itself, so it proves storage, not conversion;
  the conversion test is the quickadd one. F5 and F12 are the only DST-adjacent defects.
- Review Focus 4 holds for rename (key is the identity, name-only update), kind change (convert or refuse listing up to 20
  keys, nothing written on refusal, r14 shows narrowing options also refuses) and delete (values removed, view filters/sorts/
  columns/group_by stripped). Gap: F6 (update path bypasses types).
- Restore guard (backup.rs:136): sound for its purpose. r17: a separate process (sqlite3 CLI) holding live.db open and idle makes
  restore() return Conflict; after it exits restore() succeeds. WAL connections keep a shared lock, so the refused journal-mode
  switch is a real signal and both result shapes (unchanged mode, BUSY) are handled. Limits: TOCTOU between ensure_unused and
  fs::rename (another process can open in between); ensure_unused itself switches a free live file to DELETE mode (harmless,
  Db::open sets WAL again); and F2 (the app's own pool trips it) and F3.

## Break tests (each: edit in place, run that area's test file(s), git checkout; --cap-lints warn so unused-variable lints do not mask results)

| Feature | Behaviour broken | Test that caught it |
|---|---|---|
| issues | history written on update | update_writes_one_change_per_field |
| issues | completed_at set on completing | completing_sets_completed_at_and_reopening_clears_it |
| issues | create takes write lock up front (IMMEDIATE) | concurrent_writers_do_not_error |
| issues | self-parent refused | none |
| statuses | delete into itself refused | delete_status_into_itself_is_invalid |
| statuses | delete records a change per moved issue | none |
| statuses | color validated | none |
| properties | change_kind refuses unconvertible values | change_kind_lists_at_most_20_keys, change_kind_refuses_and_lists_keys, refused_change_kind_writes_no_history |
| properties | select value must be an option | select_value_not_in_options_rejected, validate_each_kind |
| properties | delete_def cleans saved views | delete_def_removes_values_and_cleans_views, delete_def_single_condition_view_has_no_filter |
| properties | scoped def rejects other project | project_scoped_def_rejects_other_project |
| projects | doc_link refuses ':' in paths | none |
| projects | delete refuses while scoped defs exist | project_delete_refuses_while_scoped_defs_exist |
| projects | slug may not start with hyphen | project_slug_validation |
| cycles | overlap refused on create | overlapping_cycles_rejected |
| cycles | end before start refused | overlapping_cycles_rejected |
| cycles | overlap refused on update | overlapping_cycles_rejected |
| labels | add_label records history | label_add_remove_records_history |
| labels | remove_label records history | label_add_remove_records_history |
| labels | update records changed fields | none |
| comments | add reindexes search | comment_edits_reach_the_search_index |
| comments | edit records history | comment_accepts_migration_timestamp |
| comments | delete reindexes search | comment_edits_reach_the_search_index |
| blocks | end must be after start | work_block_end_before_start_rejected |
| blocks | move of unknown block is NotFound | none |
| blocks | delete of unknown block is NotFound | none |
| relations | blocks cycle refused | blocks_cycle_rejected |
| relations | self relation refused | self_relation_rejected |
| relations | add records history | duplicate_add_is_idempotent, remove_deletes_and_records_once |
| schedule | dependents shift only when overlapping | schedule::tests::shift_ignores_unaffected_gap, schedule::tests::shift_never_pulls_earlier |
| schedule | timed due uses local date | schedule::tests::bar_uses_local_date_of_timed_due |
| schedule | 1 day = 480 minutes | schedule::tests::bar_from_estimate |
| schedule | critical path ignores stored cycle (on_path guard) | none |
| views | filter depth limit | none |
| views | property key with quote refused | none |
| views | archived excluded by default | archived_excluded_by_default, filter_fixed_and_property, in_with_empty_list_matches_nothing, sort_property_nulls_last |
| views | reorder needs full set | reorder_missing_or_duplicate_id_is_invalid |
| search | query tokens quoted | search_tolerates_punctuation |
| search | archived issues not searchable | archived_issue_is_not_found_after_reindex |
| search | create reindexes | create_and_update_keep_search_current |
| quickadd | ambiguous local time picks earliest | none |
| quickadd | priority limited to 0-4 | quickadd::tests::p5_and_bare_p_stay_in_title |
| quickadd | same weekday means next week | quickadd::tests::friday_on_friday_is_next_week, quickadd::tests::full_example |
| quickadd | unknown project token stays in title | quickadd::tests::unknown_project_kept_in_title |
| pins | note must be obsidian link | note_must_be_obsidian_link |
| pins | url scheme allow-list | url_scheme_must_be_http_or_obsidian |
| pins | archived issue flagged | archived_issue_pin_kept_and_flagged |
| pins | reorder needs full set | reorder_duplicate_id_invalid, reorder_requires_full_set |
| backup | integrity check gates restore | none |
| backup | restore refuses while db open | restore_refuses_while_another_connection_holds_the_db |
| backup | stale WAL/shm removed before restore | none |
| backup | prune keeps newest 14 nightly | prune_keeps_14_and_30 |
| backup | nightly written via .partial then rename | none |
| history | old value stored | history_record_and_list |
| history | new value stored | history_record_and_list |
| history | source stored | history_record_and_list, update_writes_one_change_per_field |
| settings | set stores the value | settings_roundtrip |
| settings | get returns stored value | migrations_seed_statuses_and_prefix, settings_roundtrip |
| settings | get of invalid json is an error | none |
| migrations/db | WAL mode | open_sets_wal_and_busy_timeout |
| migrations/db | busy_timeout 5000 | concurrent_writers_do_not_error, open_sets_wal_and_busy_timeout |
| migrations/db | foreign keys on | none |
| migrations/db | relations unique edge | duplicate_add_is_idempotent |
| migrations/db | priority CHECK 0-4 | none |
| migrations/db | status category CHECK | none |
| migrations/db | foreign keys (in-memory) | unknown_issue_is_not_found |

66 mutations, 18 not noticed by any test. Not noticed:
- issues: self-parent refused
- statuses: delete records a change per moved issue
- statuses: color validated
- projects: doc_link refuses ':' in paths
- labels: update records changed fields
- blocks: move of unknown block is NotFound
- blocks: delete of unknown block is NotFound
- schedule: critical path ignores stored cycle (on_path guard)
- views: filter depth limit
- views: property key with quote refused
- quickadd: ambiguous local time picks earliest
- backup: integrity check gates restore
- backup: stale WAL/shm removed before restore
- backup: nightly written via .partial then rename
- settings: get of invalid json is an error
- migrations/db: foreign keys on
- migrations/db: priority CHECK 0-4
- migrations/db: status category CHECK

Notes on the misses: issues self-parent, statuses delete history and colour, labels update history, blocks NotFound on move/delete, views depth limit, quickadd earliest-of-ambiguous-time, backup integrity_check / stale WAL removal / .partial rename, settings invalid JSON, PRAGMA foreign_keys on the file-backed open (tests use in-memory), priority and status-category CHECKs, doc_link ':' rule, critical-path cycle guard are all unprotected by a test. views 'property key with quote' guard is unreachable behind the earlier defs lookup (harmless duplicate). backup integrity check: restore_rejects_corrupt_file still passes without it because SQLite refuses to open the garbage file; a file that opens but fails integrity_check is never tested.
