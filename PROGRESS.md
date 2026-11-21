# PROGRESS

## Status
Dated 2026-11-21. Phase 1 merged (PR #1, #2). Public repo github.com/ZachFara/taskapp, main protected, plan-* ruleset
active. Plan 1 running on `plan-1`: T0 in CI; T1-T11 in progress on branches that merged `task/T0`.

## Models
Subagents: about half Sonnet, half Haiku, by difficulty (Haiku: steps can be given; Sonnet: judgment). Pass the model
explicitly. Never Opus. A stuck Haiku brief is handed to Sonnet once and noted here.

## Tasks (plan docs/plans/plan-1.md)
Done criterion for every task: its listed tests pass in CI, `todo!`s in its files gone, gate green, PR merged to plan-N.

| Task | What | Done when | Model | Reason |
|---|---|---|---|---|
| T0 | Workspace, schema, types, stubs, Tauri commands | foundation tests (5) pass, workspace builds, bindings.ts generated, machete ignore lists removed, sqlx cache committed | Sonnet | cross-package contracts, everything depends on it |
| T1 | UI skeleton, api layer, mock, routes | api.mock and keymap tests pass, `pnpm -C ui build` works | Sonnet | three transports, shared contracts |
| T28 | Dev harness (devserver, shot) | devserver and shot tests pass, `pnpm -C ui shot /view/1` PNG opened | Sonnet | new tooling across Rust and TS |
| T2 | Issues and statuses | 10 tests in tests/issues.rs | Sonnet | transactions, DST, concurrent writers |
| T3 | Custom properties | 8 tests in tests/properties.rs | Sonnet | type conversion and view cleanup |
| T4 | Projects, cycles, labels, comments, blocks | 9 tests in tests/org.rs | Haiku | CRUD with stated validation rules |
| T5 | Relations and schedule | relations tests and schedule unit tests | Sonnet | graph algorithms |
| T6 | Views, filter compiler, search | 10 tests in tests/views.rs, no inlined user text in SQL | Sonnet | SQL generation, injection safety |
| T7 | Quick-add parser | 8 parser tests | Haiku | grammar fully specified |
| T8 | Google Calendar planner (pure) | 7 planner tests (digest test ignored until T24) | Haiku | pure rules fully specified |
| T9 | Backups | 5 tests in tests/backup.rs | Haiku | rules fully specified |
| T10 | Reminders Swift helper | 5 swift tests, permission prompt seen once by hand | Sonnet | EventKit, real-tool probe |
| T11 | Pins | 5 tests in tests/pins.rs | Haiku | rules fully specified |
| T12 | Shell, palette, quick-add UI | 6 component tests, shot exits 0 | Sonnet | keyboard-first interaction design |
| T13 | List view | 4 tests incl. virtualization | Haiku | spec lists behaviour per test |
| T14 | Table view | 4 tests | Haiku | spec lists behaviour per test |
| T15 | Board view | 3 tests | Haiku | spec lists behaviour per test |
| T16 | Issue detail | 7 tests | Sonnet | many panels, several APIs |
| T17 | Sidebar, saved views, view editor | 6 tests | Sonnet | nested filter builder |
| T18 | Settings and admin pages | 5 tests | Haiku | forms over existing APIs |
| T19 | Gantt view | 7 tests | Sonnet | third-party chart, drag logic |
| T20 | Calendar view | 5 tests | Haiku | library wiring, stated behaviour |
| T29 | Project page and Pinned page | 5 tests | Haiku | pages over existing components |
| T21 | MCP server | 6 tests, listed by Claude Code once by hand | Sonnet | agent-facing safety (snapshots) |
| T22 | CLI | 5 tests | Haiku | mirrors MCP one to one |
| T23 | Reminders import | 5 tests incl. crash idempotence | Sonnet | crash safety |
| T24 | Google Calendar auth, client, reconcile | 6 tests, digest encoding probed and recorded | Sonnet | OAuth, real-API probe |
| T25 | GitHub export | 5 tests, deterministic output | Haiku | rules fully specified |
| T26 | App sync loop, deep links, live refresh | 5 tests | Sonnet | concurrency |
| T27 | QA checklist and Linear runbook | checklist run and recorded, runbook written | Haiku | steps given; real-logic fixes handed up to Sonnet |

Split: 16 Sonnet, 14 Haiku.

## Open questions
- Digest reminder encoding (all-day at 08:00): the T24 probe decides between an API reminder and the extra timed event.
- Plan wave gate says `cargo test --workspace` on main; AGENTS says no full suite locally. Proposed: CI `check` on plan-N is the gate.
- Plan says task branches are cut from main; the prompt says from `plan-N`. Proposed: prompt wins.
- cargo-deny not adopted (decision 0004); say if you want licenses and advisories checked.
- tauri.conf.json, Info.plist and build.rs are T0's; `cargo tauri dev` needs tauri-cli installed.

## Probes needed (not run; owner's login and machine are involved)
- T10 reminders helper: real EventKit permission prompt and real list fetch, redacted fixtures saved.
- T24 digest reminder encoding: real test calendar, what Google stores for an all-day event with a morning notification.
- T24 OAuth: real Desktop client, loopback redirect.
Third-party tools driven: Apple Reminders (EventKit), Google Calendar API, Linear MCP (read-only, migration only).

## Rulings
1. 2026-11-21 `crates/commands` depends only on core; gcal/reminders/sync commands are wrapped in `app/src-tauri` and not served by the devserver. Why: boundary rule in the setup brief. Cost if wrong: one more crate edge in the table.
2. 2026-11-21 Dependencies pinned `=x.y.z` in `[workspace.dependencies]`; crates use `workspace = true`. Cost: bumps are explicit.
3. 2026-11-21 Pre-commit cannot run the message check first (git order); the commit-msg hook does it. Cost: a bad message is found after fmt/clippy.
4. 2026-11-21 cargo-machete `ignored` lists in each crate until T0. Cost: unused deps are not caught until T0 removes them; the T0 review must check they are gone.
5. 2026-11-21 Installs allowed by the owner; the setup agent installed none; shellcheck and machete run only in CI until installed.
6. 2026-11-21 Repo will be public; nothing secret in `.env` (only DATABASE_URL).
7. 2026-11-21 CI also runs on pushes to main (PR #2). Why: the plan-* ruleset requires `check` even when a branch is
   created, so plan-N is cut from a main commit that already has `check`. Disabling the ruleset was refused by the
   permission system and is not done. Cost if wrong: one extra CI run per merge to main.
8. 2026-11-21 Wave 1 started before T0 landed: each task branch is plan-1 plus a merge of `task/T0`; those PRs land
   after T0. Why: wall-clock. Cost if wrong: re-merge if T0 changes in CI fixes.
9. 2026-11-21 Every worktree uses one shared `CARGO_TARGET_DIR` (`../taskapp-wt/.target`). Why: 49 GB free, 7 GB per
   target dir. Cost: builds wait on the cargo lock.
10. 2026-11-21 A task may add `workspace = true` dependency lines to the manifest of the crate it works in (T0 removed
   unused deps for cargo-machete). Root Cargo.toml stays the orchestrator's. Cost: mechanical manifest conflicts at
   landing; keep both sides.
11. 2026-11-21 T0 deviations accepted: `history::Edit` struct (6-arg clippy limit), `Clearable<T>` alias for
   `Option<Option<T>>`, commands generated from one `with_commands!` list, schedule_* in `crates/commands`, bindings
   regenerated by `UPDATE_BINDINGS=1 cargo test -p taskapp-app --test bindings` instead of `cargo tauri dev`, Board
   view groups by Status, `.gitignore` gains `app/src-tauri/gen/` and `ui/dist/`.

## After this plan
- CI lint job takes ~6 min on macos-latest; shard or move clippy to ubuntu once Rust tasks land (target < 5 min).
- `actions/checkout@v4` warns about Node 20 deprecation; bump when v5 is current.

## Ledger
- 2026-11-21 07:40 setup agent (Sonnet) dispatched, worktree = main checkout, branch chore/setup. Reported 07:52.
- 2026-11-21 07:55 PR #1 opened. CI fixes by orchestrator: hashFiles in job-level if; shellcheck findings in self-tests.
- 2026-11-21 08:10 PR #1 landed. Branch protection + ruleset applied. 08:15 PR #2 (CI on main push), landed 08:30.
- 2026-11-21 08:20 T0 (Sonnet) dispatched, wt ../taskapp-wt/T0, branch task/T0. Reported 08:31 (blocked on missing plan-1).
- 2026-11-21 08:45 plan-1 created at 520cb59. T0 resumed to open its PR.
- 2026-11-21 08:50 dispatched, each wt ../taskapp-wt/T<n>, branch task/T<n>: T1 S, T2 S, T3 S, T4 H, T5 S, T6 S, T7 H,
  T8 H, T9 H, T10 S, T11 H.

## Next steps
1. Land T0, then wave 1 PRs as they go green (queue agent once ~5 are open).
2. Dispatch T28 when T1 lands; UI tasks T12-T20, T29 when T28 lands.
