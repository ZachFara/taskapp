---
links:
  - '[[Personal Coding Projects]]'
date: 2026-11-21
time: '02:35'
title: Personal Task App Implementation Plan
status: In Progress
references: null
tags:
  - Personal_Coding_Projects
---
> [!note] AI Generated
> Written by Claude (Opus 5.5) in chat on 2026-11-21, from a design session with me. Related: [[Personal Task App Design]], [[Personal Task App Design.pdf|design PDF]], [[Personal Task Manager Project Architect Agent]].

# Purpose

The step-by-step build plan for [[Personal Task App Design]], split into waves so parallel agents can build it without stepping on each other.

---

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A single-user, Linear-like Mac desktop task app with custom properties, Gantt, Apple Reminders capture, Google Calendar notifications, optional GitHub export, and an MCP server and CLI for agents.

**Architecture:** A Cargo workspace. `taskapp-core` owns the SQLite database and every domain operation. The Tauri app, the MCP server, and the CLI are thin adapters over core. Sync features (Reminders, Google Calendar, export) are separate crates that only the Tauri app runs, so the app is the single writer to external systems. A Svelte 5 UI talks to Tauri through generated TypeScript bindings.

**Tech Stack:** Rust stable, Tauri 2, SQLx 0.8 (SQLite, offline query cache committed), tokio, serde, chrono, tauri-specta (TS bindings), rmcp (MCP server, stdio), clap (CLI), oauth2 + keyring (Google auth), Swift 5.9 + EventKit (Reminders helper), Svelte 5 + Vite + TypeScript, Vitest + @testing-library/svelte, Frappe Gantt, @event-calendar/core, bits-ui (command palette).

**Spec:** `docs/superpowers/specs/2026-11-20-personal-task-app-design.md` (copy of the design doc at https://claude.ai/artifact/XeZkvTga3zbHeDJRUtGew5). Executors read both.

# TLDR

Build in four waves. Wave 0 lays down every contract and the development harness. T0 is one agent writing: the schema, all Rust types, every function signature as a stub, every Tauri command, the generated TypeScript bindings, all dependencies, then T1 builds the UI skeleton and T28 builds the harness, so from wave 1 on every agent can open the app in a browser, screenshot any screen, and read its console errors. After that, no two tasks in the same wave edit the same file, so wave 1 runs up to 20 agents at once (10 backend, 10 UI). Wave 2 (5 agents) builds the adapters and sync crates on top of finished core. Wave 3 (2 agents) wires the background sync loop and does end-to-end QA.

---

# Contents

1. Global Constraints
2. Review Focus
3. How to run this in parallel
4. File structure and ownership
5. Wave 0: contracts and harness (T0, T1, T28)
6. Wave 1: core and UI (T2 to T20, T29)
7. Wave 2: adapters and sync (T21 to T25)
8. Wave 3: integration (T26, T27)
9. Spec coverage map

---

# Global Constraints

- macOS 14 (Sonoma) or later, Apple Silicon. EventKit full Reminders access needs macOS 14.
- Single user, one Mac. Only the Tauri app talks to Reminders, Google Calendar, and the export repo. MCP and CLI never do.
- SQLite in WAL mode with `busy_timeout = 5000`. The database file lives at `~/Library/Application Support/taskapp/taskapp.db`.
- Issue keys are `<PREFIX>-<n>`; the prefix is the `issue_prefix` setting, default `ZF`.
- Priorities follow Linear: 0 none, 1 urgent, 2 high, 3 medium, 4 low.
- Estimates are stored in minutes. For Gantt bars, 1 day = 480 minutes (8 hours).
- Timed due events in Google Calendar last 15 minutes with a popup reminder at the event time.
- Date-only due issues go into one all-day digest event per local date, titled `Due today: N task(s)`, with a notification at 08:00 that day, used even for one task.
- Issues with no due date never get a calendar event.
- Calendar sync is one-way: the app overwrites any manual edits to its own events.
- Google OAuth app must be published (unverified production), never left in Testing mode.
- GitHub export is off by default.
- Every mutation writes change history with a `ChangeSource`.
- The Linear migration only reads from Linear.
- No em dashes in user-facing copy.

---

# Review Focus

1. **Time zones and DST.** A due date-time entered in local time must stay at the same local wall-clock time after a DST change. Date-only due dates must never shift a day. Test: `due_datetime_survives_dst` (T2), `digest_uses_local_date` (T8).
2. **Reminders import after a crash.** If the app dies after creating the issue but before deleting the reminder, the next import must not create a duplicate. Test: `reimport_after_crash_is_idempotent` (T23).
3. **Someone edits or deletes the app's events in Google Calendar.** The next reconcile must restore or recreate them, and events for archived or completed issues must disappear. An expired or revoked token must show a visible error state, not fail silently. Tests: `reconcile_recreates_deleted_event`, `revoked_token_sets_error_state` (T24).
4. **Changing a property that already has values.** Renaming keeps all values. Changing the type converts values that fit and refuses the change (listing the issues) when some don't. Deleting removes the values and drops the property from saved views without breaking them. Tests in T3.
5. **The MCP server or CLI writing while the app is open.** Writes must wait rather than fail on a lock, and the UI must refresh within 2 seconds. Tests: `concurrent_writers_do_not_error` (T2), `app_refreshes_on_external_change` (T26).

---

# How to run this in parallel

- Each task runs in its own git worktree on branch `task/T<n>`, created from `main` after the previous wave merged.
- **Wave gate:** a wave starts only when every task in the previous wave is merged and `cargo test --workspace && pnpm -C ui test` passes on `main`.
- **File ownership:** each task lists the files it owns. A task edits only its own files. T0 and T1 create every shared file (module declarations, Cargo manifests, `package.json`, migrations, bindings, route table) so later tasks never touch them. If a task finds it needs to change a shared file or a signature, it stops and reports instead of editing.
- **Stubs:** T0 writes every core function with its final signature and body `todo!("T<n>")`, naming the task that fills it. A task is done when its `todo!`s are gone.
- **SQLx offline cache:** run `cargo sqlx prepare --workspace` before each commit. The cache is one file per query, so parallel branches merge cleanly.
- **UI tasks** test against `ui/src/lib/api.mock.ts` (from T1), so they don't wait on the backend, and check their screens with the harness (T28).

Dependency graph:

```
T0 ──┬─> T2..T11 (backend core, parallel) ─────────┬─> T21..T25 (parallel) ──> T26 ──> T27
     └─> T1 ──> T28 (harness) ──> T12..T20, T29 (UI) ┘
```

---

# File structure and ownership

```
taskapp/
  Cargo.toml                      T0  workspace, all dependencies pinned
  .env, scripts/dev-db.sh         T0  DATABASE_URL for SQLx, builds dev.db
  crates/core/                    package taskapp-core
    migrations/0001_init.sql      T0
    src/lib.rs, error.rs, db.rs   T0  module declarations, Db, Error
    src/model.rs                  T0  every shared type
    src/history.rs                T0  implemented in T0 (everyone needs it)
    src/settings.rs               T0  implemented in T0
    src/pins.rs                   T11
    src/issues.rs, statuses.rs    T2
    src/properties.rs             T3
    src/projects.rs, cycles.rs, labels.rs, comments.rs, blocks.rs   T4
    src/relations.rs, schedule.rs T5
    src/views.rs, search.rs       T6
    src/quickadd.rs               T7
    src/backup.rs                 T9
  crates/gcal/                    package taskapp-gcal
    src/planner.rs                T8
    src/auth.rs, client.rs, reconcile.rs   T24
  crates/reminders/               package taskapp-reminders
    src/helper.rs, import.rs      T23
  helpers/reminders-helper/       Swift package             T10
  crates/export/                  package taskapp-export
    src/write.rs, rebuild.rs, git.rs       T25
  crates/mcp/                     bin taskapp-mcp           T21
  crates/cli/                     bin taskapp               T22
  crates/commands/                T0  every command as a plain async fn, plus dispatch(name, args)
  crates/devserver/               bin taskapp-devserver     T28
  ui/harness/shot.ts, scripts/seed-dev.sh, docs/harness.md   T28
  app/src-tauri/
    src/main.rs, commands.rs      T0  Tauri wrappers over crates/commands
    src/sync_loop.rs, deeplink.rs T26
    tauri.conf.json, Info.plist   T0
  ui/
    package.json, vite config     T1
    src/lib/bindings.ts           T0  generated by tauri-specta, never hand-edited
    src/lib/api.ts, api.mock.ts   T1
    src/lib/stores.ts, keymap.ts  T1
    src/routes.ts, App.svelte     T1  every route, placeholder components
    src/components/Shell.svelte, CommandPalette.svelte, QuickAdd.svelte    T12
    src/components/views/ListView.svelte       T13
    src/components/views/TableView.svelte      T14
    src/components/views/BoardView.svelte      T15
    src/components/IssueDetail.svelte          T16
    src/components/ViewEditor.svelte, Sidebar.svelte   T17
    src/components/settings/*.svelte           T18
    src/components/views/GanttView.svelte      T19
    src/components/views/CalendarView.svelte   T20
    src/components/ProjectPage.svelte, PinnedPage.svelte   T29
```

---

# Wave 0: contracts

## Task T0: Workspace, schema, types, stubs, and Tauri commands

One agent. Everything later depends on this, so finish it completely before wave 1.

**Files:** all files marked T0 above.

**Produces:** everything in this section. Later tasks rely on these exact names.

`crates/core/migrations/0001_init.sql` (tables; add indexes on every foreign key and on `issues.due_date`, `issues.status_id`):

```sql
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);        -- JSON values
CREATE TABLE statuses (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE,
  category TEXT NOT NULL CHECK (category IN ('backlog','unstarted','started','completed','canceled')),
  position INTEGER NOT NULL, color TEXT NOT NULL);
CREATE TABLE projects (id INTEGER PRIMARY KEY, slug TEXT NOT NULL UNIQUE, name TEXT NOT NULL,
  description TEXT, doc_link TEXT,  -- both optional; doc_link is usually an obsidian:// link
  start_date TEXT, target_date TEXT, archived_at TEXT, created_at TEXT NOT NULL);
CREATE TABLE milestones (id INTEGER PRIMARY KEY, project_id INTEGER NOT NULL REFERENCES projects ON DELETE CASCADE,
  name TEXT NOT NULL, target_date TEXT);
CREATE TABLE cycles (id INTEGER PRIMARY KEY, number INTEGER NOT NULL UNIQUE, name TEXT,
  start_date TEXT NOT NULL, end_date TEXT NOT NULL);
CREATE TABLE labels (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, color TEXT NOT NULL);
CREATE TABLE issues (id INTEGER PRIMARY KEY, number INTEGER NOT NULL UNIQUE, title TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '', status_id INTEGER NOT NULL REFERENCES statuses,
  priority INTEGER NOT NULL DEFAULT 0 CHECK (priority BETWEEN 0 AND 4),
  project_id INTEGER REFERENCES projects, milestone_id INTEGER REFERENCES milestones,
  parent_id INTEGER REFERENCES issues, cycle_id INTEGER REFERENCES cycles,
  start_date TEXT, due_date TEXT, due_time_utc TEXT, due_tz TEXT, estimate_minutes INTEGER,
  properties TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL, completed_at TEXT, archived_at TEXT);
CREATE TABLE issue_labels (issue_id INTEGER REFERENCES issues ON DELETE CASCADE,
  label_id INTEGER REFERENCES labels ON DELETE CASCADE, PRIMARY KEY (issue_id, label_id));
CREATE TABLE relations (id INTEGER PRIMARY KEY, from_id INTEGER NOT NULL REFERENCES issues ON DELETE CASCADE,
  to_id INTEGER NOT NULL REFERENCES issues ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK (kind IN ('blocks','related','duplicate')), UNIQUE (from_id, to_id, kind));
CREATE TABLE comments (id INTEGER PRIMARY KEY, issue_id INTEGER NOT NULL REFERENCES issues ON DELETE CASCADE,
  body TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
CREATE TABLE property_defs (key TEXT PRIMARY KEY, name TEXT NOT NULL, kind TEXT NOT NULL,
  options TEXT NOT NULL DEFAULT '[]', project_id INTEGER REFERENCES projects, position INTEGER NOT NULL);
CREATE TABLE views (id INTEGER PRIMARY KEY, name TEXT NOT NULL, definition TEXT NOT NULL, position INTEGER NOT NULL);
CREATE TABLE pins (id INTEGER PRIMARY KEY,
  kind TEXT NOT NULL CHECK (kind IN ('issue','project','view','cycle','note','url')),
  target TEXT NOT NULL, title TEXT, position INTEGER NOT NULL, UNIQUE (kind, target));
CREATE TABLE work_blocks (id INTEGER PRIMARY KEY, issue_id INTEGER NOT NULL REFERENCES issues ON DELETE CASCADE,
  start_utc TEXT NOT NULL, end_utc TEXT NOT NULL);
CREATE TABLE changes (id INTEGER PRIMARY KEY, at TEXT NOT NULL, source TEXT NOT NULL,
  entity TEXT NOT NULL, entity_id TEXT NOT NULL, field TEXT NOT NULL, old_value TEXT, new_value TEXT);
CREATE TABLE import_log (external_id TEXT PRIMARY KEY, source TEXT NOT NULL, issue_id INTEGER NOT NULL,
  imported_at TEXT NOT NULL, source_deleted INTEGER NOT NULL DEFAULT 0);
CREATE TABLE gcal_events (google_event_id TEXT PRIMARY KEY, calendar_id TEXT NOT NULL,
  managed_key TEXT NOT NULL UNIQUE, etag TEXT, last_synced_hash TEXT NOT NULL);
CREATE VIRTUAL TABLE issue_fts USING fts5(title, description, comments, content='', tokenize='porter');
-- seed statuses: Backlog(backlog), Todo(unstarted), In Progress(started), Done(completed), Canceled(canceled)
-- seed settings: issue_prefix "ZF", gcal_enabled false, export_enabled false
-- seed views: "Active" (list, status category unstarted or started), "Board" (board, same filter),
--   "Timeline: all projects" (gantt, group_by project, not completed)
```

`crates/core/src/model.rs` (all `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]`):

```rust
pub struct IssueId(pub i64); pub struct StatusId(pub i64); pub struct ProjectId(pub i64);
pub struct MilestoneId(pub i64); pub struct CycleId(pub i64); pub struct LabelId(pub i64);
pub struct ViewId(pub i64); pub struct CommentId(pub i64); pub struct WorkBlockId(pub i64);

pub enum Due { Date(NaiveDate), DateTime { utc: DateTime<Utc>, tz: String } } // tz = IANA name
pub enum StatusCategory { Backlog, Unstarted, Started, Completed, Canceled }
pub enum ChangeSource { Ui, Agent, Cli, RemindersImport, Migration, System }
pub enum PropertyKind { Text, Number, Date, Select, MultiSelect, Checkbox, Url, NoteLink }
pub enum RelationKind { Blocks, Related, Duplicate }

pub struct Issue { pub id: IssueId, pub key: String, pub title: String, pub description: String,
  pub status_id: StatusId, pub priority: u8, pub project_id: Option<ProjectId>,
  pub milestone_id: Option<MilestoneId>, pub parent_id: Option<IssueId>, pub cycle_id: Option<CycleId>,
  pub label_ids: Vec<LabelId>, pub start_date: Option<NaiveDate>, pub due: Option<Due>,
  pub estimate_minutes: Option<u32>, pub properties: serde_json::Map<String, Value>,
  pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc>,
  pub completed_at: Option<DateTime<Utc>>, pub archived_at: Option<DateTime<Utc>> }

pub struct NewIssue { /* every Issue field except id, key, timestamps, all Option except title;
  plus created_at: Option<DateTime<Utc>>, completed_at: Option<DateTime<Utc>> for migrations */ }
pub struct IssuePatch { /* every editable field as Option<T>; nullable fields as Option<Option<T>> */ }

pub struct Status { pub id: StatusId, pub name: String, pub category: StatusCategory, pub position: i32, pub color: String }
pub struct Project { pub id: ProjectId, pub slug: String, pub name: String,
  pub description: Option<String>, pub doc_link: Option<String>,
  pub start_date: Option<NaiveDate>, pub target_date: Option<NaiveDate>, pub archived_at: Option<DateTime<Utc>> }
pub struct Milestone { pub id: MilestoneId, pub project_id: ProjectId, pub name: String, pub target_date: Option<NaiveDate> }
pub struct Cycle { pub id: CycleId, pub number: u32, pub name: Option<String>, pub start_date: NaiveDate, pub end_date: NaiveDate }
pub struct Label { pub id: LabelId, pub name: String, pub color: String }
pub struct Comment { pub id: CommentId, pub issue_id: IssueId, pub body: String, pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc> }
pub struct Relation { pub from: IssueId, pub to: IssueId, pub kind: RelationKind }
pub struct SelectOption { pub value: String, pub color: String }
pub struct PropertyDef { pub key: String, pub name: String, pub kind: PropertyKind,
  pub options: Vec<SelectOption>, pub project_id: Option<ProjectId>, pub position: i32 }
pub struct PinId(pub i64);
pub enum PinKind { Issue, Project, View, Cycle, Note, Url }
pub struct Pin { pub id: PinId, pub kind: PinKind, pub target: String, pub title: Option<String>, pub position: i32 }
pub struct ResolvedPin { pub pin: Pin, pub label: String, pub archived: bool, pub route: String } // route = UI route or the external URL
pub struct WorkBlock { pub id: WorkBlockId, pub issue_id: IssueId, pub start_utc: DateTime<Utc>, pub end_utc: DateTime<Utc> }

pub enum FixedField { Title, Status, StatusCategory, Priority, Project, Milestone, Cycle, Label, Parent,
  StartDate, DueDate, Estimate, CreatedAt, UpdatedAt, CompletedAt, Archived }
pub enum FieldRef { Fixed(FixedField), Property(String) }
pub enum Op { Eq, Neq, In, NotIn, Lt, Lte, Gt, Gte, Contains, IsEmpty, IsNotEmpty }
pub enum Filter { And(Vec<Filter>), Or(Vec<Filter>), Not(Box<Filter>), Cond { field: FieldRef, op: Op, value: Value } }
pub struct Sort { pub field: FieldRef, pub descending: bool }
pub enum Layout { List, Table, Board, Gantt, Calendar }
pub struct ViewDef { pub filter: Option<Filter>, pub sorts: Vec<Sort>, pub group_by: Option<FieldRef>,
  pub layout: Layout, pub columns: Vec<FieldRef>, pub include_archived: bool }
pub struct SavedView { pub id: ViewId, pub name: String, pub def: ViewDef, pub position: i32 }

pub struct Bar { pub issue_id: IssueId, pub start: NaiveDate, pub end: NaiveDate, pub from_estimate: bool }
pub struct Reschedule { pub issue_id: IssueId, pub start_date: NaiveDate, pub due_date: NaiveDate }
pub struct ParsedQuickAdd { pub title: String, pub due: Option<Due>, pub start_date: Option<NaiveDate>,
  pub priority: Option<u8>, pub project_slug: Option<String>, pub label_names: Vec<String>, pub estimate_minutes: Option<u32> }
pub struct SyncStatus { pub reminders: SyncState, pub gcal: SyncState, pub export: SyncState }
pub enum SyncState { Off, Ok { at: DateTime<Utc> }, Error { at: DateTime<Utc>, message: String } }
```

`error.rs`: `pub enum Error { NotFound(String), Invalid(String), Conflict(String), Db(sqlx::Error), Io(std::io::Error) }`, `pub type Result<T> = std::result::Result<T, Error>`, serializable for Tauri as `{ kind, message }`.

`db.rs`: `pub struct Db { pub pool: SqlitePool }`, `impl Db { pub async fn open(path: &Path) -> Result<Db>; pub async fn open_in_memory() -> Result<Db>; pub async fn data_version(&self) -> Result<i64> }`. `open` sets WAL, `busy_timeout=5000`, `foreign_keys=ON`, runs migrations.

`history.rs` (implemented here): `pub async fn record(tx: &mut SqliteConnection, source: ChangeSource, entity: &str, entity_id: &str, field: &str, old: Option<&Value>, new: Option<&Value>) -> Result<()>` and `pub async fn list_for(db: &Db, entity: &str, entity_id: &str) -> Result<Vec<Change>>` (add `pub struct Change` with the table's columns to model.rs).

`settings.rs` (implemented here): `pub async fn get<T: DeserializeOwned>(db: &Db, key: &str) -> Result<Option<T>>`, `pub async fn set<T: Serialize>(db: &Db, key: &str, value: &T) -> Result<()>`. Keys: `issue_prefix`, `reminders_list_id`, `gcal_enabled`, `gcal_calendar_id`, `export_enabled`, `export_dir`, `export_remote`, `backup_dir`, `sync_status`.

Core stubs, each `todo!("T<n>")` (signatures final):

```rust
// issues.rs (T2)
pub async fn create(db: &Db, new: NewIssue, src: ChangeSource) -> Result<Issue>;
pub async fn update(db: &Db, id: IssueId, patch: IssuePatch, src: ChangeSource) -> Result<Issue>;
pub async fn get(db: &Db, id: IssueId) -> Result<Issue>;
pub async fn get_by_key(db: &Db, key: &str) -> Result<Issue>;
pub async fn archive(db: &Db, id: IssueId, src: ChangeSource) -> Result<()>;
pub async fn restore(db: &Db, id: IssueId, src: ChangeSource) -> Result<()>;
pub async fn children(db: &Db, id: IssueId) -> Result<Vec<Issue>>;
// statuses.rs (T2)
pub async fn list(db: &Db) -> Result<Vec<Status>>;
pub async fn create(db: &Db, name: &str, category: StatusCategory, color: &str) -> Result<Status>;
pub async fn update(db: &Db, id: StatusId, name: Option<String>, color: Option<String>, position: Option<i32>) -> Result<Status>;
pub async fn delete(db: &Db, id: StatusId, move_issues_to: StatusId) -> Result<()>;
// properties.rs (T3)
pub async fn list_defs(db: &Db) -> Result<Vec<PropertyDef>>;
pub async fn create_def(db: &Db, def: PropertyDef, src: ChangeSource) -> Result<PropertyDef>;
pub async fn rename_def(db: &Db, key: &str, new_name: &str, src: ChangeSource) -> Result<PropertyDef>;
pub async fn change_kind(db: &Db, key: &str, kind: PropertyKind, options: Vec<SelectOption>, src: ChangeSource) -> Result<PropertyDef>;
pub async fn delete_def(db: &Db, key: &str, src: ChangeSource) -> Result<()>;
pub async fn set_value(db: &Db, issue: IssueId, key: &str, value: Option<Value>, src: ChangeSource) -> Result<Issue>;
pub fn validate(def: &PropertyDef, value: &Value) -> Result<()>;
// projects.rs, cycles.rs, labels.rs, comments.rs (T4): list/create/update/delete per type, plus
pub async fn add_label(db: &Db, issue: IssueId, label: LabelId, src: ChangeSource) -> Result<()>;
pub async fn remove_label(db: &Db, issue: IssueId, label: LabelId, src: ChangeSource) -> Result<()>;
pub async fn add_comment(db: &Db, issue: IssueId, body: &str, created_at: Option<DateTime<Utc>>, src: ChangeSource) -> Result<Comment>;
// relations.rs (T5)
pub async fn add(db: &Db, rel: Relation, src: ChangeSource) -> Result<()>;
pub async fn remove(db: &Db, rel: Relation, src: ChangeSource) -> Result<()>;
pub async fn for_issue(db: &Db, id: IssueId) -> Result<Vec<Relation>>;
pub async fn all_blocking(db: &Db) -> Result<Vec<Relation>>;
// schedule.rs (T5), pure
pub fn bar_for(issue: &Issue) -> Option<Bar>;
pub fn shift_dependents(issues: &[Issue], blocks: &[Relation], moved: &Reschedule) -> Vec<Reschedule>;
pub fn critical_path(issues: &[Issue], blocks: &[Relation]) -> Vec<IssueId>;
// views.rs (T6)
pub fn compile(def: &ViewDef, defs: &[PropertyDef]) -> Result<(String, Vec<Value>)>;
pub async fn query(db: &Db, def: &ViewDef) -> Result<Vec<Issue>>;
pub async fn list(db: &Db) -> Result<Vec<SavedView>>;
pub async fn save(db: &Db, id: Option<ViewId>, name: &str, def: ViewDef) -> Result<SavedView>;
pub async fn delete(db: &Db, id: ViewId) -> Result<()>;
// search.rs (T6)
pub async fn search(db: &Db, text: &str, limit: u32) -> Result<Vec<Issue>>;
pub async fn reindex(db: &Db, id: IssueId) -> Result<()>;
// quickadd.rs (T7), pure
pub fn parse(input: &str, today: NaiveDate, tz: &str) -> ParsedQuickAdd;
pub async fn create_from(db: &Db, input: &str, today: NaiveDate, tz: &str, src: ChangeSource) -> Result<Issue>;
// pins.rs (T11)
pub async fn pin(db: &Db, kind: PinKind, target: &str, title: Option<String>) -> Result<Pin>; // idempotent
pub async fn unpin(db: &Db, id: PinId) -> Result<()>;
pub async fn reorder(db: &Db, ids_in_order: &[PinId]) -> Result<()>;
pub async fn list_resolved(db: &Db) -> Result<Vec<ResolvedPin>>;
// backup.rs (T9)
pub async fn snapshot(db: &Db, dir: &Path, reason: &str) -> Result<PathBuf>;
pub async fn nightly(db: &Db, dir: &Path, now: DateTime<Utc>) -> Result<Option<PathBuf>>;
pub fn prune(dir: &Path) -> Result<Vec<PathBuf>>;
pub async fn restore(from: &Path, to: &Path) -> Result<()>;
// projects.rs (T4) progress, for project pages
pub async fn progress(db: &Db, id: ProjectId) -> Result<(u32, u32)>; // (completed, total)
// blocks.rs (T4) work blocks
pub async fn create_block(db: &Db, issue: IssueId, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<WorkBlock>;
pub async fn move_block(db: &Db, id: WorkBlockId, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<WorkBlock>;
pub async fn delete_block(db: &Db, id: WorkBlockId) -> Result<()>;
pub async fn blocks_between(db: &Db, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Vec<WorkBlock>>;
```

Sync crate stubs:

```rust
// taskapp-gcal planner.rs (T8)
pub struct DesiredEvent { pub managed_key: String, pub summary: String, pub description: String,
  pub timing: EventTiming, pub reminder_minutes: Option<u32> }
pub enum EventTiming { AllDay(NaiveDate), Timed { start: DateTime<Utc>, end: DateTime<Utc> } }
pub struct ManagedEvent { pub google_event_id: String, pub managed_key: String, pub last_synced_hash: String }
pub enum EventOp { Create(DesiredEvent), Update { google_event_id: String, event: DesiredEvent }, Delete { google_event_id: String } }
pub fn desired(issues: &[Issue], statuses: &[Status], blocks: &[WorkBlock], tz: &str, issue_prefix: &str) -> Vec<DesiredEvent>;
pub fn plan(desired: &[DesiredEvent], existing: &[ManagedEvent]) -> Vec<EventOp>;
pub fn hash(event: &DesiredEvent) -> String;
// auth.rs, client.rs, reconcile.rs (T24)
pub async fn connect(db: &Db) -> Result<()>;                       // browser OAuth, PKCE, loopback
pub async fn list_calendars(db: &Db) -> Result<Vec<(String, String)>>; // (id, name)
pub async fn reconcile(db: &Db) -> Result<ReconcileReport>;
pub struct ReconcileReport { pub created: u32, pub updated: u32, pub deleted: u32 }
// taskapp-reminders (T23)
pub async fn list_reminder_lists(helper: &Path) -> Result<Vec<(String, String)>>;
pub async fn import(db: &Db, helper: &Path, today: NaiveDate, tz: &str) -> Result<ImportReport>;
pub struct ImportReport { pub imported: u32, pub skipped_duplicates: u32, pub failed: Vec<String> }
// taskapp-export (T25)
pub async fn write_snapshot(db: &Db, dir: &Path) -> Result<ExportStats>;
pub async fn rebuild(dir: &Path, db_path: &Path) -> Result<Db>;
pub async fn commit_and_push(dir: &Path, message: &str, remote: Option<&str>) -> Result<bool>;
pub struct ExportStats { pub issues: u32, pub files_written: u32, pub files_removed: u32 }
```

`crates/commands/src/lib.rs`: one plain `pub async fn <module>_<fn>(state: &AppState, ...)` per public function above that the UI needs (for example `issues_create`, `views_query`, `pins_list_resolved`, `gcal_list_calendars`, `sync_status`), each forwarding with `ChangeSource::Ui`, plus `pub async fn dispatch(state: &AppState, name: &str, args: serde_json::Value) -> Result<serde_json::Value>` routing every command by name (unknown name: `Error::NotFound`). `AppState { db: Db, helper_path: PathBuf, tz: String }`. `app/src-tauri/src/commands.rs`: one `#[tauri::command] #[specta::specta]` wrapper per command in `crates/commands`. Commands that are not a plain one-to-one forward, which UI tasks rely on by name: `issues_parse_quickadd(input) -> ParsedQuickAdd` (uses today and the system tz), `schedule_bars(view_id) -> Vec<Bar>`, `schedule_critical_path(view_id) -> Vec<IssueId>`, `schedule_shift_dependents(moved: Reschedule) -> Vec<Reschedule>` (these three load issues and blocking relations from the db, then call the pure functions), `reminders_list_lists() -> Vec<(String, String)>` (uses the bundled helper path), `sync_status() -> SyncStatus` (reads the `sync_status` setting), and `sync_now() -> Result<SyncStatus>` (body `todo!("T26")`). `main.rs` registers them all and exports bindings to `ui/src/lib/bindings.ts` in debug builds. Add the single-instance plugin and the `taskapp://` deep-link scheme in `tauri.conf.json`. `Info.plist` gets `NSRemindersFullAccessUsageDescription` = "Imports tasks you add to your inbox list in Reminders."

- [ ] **Step 1: Create the workspace, manifests with all dependencies, `.env`, `scripts/dev-db.sh`, and the migration.**
- [ ] **Step 2: Write model, error, db, history, settings, and every stub with the exact signatures above.**
- [ ] **Step 3: Write tests for the T0-implemented pieces** in `crates/core/tests/foundation.rs`:

```rust
#[tokio::test] async fn migrations_seed_statuses_and_prefix() {
  let db = Db::open_in_memory().await.unwrap();
  assert_eq!(statuses_raw_count(&db).await, 5);
  assert_eq!(settings::get::<String>(&db, "issue_prefix").await.unwrap(), Some("ZF".into()));
}
#[tokio::test] async fn settings_roundtrip() { /* set("gcal_calendar_id", "abc") then get == Some("abc") */ }
#[tokio::test] async fn history_record_and_list() { /* record one change, list_for returns it with source Agent */ }
#[tokio::test] async fn dispatch_unknown_command_is_not_found() { /* dispatch(state, "nope", json!({})) is Err(NotFound) */ }
#[tokio::test] async fn open_sets_wal_and_busy_timeout() { /* PRAGMA journal_mode == "wal", busy_timeout == 5000 on a temp file db */ }
```

- [ ] **Step 4: Run** `cargo test -p taskapp-core --test foundation`. Expected: 5 passed. Run `cargo build --workspace`. Expected: builds (stubs compile).
- [ ] **Step 5: Run** `cargo tauri dev` once and check `ui/src/lib/bindings.ts` is generated with every command. Run `cargo sqlx prepare --workspace`.
- [ ] **Step 6: Commit** `git commit -m "feat: workspace, schema, contracts, and stubs"`

## Task T1: UI skeleton, API layer, mock, and routes

**Depends on:** T0. **Files:** `ui/` files marked T1.

**Produces:**
- `ui/src/lib/api.ts`: `export const api = { issues: { create, update, get, ... }, views: {...}, ... }` grouping every command by module, with three transports picked by `import.meta.env.VITE_API_MODE`: `tauri` (default, the generated `bindings.ts`), `http` (POST `http://127.0.0.1:7878/rpc/<command>` with the JSON args, for the harness), and `mock`. Also `export function setApi(impl)` so tests can swap in the mock.
- `ui/src/lib/api.mock.ts`: `export function createMockApi(seed?: MockSeed): Api` backed by in-memory arrays, implementing every command with realistic behavior (create assigns `ZF-<n>` keys, `views.query` supports `Eq`/`In`/`Contains` on fixed fields and properties). `export const fixtures: MockSeed` with 5 statuses, 2 projects, 3 property defs (one select, one number, one date), and 20 issues.
- `ui/src/lib/stores.ts`: `issues`, `statuses`, `projects`, `propertyDefs`, `views`, `currentViewId`, `selectedIssueId` as Svelte 5 runes-based stores, plus `refreshAll(): Promise<void>`.
- `ui/src/lib/keymap.ts`: `registerShortcut(combo: string, id: string, handler: () => void): () => void` and the default map: `cmd+k` palette, `c` create, `/` search, `j`/`k` move selection, `enter` open, `esc` close, `s` status, `p` priority, `l` labels, `g i` inbox view, `g v` views.
- `ui/package.json` scripts: `dev:mock` (`VITE_API_MODE=mock vite`), `dev:real` (`VITE_API_MODE=http vite`), `shot` (`tsx harness/shot.ts`), and Playwright as a dev dependency.
- `ui/src/routes.ts`: routes `/view/:id`, `/issue/:key`, `/project/:slug`, `/projects/timeline`, `/pinned`, `/cycles`, `/settings/:section`, each mapped to the component file listed in the ownership table, created here as placeholders rendering their name.

- [ ] **Step 1: Write failing tests** `ui/src/lib/api.mock.test.ts`: `create assigns sequential keys` (`ZF-21`, then `ZF-22` after fixtures), `query filters by select property` (filter `Property("area") Eq "home"` returns only those), `setApi swaps implementation`, `http transport posts to rpc endpoint` (fetch mocked). `ui/src/lib/keymap.test.ts`: `sequence g then i fires once`, `shortcuts ignored while typing in an input`.
- [ ] **Step 2: Run** `pnpm -C ui test`. Expected: FAIL.
- [ ] **Step 3: Implement** the files above.
- [ ] **Step 4: Run** `pnpm -C ui test`. Expected: PASS. Run `pnpm -C ui build`. Expected: builds.
- [ ] **Step 5: Commit** `git commit -m "feat(ui): skeleton, api layer, mock, routes"`

## Task T28: Development harness

**Depends on:** T1. Backend wave 1 tasks may start before T28 finishes; UI wave 1 tasks wait for it.
**Files:** `crates/devserver/`, `ui/harness/shot.ts`, `ui/harness/shot.test.ts`, `scripts/seed-dev.sh`, `docs/harness.md`.

**Produces:**
- `taskapp-devserver serve --db <path> [--port 7878]`: an axum server on `127.0.0.1` only, `POST /rpc/<command>` calling `commands::dispatch` and returning `{ok: value}` or `{error: {kind, message}}`, with CORS allowed for `http://localhost:5173`. It refuses to start if `--db` resolves to the real app database path (Global Constraints). It never runs any sync.
- `taskapp-devserver seed --db <path>`: fills a database with the same fixtures as `api.mock.ts` through core functions (works once wave 1 core is merged; until then use mock mode).
- `pnpm -C ui shot <route> [--mode mock|real] [--width 1440] [--height 900] [--keys "c,renew lease,Enter"] [--click "<css selector>"]`: starts or reuses the matching dev server, opens the route in Playwright Chromium, runs the steps, waits for network idle, and writes `harness-out/<route-slug>.png`, `harness-out/<route-slug>.aria.txt` (Playwright `ariaSnapshot` of the page), and `harness-out/<route-slug>.console.json`. Exits 1 if any console error or failed request happened.
- `docs/harness.md`: one page telling agents how to run each mode and what to check after a UI change.

- [ ] **Step 1: Tests:** Rust `crates/devserver/tests/server.rs`: `rpc_round_trip` (create then get an issue on a temp db), `unknown_command_returns_error_json`, `refuses_real_app_db`, `binds_localhost_only`. TS `ui/harness/shot.test.ts`: `mock_mode_writes_png_aria_and_console` (route `/view/1`, PNG non-empty, aria text contains the fixture issue "ZF-1"), `console_error_exits_nonzero` (a test-only route that logs an error).
- [ ] **Step 2: Run** `cargo test -p taskapp-devserver` and `pnpm -C ui test -- harness`. Expected: FAIL.
- [ ] **Step 3: Implement** the devserver, seed command, shot script, and doc.
- [ ] **Step 4: Run** both again. Expected: PASS. Run `pnpm -C ui shot /view/1` by hand and open the PNG.
- [ ] **Step 5: Commit** `git commit -m "feat: development harness"`

---

# Wave 1: core and UI (all parallel)

Backend tasks T2 to T11 start after T0. UI tasks T12 to T20 and T29 start after T28. Each backend task's tests use `Db::open_in_memory()` and create the records they need with raw SQL inserts, so no task waits on another wave 1 task.

Every task follows the same cycle: write the listed tests, run them and see them fail, implement the listed signatures, run and see them pass, run `cargo sqlx prepare --workspace` (backend), commit.

## Task T2: Issues and statuses

**Files:** `crates/core/src/issues.rs`, `crates/core/src/statuses.rs`, `crates/core/tests/issues.rs`.
**Consumes:** `history::record`, `settings::get("issue_prefix")`. **Produces:** the T2 stubs from T0.

Rules: `number` is `max(number)+1` inside the insert transaction. `key` = prefix + `-` + number. Moving to a `completed` or `canceled` status sets `completed_at` to now (or the given migration timestamp); moving out clears it. Each changed field writes one `changes` row. `create` accepts `created_at`/`completed_at` overrides. `Due::DateTime` stores `due_time_utc` + `due_tz`; reading reconstructs it. `children` returns non-archived sub-issues.

- [ ] **Step 1: Tests** in `crates/core/tests/issues.rs`:

```rust
create_assigns_prefixed_sequential_keys      // "ZF-1", "ZF-2"
create_respects_migration_timestamps         // created_at == given value
update_writes_one_change_per_field           // title + priority patch -> 2 rows, source Agent
completing_sets_completed_at_and_reopening_clears_it
due_datetime_survives_dst                    // due 2026-03-09 09:00 America/New_York set on 2026-03-01 reads back 09:00 local
date_only_due_never_shifts                   // Due::Date(2026-11-30) reads back identical
archive_hides_from_children_and_restore_returns_it
get_by_key_unknown_is_not_found
delete_status_moves_issues                   // issues move to the target status
concurrent_writers_do_not_error              // two Db handles on one temp file, 50 creates each via tokio::join!, 100 issues, unique numbers
```

- [ ] **Step 2: Run** `cargo test -p taskapp-core --test issues`. Expected: FAIL (`todo!`).
- [ ] **Step 3: Implement** all T2 stubs.
- [ ] **Step 4: Run** the same. Expected: 10 passed.
- [ ] **Step 5: Commit** `git commit -m "feat(core): issues and statuses"`

## Task T3: Custom properties

**Files:** `crates/core/src/properties.rs`, `crates/core/tests/properties.rs`.
**Produces:** the T3 stubs.

Rules: keys match `^[a-z][a-z0-9_]{0,39}$`. `validate`: Text string; Number finite number; Date `YYYY-MM-DD`; Select one of `options[].value`; MultiSelect array of option values, no duplicates; Checkbox bool; Url `http`/`https`; NoteLink string ending `.md` or an `obsidian://` URL. Project-scoped defs reject values on issues of other projects. `rename_def` changes only `name` (key is permanent). `change_kind` converts every existing value (Text to Number parses, Select to MultiSelect wraps, MultiSelect to Select only if all have 0 or 1 values, anything to Text stringifies) and if any value fails, returns `Error::Conflict` listing up to 20 issue keys and changes nothing. `delete_def` removes the key from every issue's `properties` and from every saved view's filter, sorts, group_by, and columns (simplifying empty `And`/`Or`).

- [ ] **Step 1: Tests:**

```rust
invalid_key_rejected                          // "Area" and "1x" -> Invalid
validate_each_kind                            // table test, one valid and one invalid value per kind
set_value_records_history
rename_keeps_values
change_kind_text_to_number_converts           // "3" -> 3
change_kind_refuses_and_lists_keys            // one "abc" value -> Conflict containing "ZF-1", def unchanged
delete_def_removes_values_and_cleans_views    // saved view filtering on it still loads and returns all issues
project_scoped_def_rejects_other_project
```

- [ ] **Step 2 to 5:** run `cargo test -p taskapp-core --test properties` (fail, then 8 passed), commit `feat(core): custom properties`.

## Task T4: Projects, milestones, cycles, labels, comments, work blocks

**Files:** `crates/core/src/{projects,cycles,labels,comments,blocks}.rs`, `crates/core/tests/org.rs`.
**Produces:** the T4 stubs. Project `description` and `doc_link` are both optional; `doc_link` must be an `obsidian://` URL, an `https` URL, or a vault-relative path ending in `.md`. Project slugs match `^[a-z0-9][a-z0-9-]{0,39}$`. Cycles must not overlap (`Error::Conflict`). Comments call `search::reindex` only if T6 is merged; until then leave the call in place (it is a stub that T6 fills, and tests here don't assert search). Work block `end` must be after `start`.

- [ ] **Step 1: Tests:** `project_without_description_or_link`, `doc_link_validation`, `project_slug_validation`, `project_progress_counts_completed` (`projects::progress` returns (completed, total)), `overlapping_cycles_rejected`, `label_add_remove_records_history`, `comment_accepts_migration_timestamp`, `work_block_end_before_start_rejected`, `blocks_between_returns_overlapping_only`.
- [ ] **Step 2 to 5:** `cargo test -p taskapp-core --test org`, commit `feat(core): projects, cycles, labels, comments, work blocks`.

## Task T5: Relations and schedule logic

**Files:** `crates/core/src/relations.rs`, `crates/core/src/schedule.rs`, `crates/core/tests/relations.rs`, unit tests inside `schedule.rs`.

Rules: `add` with `Blocks` rejects self-relations and any edge creating a cycle (`Error::Conflict("would create a dependency cycle")`). `bar_for`: start and due both set gives `[start, due]`; one date plus estimate derives the other with `ceil(minutes / 480)` days (minimum 1 day); otherwise `None`. `shift_dependents`: finish-to-start. After the move, any issue blocked by a moved issue whose start is on or before the blocker's due date shifts forward so its start is the day after, keeping its duration, recursively; issues are never pulled earlier. `critical_path`: the longest chain by total days through blocking edges among issues that have bars, ties broken by lowest issue id.

- [ ] **Step 1: Tests:** `blocks_cycle_rejected` (A→B, B→C, then C→A), `self_relation_rejected`, `bar_from_estimate` (due 2026-12-10, estimate 960 gives start 2026-12-09), `no_dates_no_bar`, `shift_pushes_chain` (A→B→C, A's due moves +3 days, B and C both move +3), `shift_never_pulls_earlier`, `shift_ignores_unaffected_gap` (B starts 10 days after A, A moves +2, B unchanged), `critical_path_longest_chain`.
- [ ] **Step 2 to 5:** `cargo test -p taskapp-core relations schedule`, commit `feat(core): relations and scheduling`.

## Task T6: Views, filter compiler, search

**Files:** `crates/core/src/views.rs`, `crates/core/src/search.rs`, `crates/core/tests/views.rs`.

Rules: `compile` emits a `SELECT` over `issues` with only bound parameters. Property fields compile to `json_extract(properties, '$.' || ?)` with the key passed as a parameter after validating it exists in `defs` (unknown key: `Error::Invalid`). MultiSelect `Contains` uses `EXISTS (SELECT 1 FROM json_each(...) WHERE value = ?)`. `Label` uses `issue_labels`. `StatusCategory` joins `statuses`. Archived issues are excluded unless `include_archived`. Sorting by a property sorts nulls last. `search` uses FTS5 `MATCH` with the input quoted as a phrase per token (so `-`, `:`, and quotes in user text can't break the query), ranked by `bm25`. `reindex` rewrites the issue's FTS row from title, description, and all comment bodies.

- [ ] **Step 1: Tests:** `filter_fixed_and_property`, `multiselect_contains`, `nested_and_or_not`, `unknown_property_is_invalid`, `property_key_never_inlined` (assert the SQL string has no user-supplied key text), `sort_property_nulls_last`, `archived_excluded_by_default`, `saved_view_roundtrip`, `search_finds_comment_text`, `search_tolerates_punctuation` (`"ZF-1: fix 'x'"` returns without error).
- [ ] **Step 2 to 5:** `cargo test -p taskapp-core --test views`, commit `feat(core): views, filters, search`.

## Task T7: Quick-add parser

**Files:** `crates/core/src/quickadd.rs` (unit tests inline).

Grammar (tokens anywhere, case-insensitive; everything not consumed becomes the title, whitespace collapsed):
- `p0` to `p4` sets priority.
- `#slug` sets the project slug. `@name` adds a label name.
- `by <date>` or `due <date>` sets due; `start <date>` sets start; `at <time>` (`3pm`, `15:30`, `9:15am`) turns the due into a `Due::DateTime` in `tz`.
- Dates: `today`, `tomorrow`, weekday names and 3-letter forms (next occurrence after today; today's weekday means 7 days out), `in N days|weeks`, `next week` (next Monday), ISO `2026-12-03`, `dec 3` / `3 dec` (this year, or next year if already past).
- `~30m`, `~2h`, `~1d` sets estimate (1d = 480 minutes).
- `create_from`: resolves the project slug and label names. An unknown project or label leaves its token in the title and creates no project or label (nothing is lost), then calls `issues::create` with the default `unstarted` status.

- [ ] **Step 1: Tests** (today = 2026-11-20, a Friday, tz `America/New_York`): `full_example` (`"renew lease by fri p1 #ohio-move"` gives title "renew lease", due 2026-11-27, priority 1, slug "ohio-move"), `friday_on_friday_is_next_week`, `time_makes_datetime` (`"call bank tomorrow at 3pm"` is 2026-11-21 15:00 local), `estimate_units`, `month_day_rolls_to_next_year` (`"jan 5"` is 2027-01-05), `labels_and_start`, `no_tokens_whole_title`, `unknown_project_kept_in_title` (async, against in-memory db).
- [ ] **Step 2 to 5:** `cargo test -p taskapp-core quickadd`, commit `feat(core): quick-add parser`.

## Task T8: Google Calendar planner (pure)

**Files:** `crates/gcal/src/planner.rs` (unit tests inline).

Rules from the spec: issues not archived and not in a completed/canceled status, with `Due::DateTime`, give a timed event `[due, due + 15 min]`, `reminder_minutes = Some(0)`, `managed_key = "issue:<id>"`. Issues with `Due::Date` group by date into one all-day event per date, `managed_key = "digest:<YYYY-MM-DD>"`, summary `Due today: 1 task` or `Due today: N tasks`, description listing `ZF-12 Title` lines each followed by `taskapp://issue/ZF-12`. Digest reminder: see the verification step in T24; the planner exposes `pub const DIGEST_NOTIFY_LOCAL: NaiveTime = 08:00` and sets `reminder_minutes` from `digest_reminder_minutes()` (T24 confirms the encoding; until then return `None` and a test marks it ignored). Work blocks give timed events `managed_key = "block:<id>"`, summary = issue title, no reminder. Every description starts with the issue key, title, priority, and project name so the notification is useful alone. `plan`: create desired keys missing from existing, update when `hash` differs, delete existing keys not desired.

- [ ] **Step 1: Tests:** `timed_due_is_15_min_with_popup`, `date_only_grouped_into_digest`, `single_task_digest_singular`, `completed_and_archived_excluded`, `no_due_no_event`, `digest_uses_local_date` (a `Due::DateTime` at 2026-11-21 01:00 UTC with tz New York is a timed event on 2026-11-20 local, and two `Due::Date(2026-11-21)` issues land on the 21st digest), `plan_creates_updates_deletes`, `unchanged_event_produces_no_op`.
- [ ] **Step 2 to 5:** `cargo test -p taskapp-gcal planner`, commit `feat(gcal): event planner`.

## Task T9: Backups and snapshots

**Files:** `crates/core/src/backup.rs`, `crates/core/tests/backup.rs`.

Rules: `snapshot` uses `VACUUM INTO` to `<dir>/snapshots/<UTC timestamp>-<reason-slug>.db`. `nightly` writes `<dir>/nightly/<YYYY-MM-DD>.db` only if today's file doesn't exist. `prune` keeps the newest 14 nightly and newest 30 snapshots and returns what it deleted. `restore` copies a backup over the target path only after opening it read-only and running `PRAGMA integrity_check` == `ok`.

- [ ] **Step 1: Tests:** `snapshot_is_openable_and_complete`, `nightly_once_per_day`, `prune_keeps_14_and_30`, `restore_roundtrip` (create 3 issues, snapshot, add 1, restore, 3 issues), `restore_rejects_corrupt_file`.
- [ ] **Step 2 to 5:** `cargo test -p taskapp-core --test backup`, commit `feat(core): backups`.

## Task T10: Reminders Swift helper

**Files:** `helpers/reminders-helper/Package.swift`, `Sources/RemindersHelper/main.swift`, `Sources/RemindersCore/*.swift`, `Tests/RemindersCoreTests/*.swift`.

**Produces** a CLI with JSON on stdout, errors as `{"error": "..."}` with exit code 1:
- `reminders-helper authorize` → `{"granted": true|false}` (requests full access).
- `reminders-helper lists` → `[{"id": "...", "title": "..."}]`.
- `reminders-helper fetch --list <id>` → `[{"external_id": "...", "title": "...", "notes": "..." | null, "due": null | {"date": "YYYY-MM-DD"} | {"datetime": "RFC3339 UTC", "tz": "IANA"}}]`, incomplete reminders only. `external_id` = `calendarItemExternalIdentifier`.
- `reminders-helper delete <external_id>...` → `{"deleted": [...], "missing": [...]}`.
- `reminders-helper watch --list <id>` prints `{"changed": true}` on each `EKEventStoreChanged` and runs until stdin closes.

Keep EventKit behind a `ReminderStore` protocol so the JSON mapping is unit-tested with a fake.

- [ ] **Step 1: Tests:** `fetch_maps_date_only_due`, `fetch_maps_datetime_due_with_tz`, `fetch_excludes_completed`, `delete_reports_missing`, `errors_are_json`.
- [ ] **Step 2 to 5:** `swift test --package-path helpers/reminders-helper`; then build release and run `reminders-helper lists` once by hand to confirm the permission prompt appears. Commit `feat: reminders helper`.

## Task T11: Pins

**Files:** `crates/core/src/pins.rs`, `crates/core/tests/pins.rs`.
Rules: `pin` is idempotent on `(kind, target)` and appends at the end. `target` is the issue key, project slug, view id, cycle number, note link, or URL. `list_resolved` returns pins in order with a display label (pin `title` if set, else the target's name), `archived = true` for archived issues or projects, and `route` (`/issue/ZF-3`, `/project/<slug>`, `/view/<id>`, `/cycles#<n>`, or the external link). Pins whose target no longer exists are deleted during `list_resolved` and not returned. `reorder` rejects a list that isn't exactly the current set of ids.
- [ ] **Tests:** `pin_is_idempotent`, `resolved_labels_and_routes`, `archived_issue_pin_kept_and_flagged`, `deleted_target_pin_removed`, `reorder_requires_full_set`.
- [ ] Run `cargo test -p taskapp-core --test pins`, commit `feat(core): pins`.

## UI tasks (T12 to T20, T29)

Each UI task tests with Vitest + Testing Library against `createMockApi(fixtures)`, owns only its component files, and finishes with `pnpm -C ui test -- <Component>` passing, `pnpm -C ui build` succeeding, and `pnpm -C ui shot <its route>` exiting 0, with the agent looking at the screenshot before committing. Follow Linear's general look: dense, keyboard-first, neutral colors, status and priority icons.

## Task T12: Shell, command palette, quick-add

**Files:** `ui/src/components/{Shell,CommandPalette,QuickAdd}.svelte` and their `.test.ts`.
Shell: left sidebar slot (filled by T17's `Sidebar`), main view, right issue panel, sync status indicator from `api.sync.status()` (red dot with message on `Error`). Palette (`cmd+k`): fuzzy search across commands, views, projects, and issues (`api.search.search`), arrow keys and enter. QuickAdd (`c`): one input with a live parse preview chip row (call the backend parse via an `issues_parse_quickadd` command, declared in T0 as forwarding to `quickadd::parse`).
- [ ] **Tests:** `cmd_k_opens_and_esc_closes`, `palette_runs_create_command`, `palette_finds_issue_by_key`, `quickadd_preview_shows_parsed_chips`, `quickadd_enter_creates_and_closes`, `sync_error_shows_indicator`. Commit `feat(ui): shell, palette, quick-add`.

## Task T13: List view

**Files:** `ui/src/components/views/ListView.svelte` + test. Grouped rows (by `group_by`), each row: priority icon, key, title, property chips for `columns`, status icon, due. `j`/`k` selection, `enter` opens detail, `s`/`p` open inline pickers on the selected row. Virtualize past 200 rows.
- [ ] **Tests:** `renders_groups_in_status_order`, `jk_moves_selection`, `s_changes_status_via_api`, `renders_1000_issues_under_100ms` (fake timers, assert only visible rows are in the DOM). Commit `feat(ui): list view`.

## Task T14: Table view

**Files:** `ui/src/components/views/TableView.svelte` + test. Spreadsheet grid: one column per fixed field and property in `columns`, inline editors by `PropertyKind`, arrow-key cell navigation, `enter` to edit, `esc` cancel, column add/remove/reorder writing back to the view def. Validation errors from the API show inline and revert the cell.
- [ ] **Tests:** `edit_number_property_saves`, `invalid_value_reverts_with_message`, `arrow_navigation`, `add_column_updates_view_def`. Commit `feat(ui): table view`.

## Task T15: Board view

**Files:** `ui/src/components/views/BoardView.svelte` + test. Columns by status (or by the view's `group_by` if it's a select property), drag cards between columns to update, card shows key, title, priority, labels.
- [ ] **Tests:** `columns_follow_status_positions`, `drop_updates_status`, `group_by_select_property_columns`. Commit `feat(ui): board view`.

## Task T16: Issue detail

**Files:** `ui/src/components/IssueDetail.svelte`, `ui/src/components/detail/*.svelte` + tests. Title and markdown description (edit in place), fixed fields sidebar (status, priority, project, milestone, cycle, labels, start, due with optional time, estimate), properties panel (editor per kind, only defs in scope for the issue's project), sub-issues list with add, relations (blocks, blocked by, related, duplicate) with issue search, comments with markdown, history tab from `history.list_for`.
- [ ] **Tests:** `edits_title_and_saves`, `due_time_toggle_creates_datetime`, `properties_filtered_by_project_scope`, `add_blocking_relation`, `cycle_error_shown` (mock returns Conflict), `comment_posts`, `history_tab_lists_changes`. Commit `feat(ui): issue detail`.

## Task T17: Sidebar, saved views, view editor

**Files:** `ui/src/components/{Sidebar,ViewEditor}.svelte` + tests. Sidebar: a Pinned section at the top (from `pins_list_resolved`, drag to reorder, unpin on hover), then saved views (drag to reorder), projects, cycles, settings. Every saved view, project, and cycle row has a pin toggle. View editor: filter builder (nested groups, field picker covering fixed fields and properties, operator by type, value editor by type), sorts, group by, layout, columns, include archived, save as new or update.
- [ ] **Tests:** `build_nested_filter_and_save`, `operators_match_field_type` (Date gets Lt/Gt, Select gets In), `layout_switch_persists`, `reorder_views`, `pinned_section_lists_and_reorders`, `pin_toggle_on_view_row`. Saved views can be scoped to one project, several, or all through the filter's Project condition. Commit `feat(ui): views sidebar and editor`.

## Task T18: Settings and admin pages

**Files:** `ui/src/components/settings/{General,Statuses,Properties,Projects,Cycles,Labels,Reminders,Calendar,Export,Backups}.svelte` + tests. General: issue prefix. Properties: create, rename, change kind (show the Conflict list on refusal), delete with confirm, scope to project, reorder. Reminders: authorize button, list picker from `reminders_list_lists`. Calendar: connect button, calendar picker from `gcal_list_calendars`, enable toggle, a note recommending a dedicated calendar and the Google Calendar phone app. Export: enable toggle, directory, remote, "export now". Backups: directory, "snapshot now", list with restore.
- [ ] **Tests:** `change_kind_conflict_lists_keys`, `delete_property_requires_confirm`, `reminders_list_picker_saves_setting`, `calendar_picker_saves_setting`, `export_toggle_off_by_default`. Commit `feat(ui): settings`.

## Task T19: Gantt view

**Files:** `ui/src/components/views/GanttView.svelte`, `ui/src/lib/gantt.ts` + tests. Frappe Gantt for bars, using `schedule_bars`, `schedule_critical_path`, and `schedule_shift_dependents` commands (declared in T0). Dependency arrows from blocking relations. Drag a bar: call `shift_dependents`, show the affected issues in a confirm toast ("Also moves ZF-4, ZF-7"), then apply all updates. Critical path bars get a distinct class. Roll-up bars for projects and parent issues computed in `gantt.ts` (min start, max end of children). Milestones as zero-length markers. Zoom: Day, Week, Month, Quarter. Multi-project timeline: when the view spans several projects, group rows by project, each with its roll-up bar. Projects mode (route `/projects/timeline`, rendered by this component with `mode="projects"`): one bar per project from `start_date` to `target_date`, milestones as markers, dragging a bar updates the project dates. An "Unscheduled" side lane lists issues with an estimate but no dates; dropping one onto the timeline sets `start_date` to the drop day.
- [ ] **Tests:** `rollup_spans_children`, `drag_confirms_and_applies_shift`, `critical_path_class_applied`, `drop_unscheduled_sets_start`, `zoom_levels_switch`, `multi_project_timeline_groups_by_project`, `projects_mode_drag_updates_target_date`. Commit `feat(ui): gantt`.

## Task T20: Calendar view (events view)

**Files:** `ui/src/components/views/CalendarView.svelte` + test. `@event-calendar/core` with day, week, and month layouts. Shows work blocks from `blocks_blocks_between`, timed due dates as markers, and date-only due dates as all-day items (month shows all three). Drag an issue from a side list onto a slot to create a 60-minute block (my call), drag to move, resize to change length, delete key removes. Only the app's own data is shown.
- [ ] **Tests:** `drop_creates_60_minute_block`, `move_block_calls_api`, `delete_removes_block`, `week_shows_timed_due_markers`, `month_view_shows_all_day_dues`. Commit `feat(ui): calendar view`.

## Task T29: Project page and Pinned page

**Files:** `ui/src/components/ProjectPage.svelte`, `ui/src/components/PinnedPage.svelte`, `ui/src/components/project/*.svelte` + tests.
Project page: header with name, a prominent "Open in Obsidian" button when `doc_link` is set (opens through the Tauri shell plugin; in the browser harness it opens a new tab), the description only when present (an "Add description" link otherwise), progress, milestones with target dates, start and target dates, a pin toggle, and the project's issues in any layout (reusing the view components with a filter fixed to this project). Pinned page: every resolved pin as a card, grouped by kind, drag to reorder, archived items shown dimmed with an "Archived" tag.
- [ ] **Tests:** `doc_link_button_shown_only_when_set`, `empty_description_hidden`, `project_issues_filtered_to_project`, `pinned_page_groups_by_kind`, `archived_pin_dimmed`. Commit `feat(ui): project and pinned pages`.

---

# Wave 2: adapters and sync (parallel)

Starts after every wave 1 task is merged. Each task owns its whole crate.

## Task T21: MCP server

**Files:** `crates/mcp/src/{main,tools}.rs`, `crates/mcp/tests/tools.rs`.
**Consumes:** core. stdio transport via `rmcp`. Opens the database path from `--db` or the default path. Every write uses `ChangeSource::Agent` (or `Migration` when the tool's `migration: true` argument is set).

Tools (inputs mirror the core types as JSON): `search_issues`, `query_view` (ad hoc `ViewDef` or saved view id), `get_issue` (by key, with relations, comments, children), `create_issue`, `update_issue`, `archive_issue`, `add_comment`, `add_relation`, `remove_relation`, `list_properties`, `create_property`, `rename_property`, `change_property_kind`, `delete_property`, `set_property`, `list_statuses`, `create_status`, `list_projects`, `create_project`, `create_milestone`, `list_cycles`, `create_cycle`, `list_labels`, `create_label`, `list_pins`, `pin`, `unpin`, `update_project` (including `doc_link`), `bulk_update` (list of `{key, patch}`), `get_history`.

Snapshot rule (my call for "bulk change"): `bulk_update` with more than 10 issues, and every `change_property_kind` and `delete_property`, first calls `backup::snapshot(reason = tool name)`. `create_issue` accepts `created_at`, `completed_at`, and `properties` including `linear_id`, so the Linear migration keeps history. Tool descriptions say plainly what each does and that this server never touches Linear.

- [ ] **Tests:** `every_tool_listed`, `create_issue_with_migration_timestamps`, `bulk_update_over_10_snapshots_first`, `delete_property_snapshots_first`, `writes_record_agent_source`, `errors_are_tool_errors_not_crashes`.
- [ ] Run `cargo test -p taskapp-mcp`, then connect it to Claude Code (`claude mcp add taskapp -- taskapp-mcp`) and list tools once by hand. Commit `feat: mcp server`.

## Task T22: CLI

**Files:** `crates/cli/src/{main,commands}.rs`, `crates/cli/tests/cli.rs` (using `assert_cmd`).
Subcommands mirror the MCP tools one-to-one (`taskapp issue create|update|get|archive|search`, `taskapp view query`, `taskapp property ...`, `taskapp project ...`, `taskapp cycle ...`, `taskapp label ...`, `taskapp comment add`, `taskapp relation add|remove`, `taskapp pin add|remove|list`, `taskapp history`), plus `taskapp add "<quick-add text>"`. `--json` prints machine output; default output is a compact table. `--db <path>` overrides the database. Writes use `ChangeSource::Cli`. Same snapshot rule as T21.
- [ ] **Tests:** `add_quickadd_prints_key`, `json_output_parses`, `issue_update_by_key`, `unknown_key_exits_nonzero_with_message`, `property_change_kind_snapshots_first`. Commit `feat: cli`.

## Task T23: Reminders import

**Files:** `crates/reminders/src/{helper,import}.rs`, `crates/reminders/tests/import.rs` (uses a fake helper script that prints fixture JSON).
Rules: fetch from the list in the `reminders_list_id` setting (none set: return `Ok` with zero imported). For each reminder not in `import_log` (source `reminders`): create the issue with `quickadd::create_from` on the title; notes become the description; a reminder due overrides any parsed due; write the `import_log` row in the same transaction as the issue. Then delete all imported reminders in one helper call and mark `source_deleted = 1`. On startup, retry deletes for rows with `source_deleted = 0`. A helper failure sets `sync_status.reminders = Error`.
- [ ] **Tests:** `imports_and_deletes`, `reminder_due_overrides_parsed_due`, `reimport_after_crash_is_idempotent` (log row exists, delete never happened: no duplicate issue, delete retried), `no_list_configured_is_noop`, `helper_error_sets_status`. Commit `feat: reminders import`.

## Task T24: Google Calendar auth, client, reconcile

**Files:** `crates/gcal/src/{auth,client,reconcile}.rs`, `crates/gcal/tests/reconcile.rs` (HTTP mocked with `wiremock`).
Auth: OAuth 2 installed-app flow with PKCE and a loopback redirect on a random port, scopes `calendar.events` and `calendar.calendarlist.readonly`. The client ID comes from `GOOGLE_CLIENT_ID` at build time. The refresh token is stored in the macOS Keychain (`keyring`, service `taskapp.gcal`). `README` steps: create a Google Cloud project, enable the Calendar API, create a Desktop OAuth client, and **publish the app to production** (never Testing, which expires tokens after 7 days).
Client: events are written with `extendedProperties.private = {"taskapp": "1", "managed_key": "<key>"}`.
Reconcile: load `gcal_events`, list the calendar's events with `privateExtendedProperty=taskapp=1` to catch manual deletions or edits, treat missing ones as absent, run `planner::plan`, apply ops, update `gcal_events`. Manual edits to managed events are overwritten (one-way). A 401 after a refresh attempt sets `sync_status.gcal = Error("Google Calendar disconnected. Reconnect in Settings.")`.
- [ ] **Step 1: Verify the digest reminder encoding first.** Against a real test calendar, create an all-day event and read back what Google stores when "on the day at 8:00 AM" is chosen in the Google Calendar web UI. If the API can express it, implement `digest_reminder_minutes()` with that value and un-ignore the T8 test. If it can't (reminder minutes are counted before midnight and can't be negative), use the fallback: the digest stays all-day with no reminder, and the planner adds one 15-minute timed event at 08:00 local titled the same as the digest, with `reminder_minutes = Some(0)` and `managed_key = "digest-alert:<date>"`. Record which path was taken in `crates/gcal/README.md` and tell Zach.
- [ ] **Tests:** `reconcile_creates_from_scratch`, `reconcile_recreates_deleted_event`, `reconcile_overwrites_manual_edit`, `completed_issue_event_deleted`, `revoked_token_sets_error_state`, `disabled_setting_is_noop`. Commit `feat(gcal): auth and reconcile`.

## Task T25: GitHub export

**Files:** `crates/export/src/{write,rebuild,git}.rs`, `crates/export/tests/export.rs`.
Layout: `issues/<KEY>.md` (YAML frontmatter with every fixed field, label names, relations as keys, properties; body = description; comments appended under `## Comments` with timestamps), `projects/<slug>.yaml` (with milestones, description, and doc link), `cycles.yaml`, `labels.yaml`, `statuses.yaml`, `properties.yaml`, `views.yaml`, `pins.yaml`, `settings.yaml` (excluding tokens and paths). Output is deterministic (sorted keys, stable ordering) so unchanged data gives no diff. Files for deleted issues are removed. Git via the `git` CLI: `add -A`, commit only if there are changes, push if a remote is set.
- [ ] **Tests:** `export_is_deterministic` (two exports, byte-identical), `rebuild_roundtrip` (export, rebuild into a new db, export again: identical tree), `removed_issue_file_deleted`, `no_changes_no_commit`, `secrets_never_exported`. Commit `feat: github export`.

---

# Wave 3: integration

## Task T26: App sync loop, deep links, live refresh

**Depends on:** all of wave 2. **Files:** `app/src-tauri/src/{sync_loop,deeplink}.rs`, `app/src-tauri/tests/sync_loop.rs`.
Sync loop (a single tokio task, so syncs never overlap): on app start run backup nightly check, Reminders import with delete retries, gcal reconcile, then export. Then:
- Reminders: spawn `reminders-helper watch` (bundled as a Tauri sidecar); import on each change line, plus every 5 minutes.
- Calendar: reconcile 10 seconds after the last issue, status, or work block change (debounced), plus every 15 minutes.
- Export: hourly when enabled, and right after any MCP or CLI snapshot.
- Backups: nightly check every hour, prune after each.
- `sync_now` runs all of them in order and returns `SyncStatus`.
Live refresh: poll `Db::data_version()` every second, and on change emit a Tauri event `db-changed` that the UI's `refreshAll` listens to (add the listener in `ui/src/lib/stores.ts`, the one shared file this task may edit). Deep links: `taskapp://issue/<KEY>` focuses the window and opens the issue.
- [ ] **Tests:** `syncs_never_overlap`, `calendar_debounce_coalesces_changes`, `app_refreshes_on_external_change` (write via a second Db handle, event emitted within 2 s), `deeplink_parses_key`, `sync_now_reports_each_state`. Commit `feat(app): sync loop, live refresh, deep links`.

## Task T27: End-to-end QA and the Linear migration runbook

**Files:** `docs/qa-checklist.md`, `docs/linear-migration.md`, fixes in whichever files the QA finds broken (each fix gets a regression test in its owning file).
- [ ] **Step 1:** Build the signed app with `cargo tauri build` and install it.
- [ ] **Step 2:** Run the checklist by hand and record results: quick-add and command palette; create and change custom properties of every kind, including a refused type change; views in all five layouts, including a multi-project timeline and projects mode; pin an issue, a view, and an Obsidian note and reorder them on the Pinned page; a project with only an Obsidian link opens the note; a Gantt drag that cascades; a Siri-added reminder imported within a minute and removed from Reminders; a timed due date and a date-only due date appearing in the chosen Google calendar with notifications on the phone via the Google Calendar app; manually deleting an app event in Google and seeing it restored after `sync_now`; completing an issue removes its event; export to a private repo and `rebuild` into a scratch database; `taskapp add` and an MCP write both appear in the open app within 2 seconds; restore from a snapshot.
- [ ] **Step 3:** Write `docs/linear-migration.md`: a prompt for an agent with both MCP servers connected. Read only from Linear. Create statuses, labels, projects, milestones, cycles, and property definitions first, including `linear_id` (Text). Then issues with `migration: true`, original `created_at` and `completed_at`, and `linear_id`. Then sub-issue links, relations, and comments. Finish with a count comparison per project. Never change anything in Linear.
- [ ] **Step 4:** Run `cargo test --workspace && pnpm -C ui test && swift test --package-path helpers/reminders-helper`. Expected: all pass.
- [ ] **Step 5: Commit** `git commit -m "docs: qa checklist and linear migration runbook"`

---

# Spec coverage map

| Spec section | Tasks |
|---|---|
| 5.1 Data model, history | T0, T2, T3 |
| 5.2 Linear baseline | T2, T4, T5, T6, T7, T12 to T17 |
| 5.3 Views, timelines, calendar view | T6, T5, T13 to T15, T19, T20 |
| 5.4 Reminders capture | T10, T23, T18, T26 |
| 5.5 Google Calendar | T8, T24, T18, T26 |
| 5.6 GitHub export | T25, T18, T26 |
| 5.7 MCP, CLI, Linear migration | T21, T22, T27 |
| 5.8 Backups | T9, T21, T22, T26 |
| 5.9 Pinned items | T11, T17, T29, T21, T22, T25 |
| 5.10 Development harness | T1, T28, every UI task's shot step |
| Optional project description, Obsidian link | T0, T4, T29 |
| Multi-project timelines, projects mode | T0 seed view, T19 |
| Anti-goal: feels slow | T13 render test, T6 indexes, T26 live refresh |
| Anti-goal: property changes need code | T3, T18 |
| Anti-goal: agent damage | T9, T21, T22, history in T0 |
