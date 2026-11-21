# QA checklist

Two parts: checks an agent ran on a throwaway database (results recorded), and checks only the owner can run
(marked **owner**: they need the signed app, Reminders, Google, or a real repo).

## Packaging (automated)

- `scripts/build-helper.sh` builds the Swift helper in release and copies it to
  `app/src-tauri/binaries/reminders-helper-<target-triple>` (gitignored). `bundle.externalBin` in `tauri.conf.json`
  names `binaries/reminders-helper`; Tauri strips the triple, so the helper sits next to the executable, where the
  app looks first (`reminders-helper`).
- Result: `pnpm -C ui build`, then `cargo tauri build --debug --no-bundle` (from `app/src-tauri`) finished and
  `target/debug/` holds both `taskapp-app` and `reminders-helper`. Not signed, notarized or installed.

## Automated, on a temp database (2026-11-21)

Database: `taskapp-devserver seed --db <temp>`. Real-mode harness: `pnpm -C ui shot <route> --mode real`
(`SQLX_OFFLINE=true` exported). PNGs are in `ui/harness-out/` (gitignored); each capture reported 0 problems.

| Route | Result | PNG |
|---|---|---|
| /view/1 | pass, 8 active issues listed | ui/harness-out/view-1.png |
| /view/2 | pass | ui/harness-out/view-2.png |
| /view/3 | pass | ui/harness-out/view-3.png |
| /view/1/Table | pass | ui/harness-out/view-1-Table.png |
| /view/1/Calendar | pass | ui/harness-out/view-1-Calendar.png |
| /issue/ZF-1 | pass | ui/harness-out/issue-ZF-1.png |
| /settings/properties | pass | ui/harness-out/settings-properties.png |

Defect found and fixed: every `views_query` failed with `unknown variant "Select"` against a real database, so
every view rendered empty. Property kinds are stored JSON-encoded; `views::property_defs` parsed the bare string.
The test helper in `crates/core/tests/views.rs` stripped the quotes and hid it. The helper now stores what
production stores (test failed first with that message), and `views.rs` parses with `from_str`.

| Check | Result |
|---|---|
| `taskapp --db <temp> add "QA cli row p1"` and MCP `create_issue` ("QA mcp row") on the same db | both rows present (ids 21, 22) |
| `taskapp property delete effort` (snapshots first), then `taskapp backup restore <snapshot> --yes` | `effort` back (1 row), 22 issues; restore without `--yes` refuses |

## Owner (needs the signed app or real accounts)

Build and install: `scripts/build-helper.sh`, then `cargo tauri build` with signing set up, install the app.

1. **Siri reminder import (owner).** In Settings, grant Reminders access and pick the list. Tell Siri "remind me to
   test import" into that list. Expect: within one minute an issue "test import" appears in the app, and the
   reminder is gone from the Reminders list.
2. **Google phone notification (owner).** Follow "One-time Google setup" in `docs/gcal.md`, connect, pick the
   calendar. Create one issue with a date-only due date and one with a timed due date. Expect: both events in the
   chosen calendar; the timed one notifies on the phone via the Google Calendar app at the due time, the date-only
   one gets the 08:00 digest alert. Delete an app event in Google, press sync now: it returns. Complete the issue: its
   event disappears.
3. **Digest probe (owner).** Run the three steps in `docs/gcal.md` "Digest reminder probe" on a test calendar and
   write the result there. Expect: either the fallback stays, or `digest_reminder_minutes()` is changed.
4. **Real export repo (owner).** Create a private empty repo, set the export directory and remote in Settings,
   turn export on. Expect: a commit per change in the repo. Then rebuild into a scratch database with
   `taskapp_export::rebuild::rebuild(<export dir>, <scratch.db path that does not exist>)` (no CLI command; the
   `rebuild_roundtrip` test in `crates/export/tests/export.rs` shows the call). Expect: same issues, properties, comments.
5. **Signed app install (owner).** Copy the signed app to Applications, open it. Expect: launches without a
   Gatekeeper block; the Reminders prompt appears once; `taskapp add "x"` and an MCP write appear in the open
   app within 2 seconds.
6. **By hand in the app (owner).** Quick-add and the command palette; create and change a property of every kind,
   including a refused type change; views in all five layouts, a multi-project timeline and projects mode; pin an
   issue, a view and an Obsidian note and reorder them; a project with only an Obsidian link opens the note; a Gantt
   drag that cascades; restore from a snapshot with the app quit.
