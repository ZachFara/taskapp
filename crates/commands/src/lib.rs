//! One plain async function per UI command, plus `dispatch(name, args)` for the dev server.
//!
//! The list in `with_commands!` is the only registration point: it defines the functions, the dispatch
//! table, and (through the macro the app passes in) the Tauri wrappers. Commands that need a sync crate
//! are not here; the app defines them itself.

mod step_label;

use std::{future::Future, path::PathBuf};

use chrono::{DateTime, NaiveDate, Utc};
use serde_json::Value;
use taskapp_core::{
    Applied, AppliedTemplate, Bar, BulkPatch, Change, ChangeSource, Comment, CommentId, Cycle,
    CycleId, Db, Error, ImportPlan, Issue, IssueId, IssuePatch, IssueStats, IssueTemplate, Label,
    LabelId, Measure, Milestone, MilestoneId, NewIssue, ParsedQuickAdd, Pin, PinId, PinKind,
    ProgressScope, ProgressSeries, Project, ProjectId, PropertyDef, PropertyKind, Relation,
    RelationKind, RelationKindDef, Reschedule, ResolvedPin, Result, SavedView, SelectOption,
    Status, StatusCategory, StatusId, SyncState, SyncStatus, TemplateId, TemplateInput, UndoStatus,
    VelocityPeriod, ViewDef, ViewId, ViewTemplate, ViewTemplateId, WorkBlock, WorkBlockId,
    WorkspaceStatus, backup, blocks, bulk, comments, cycles, history, import, insights, issues,
    labels, pins, projects, properties, quickadd, relation_kinds, relations, schedule, search,
    settings, statuses, templates, undo, view_templates, views, workspace,
};

pub struct AppState {
    pub db: Db,
    pub helper_path: PathBuf,
    /// IANA name of the system time zone.
    pub tz: String,
}

fn today(s: &AppState) -> Result<NaiveDate> {
    let tz: chrono_tz::Tz =
        s.tz.parse()
            .map_err(|_| Error::Invalid(format!("unknown time zone {}", s.tz)))?;
    Ok(Utc::now().with_timezone(&tz).date_naive())
}

/// The issues a saved view currently shows.
async fn view_issues(s: &AppState, id: ViewId) -> Result<Vec<Issue>> {
    let view = views::list(&s.db).await?.into_iter().find(|v| v.id == id);
    views::query(
        &s.db,
        &view
            .ok_or_else(|| Error::NotFound(format!("view {}", id.0)))?
            .def,
    )
    .await
}

/// Every issue, archived included.
fn all_issues_def() -> ViewDef {
    ViewDef {
        include_archived: true,
        ..ViewDef::default()
    }
}

/// A function the dispatch table can call: named args as a JSON object in, JSON out.
type Handler = for<'a> fn(
    &'a AppState,
    Value,
) -> std::pin::Pin<Box<dyn Future<Output = Result<Value>> + Send + 'a>>;

fn invalid(e: &serde_json::Error) -> Error {
    Error::Invalid(e.to_string())
}

macro_rules! define {
    ($($name:ident($s:ident $(, $a:ident : $t:ty)*) -> $r:ty $body:block)*) => {
        $(
            #[doc = "Forwards to core with `ChangeSource::Ui`.\n\n# Errors\nFails as the core function it forwards to does."]
            pub async fn $name($s: &AppState $(, $a: $t)*) -> Result<$r> {
                // One command call is one undoable step, named once the outcome is known.
                let args = serde_json::json!({ $(stringify!($a): &$a),* });
                let name = stringify!($name);
                history::step(&step_label::step_label(name, &args, None), async move {
                    let out: Result<$r> = (async move $body).await;
                    let out = out?;
                    let shown = serde_json::to_value(&out).unwrap_or(Value::Null);
                    history::relabel(&$s.db, &step_label::step_label(name, &args, Some(&shown))).await?;
                    Ok(out)
                }).await
            })*

        const TABLE: &[(&str, Handler)] = &[$((stringify!($name), |st, args| Box::pin(async move {
            #[derive(serde::Deserialize)]
            struct Args { $($a: $t),* }
            let Args { $($a),* } = serde_json::from_value(args).map_err(|e| invalid(&e))?;
            serde_json::to_value($name(st, $($a),*).await?).map_err(|e| invalid(&e))
        })),)*];
    };
}

/// Calls the command `name` with `args`, a JSON object of its named arguments.
///
/// # Errors
/// `NotFound` for an unknown name, `Invalid` for bad arguments, otherwise whatever the command returns.
pub async fn dispatch(state: &AppState, name: &str, args: Value) -> Result<Value> {
    let (_, handler) = TABLE
        .iter()
        .find(|(n, _)| *n == name)
        .ok_or_else(|| Error::NotFound(format!("command {name}")))?;
    handler(state, args).await
}

/// Every command name `dispatch` serves.
pub fn names() -> impl Iterator<Item = &'static str> {
    TABLE.iter().map(|(n, _)| *n)
}

/// Calls `$m! { <every command> }`. The app passes its own wrapper-generating macro.
#[macro_export]
macro_rules! with_commands {
    ($m:ident) => {
        $m! {
            issues_create(s, input: NewIssue) -> Issue { issues::create(&s.db, input, ChangeSource::Ui).await }
            issues_update(s, id: IssueId, patch: IssuePatch) -> Issue {
                issues::update(&s.db, id, patch, ChangeSource::Ui).await
            }
            issues_bulk_update(s, ids: Vec<IssueId>, patch: BulkPatch) -> () {
                bulk::update(&s.db, &ids, patch, ChangeSource::Ui).await
            }
            issues_duplicate(s, ids: Vec<IssueId>) -> Vec<Issue> {
                bulk::duplicate(&s.db, &ids, ChangeSource::Ui).await
            }
            issues_get(s, id: IssueId) -> Issue { issues::get(&s.db, id).await }
            issues_get_by_key(s, key: String) -> Issue { issues::get_by_key(&s.db, &key).await }
            issues_archive(s, id: IssueId) -> () { issues::archive(&s.db, id, ChangeSource::Ui).await }
            issues_restore(s, id: IssueId) -> () { issues::restore(&s.db, id, ChangeSource::Ui).await }
            issues_children(s, id: IssueId) -> Vec<Issue> { issues::children(&s.db, id).await }
            issues_parse_quickadd(s, input: String) -> ParsedQuickAdd {
                quickadd::parse_for(&s.db, &input, today(s)?, &s.tz).await
            }
            issues_create_from_quickadd(s, input: String) -> Issue {
                quickadd::create_from(&s.db, &input, today(s)?, &s.tz, ChangeSource::Ui).await
            }

            workspace_status(s) -> WorkspaceStatus { workspace::status(&s.db).await }
            workspace_setup(s, name: String, key: String, timezone: Option<String>) -> WorkspaceStatus {
                workspace::setup(&s.db, &name, &key, timezone.as_deref()).await
            }

            statuses_list(s) -> Vec<Status> { statuses::list(&s.db).await }
            statuses_create(s, name: String, category: StatusCategory, color: String, icon: Option<String>) -> Status {
                statuses::create(&s.db, &name, category, &color, icon, ChangeSource::Ui).await
            }
            statuses_update(s, id: StatusId, name: Option<String>, color: Option<String>, position: Option<i32>, icon: Option<String>)
                -> Status { statuses::update(&s.db, id, name, position, taskapp_core::palette::Look { icon, color }, ChangeSource::Ui).await }
            statuses_reorder(s, ids_in_order: Vec<StatusId>) -> () {
                statuses::reorder(&s.db, &ids_in_order, ChangeSource::Ui).await
            }
            statuses_delete(s, id: StatusId, move_issues_to: StatusId) -> () {
                statuses::delete(&s.db, id, move_issues_to).await
            }

            properties_list_defs(s) -> Vec<PropertyDef> { properties::list_defs(&s.db).await }
            properties_create_def(s, def: PropertyDef) -> PropertyDef {
                properties::create_def(&s.db, def, ChangeSource::Ui).await
            }
            properties_rename_def(s, key: String, new_name: String) -> PropertyDef {
                properties::rename_def(&s.db, &key, &new_name, ChangeSource::Ui).await
            }
            properties_change_kind(s, key: String, kind: PropertyKind, options: Vec<SelectOption>) -> PropertyDef {
                properties::change_kind(&s.db, &key, kind, options, ChangeSource::Ui).await
            }
            properties_reorder(s, keys: Vec<String>) -> () { properties::reorder(&s.db, &keys).await }
            properties_set_scope(s, key: String, project: Option<ProjectId>) -> PropertyDef {
                properties::set_scope(&s.db, &key, project, ChangeSource::Ui).await
            }
            properties_delete_def(s, key: String) -> () { properties::delete_def(&s.db, &key, ChangeSource::Ui).await }
            properties_set_value(s, issue: IssueId, key: String, value: Option<Value>) -> Issue {
                properties::set_value(&s.db, issue, &key, value, ChangeSource::Ui).await
            }

            projects_list(s) -> Vec<Project> { projects::list(&s.db).await }
            projects_create(s, slug: String, name: String, icon: Option<String>, color: Option<String>) -> Project {
                projects::create(&s.db, &slug, &name, icon, color, ChangeSource::Ui).await
            }
            projects_update(s, project: Project) -> Project { projects::update(&s.db, project, ChangeSource::Ui).await }
            projects_delete(s, id: ProjectId) -> () { projects::delete(&s.db, id, ChangeSource::Ui).await }
            projects_progress(s, id: ProjectId) -> (u32, u32) { projects::progress(&s.db, id).await }
            projects_list_milestones(s, project: ProjectId) -> Vec<Milestone> {
                projects::list_milestones(&s.db, project).await
            }
            projects_create_milestone(s, project: ProjectId, name: String, target_date: Option<NaiveDate>)
                -> Milestone {
                projects::create_milestone(&s.db, project, &name, target_date, ChangeSource::Ui).await
            }
            projects_update_milestone(s, milestone: Milestone) -> Milestone {
                projects::update_milestone(&s.db, milestone, ChangeSource::Ui).await
            }
            projects_delete_milestone(s, id: MilestoneId) -> () {
                projects::delete_milestone(&s.db, id, ChangeSource::Ui).await
            }

            cycles_list(s) -> Vec<Cycle> { cycles::list(&s.db).await }
            cycles_create(s, name: Option<String>, start_date: NaiveDate, end_date: NaiveDate, icon: Option<String>, color: Option<String>) -> Cycle {
                cycles::create(&s.db, name, start_date, end_date, taskapp_core::palette::Look { icon, color }, ChangeSource::Ui).await
            }
            cycles_update(s, cycle: Cycle) -> Cycle { cycles::update(&s.db, cycle, ChangeSource::Ui).await }
            cycles_delete(s, id: CycleId) -> () { cycles::delete(&s.db, id, ChangeSource::Ui).await }

            labels_list(s) -> Vec<Label> { labels::list(&s.db).await }
            labels_create(s, name: String, color: String, icon: Option<String>) -> Label {
                labels::create(&s.db, &name, &color, icon, ChangeSource::Ui).await
            }
            labels_update(s, label: Label) -> Label { labels::update(&s.db, label, ChangeSource::Ui).await }
            labels_delete(s, id: LabelId) -> () { labels::delete(&s.db, id, ChangeSource::Ui).await }
            labels_add_label(s, issue: IssueId, label: LabelId) -> () {
                labels::add_label(&s.db, issue, label, ChangeSource::Ui).await
            }
            labels_remove_label(s, issue: IssueId, label: LabelId) -> () {
                labels::remove_label(&s.db, issue, label, ChangeSource::Ui).await
            }

            templates_list(s) -> Vec<IssueTemplate> { templates::list(&s.db).await }
            templates_create(s, input: TemplateInput) -> IssueTemplate {
                templates::create(&s.db, input, ChangeSource::Ui).await
            }
            templates_update(s, id: TemplateId, input: TemplateInput) -> IssueTemplate {
                templates::update(&s.db, id, input, ChangeSource::Ui).await
            }
            templates_delete(s, id: TemplateId) -> () { templates::delete(&s.db, id, ChangeSource::Ui).await }
            templates_apply(s, id: TemplateId, overrides: NewIssue) -> AppliedTemplate {
                templates::apply(&s.db, id, overrides, ChangeSource::Ui).await
            }

            comments_list(s, issue: IssueId) -> Vec<Comment> { comments::list(&s.db, issue).await }
            comments_add_comment(s, issue: IssueId, body: String) -> Comment {
                comments::add_comment(&s.db, issue, &body, None, ChangeSource::Ui).await
            }
            comments_update(s, id: CommentId, body: String) -> Comment {
                comments::update(&s.db, id, &body, ChangeSource::Ui).await
            }
            comments_delete(s, id: CommentId) -> () { comments::delete(&s.db, id, ChangeSource::Ui).await }

            relations_add(s, rel: Relation) -> () { relations::add(&s.db, rel, ChangeSource::Ui).await }
            relations_remove(s, rel: Relation) -> () { relations::remove(&s.db, rel, ChangeSource::Ui).await }
            relations_for_issue(s, id: IssueId) -> Vec<Relation> { relations::for_issue(&s.db, id).await }
            relation_kinds_list(s) -> Vec<RelationKindDef> { relation_kinds::list(&s.db).await }
            relation_kinds_create(s, name: String, forward: String, reverse: String, symmetric: bool) -> RelationKindDef {
                relation_kinds::create(&s.db, &name, &forward, &reverse, symmetric, ChangeSource::Ui).await
            }
            relation_kinds_update(s, kind: RelationKindDef) -> RelationKindDef {
                relation_kinds::update(&s.db, kind, ChangeSource::Ui).await
            }
            relation_kinds_delete(s, id: RelationKind) -> () { relation_kinds::delete(&s.db, &id, ChangeSource::Ui).await }

            schedule_bars(s, view_id: ViewId) -> Vec<Bar> {
                Ok(view_issues(s, view_id).await?.iter().filter_map(schedule::bar_for).collect())
            }
            schedule_critical_path(s, view_id: ViewId) -> Vec<IssueId> {
                Ok(schedule::critical_path(&view_issues(s, view_id).await?, &relations::all_blocking(&s.db).await?))
            }
            schedule_shift_dependents(s, moved: Reschedule) -> Vec<Reschedule> {
                let all = views::query(&s.db, &all_issues_def()).await?;
                Ok(schedule::shift_dependents(&all, &relations::all_blocking(&s.db).await?, &moved))
            }

            views_query(s, def: ViewDef) -> Vec<Issue> { views::query(&s.db, &def).await }
            views_list(s) -> Vec<SavedView> { views::list(&s.db).await }
            views_save(s, id: Option<ViewId>, name: String, def: ViewDef, icon: Option<String>, color: Option<String>) -> SavedView {
                views::save(&s.db, id, &name, def, taskapp_core::palette::Look { icon, color }, ChangeSource::Ui).await
            }
            views_delete(s, id: ViewId) -> () { views::delete(&s.db, id).await }
            views_reorder(s, ids: Vec<ViewId>) -> () { views::reorder(&s.db, &ids).await }
            views_stats(s, def: ViewDef) -> Vec<IssueStats> { views::stats(&s.db, &def).await }
            views_create_from_template(s, template_id: ViewTemplateId, name: Option<String>) -> SavedView {
                view_templates::create_view(&s.db, template_id, name.as_deref(), ChangeSource::Ui).await
            }
            views_save_as_template(s, view_id: ViewId, name: Option<String>) -> ViewTemplate {
                view_templates::from_view(&s.db, view_id, name.as_deref(), ChangeSource::Ui).await
            }
            views_reset_to_template(s, view_id: ViewId) -> SavedView {
                view_templates::reset_view(&s.db, view_id, ChangeSource::Ui).await
            }
            view_templates_list(s) -> Vec<ViewTemplate> { view_templates::list(&s.db).await }
            view_templates_create(s, name: String, def: ViewDef, icon: Option<String>, color: Option<String>) -> ViewTemplate {
                view_templates::create(&s.db, &name, def, icon, color, ChangeSource::Ui).await
            }
            view_templates_update(s, template: ViewTemplate) -> ViewTemplate {
                view_templates::update(&s.db, template, ChangeSource::Ui).await
            }
            view_templates_delete(s, id: ViewTemplateId) -> () { view_templates::delete(&s.db, id, ChangeSource::Ui).await }

            import_csv_plan(s, csv: String, mapping: Option<Vec<(String, String)>>) -> ImportPlan {
                let mapping = mapping.map(|m| m.into_iter().collect());
                import::csv::plan(&s.db, &csv, mapping.as_ref()).await
            }
            import_apply(s, plan: ImportPlan) -> Applied { import::apply(&s.db, &plan).await }

            insights_progress(s, scope: ProgressScope, measure: Measure, from: Option<NaiveDate>, to: Option<NaiveDate>)
                -> ProgressSeries { insights::progress(&s.db, scope, measure, from, to).await }
            insights_velocity(s, scope: ProgressScope, measure: Measure, periods: u32) -> Vec<VelocityPeriod> {
                insights::velocity(&s.db, scope, measure, periods).await
            }

            search_search(s, text: String, limit: u32) -> Vec<Issue> { search::search(&s.db, &text, limit).await }

            pins_pin(s, kind: PinKind, target: String, title: Option<String>) -> Pin {
                pins::pin(&s.db, kind, &target, title).await
            }
            pins_set(s, kind: PinKind, targets: Vec<String>, pinned: bool) -> () {
                pins::set(&s.db, kind, &targets, pinned).await
            }
            pins_unpin(s, id: PinId) -> () { pins::unpin(&s.db, id).await }
            pins_reorder(s, ids_in_order: Vec<PinId>) -> () { pins::reorder(&s.db, &ids_in_order).await }
            pins_list_resolved(s) -> Vec<ResolvedPin> { pins::list_resolved(&s.db).await }

            blocks_create(s, issue: IssueId, start: DateTime<Utc>, end: DateTime<Utc>) -> WorkBlock {
                blocks::create_block(&s.db, issue, start, end).await
            }
            blocks_move(s, id: WorkBlockId, start: DateTime<Utc>, end: DateTime<Utc>) -> WorkBlock {
                blocks::move_block(&s.db, id, start, end).await
            }
            blocks_delete(s, id: WorkBlockId) -> () { blocks::delete_block(&s.db, id).await }
            blocks_between(s, from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<WorkBlock> {
                blocks::blocks_between(&s.db, from, to).await
            }

            history_list_for(s, entity: String, entity_id: String) -> Vec<Change> {
                history::list_for(&s.db, &entity, &entity_id).await
            }
            history_undo(s) -> UndoStatus { undo::undo(&s.db).await }
            history_redo(s) -> UndoStatus { undo::redo(&s.db).await }
            history_undo_status(s) -> UndoStatus { undo::undo_status(&s.db).await }
            settings_get(s, key: String) -> Option<Value> { settings::get(&s.db, &key).await }
            settings_set(s, key: String, value: Value) -> () { settings::set_from_user(&s.db, &key, &value).await }
            backup_snapshot(s, dir: String, reason: String) -> String {
                Ok(backup::snapshot(&s.db, dir.as_ref(), &reason).await?.to_string_lossy().into_owned())
            }

            backup_list(_s, dir: String) -> Vec<String> {
                Ok(backup::list(dir.as_ref())?.into_iter().map(|p| p.to_string_lossy().into_owned()).collect())
            }
            sync_status(s) -> SyncStatus {
                Ok(settings::get(&s.db, "sync_status").await?.unwrap_or(SyncStatus {
                    reminders: SyncState::Off,
                    gcal: SyncState::Off,
                    export: SyncState::Off,
                    backup: SyncState::Off,
                }))
            }
        }
    };
}

with_commands!(define);
