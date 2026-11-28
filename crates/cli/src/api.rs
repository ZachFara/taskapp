//! JSON access to the same core operations used by the app.

use std::{future::Future, path::Path};

use chrono::{DateTime, NaiveDate, Utc};
use serde_json::{Value, json};
use taskapp_core::{
    Applied, AppliedTemplate, Attachment, AttachmentData, AttachmentId, Bar, BulkPatch, Change,
    ChangeSource, Comment, CommentId, Cycle, CycleId, Db, Decision, DecisionId, DecisionInput,
    DecisionStatus, Error, Event, EventId, EventInput, FlowKind, FolderSection, Forecast,
    ForecastQuery, ImportPlan, Issue, IssueId, IssuePatch, IssueStats, IssueTemplate, Label,
    LabelId, Measure, Metrics, MetricsQuery, Milestone, MilestoneId, NewIssue, ParsedQuickAdd, Pin,
    PinId, PinKind, Placement, ProgressScope, ProgressSeries, Project, ProjectId, ProjectRow,
    ProjectStatus, ProjectStatusCategory, ProjectStatusId, PropertyAppliesTo, PropertyDef,
    PropertyKind, Relation, RelationKind, RelationKindDef, Reschedule, ResolvedPin, Result,
    SavedView, SelectOption, StaleIssue, Status, StatusCategory, StatusId, SyncState, SyncStatus,
    TemplateId, TemplateInput, UndoStatus, VelocityPeriod, ViewDef, ViewEntity, ViewFolder,
    ViewFolderId, ViewId, ViewTemplate, ViewTemplateId, WorkBlock, WorkBlockId, WorkspaceStatus,
    attachments, backup, blocks, bulk, comments, cycles, decisions, events, forecast, history,
    import, insights, issues, labels, metrics, pins, project_statuses, projects, properties,
    quickadd, relation_kinds, relations, schedule, search, settings, stale, statuses, templates,
    undo, view_folders, view_templates, views, workspace,
};

struct State<'a> {
    db: &'a Db,
    tz: String,
    backup_dir: &'a Path,
}

fn today(s: &State<'_>) -> Result<NaiveDate> {
    let tz: chrono_tz::Tz =
        s.tz.parse()
            .map_err(|_| Error::Invalid(format!("unknown time zone {}", s.tz)))?;
    Ok(Utc::now().with_timezone(&tz).date_naive())
}

fn all_issues_def() -> ViewDef {
    ViewDef {
        include_archived: true,
        ..ViewDef::default()
    }
}

type Handler = for<'a> fn(
    &'a State<'a>,
    Value,
) -> std::pin::Pin<Box<dyn Future<Output = Result<Value>> + Send + 'a>>;

struct Operation {
    name: &'static str,
    args: &'static [(&'static str, &'static str)],
    result: &'static str,
    handler: Handler,
}

macro_rules! define_api {
    ($($name:ident($s:ident $(, $a:ident : $t:ty)*) -> $r:ty $body:block)*) => {
        const OPERATIONS: &[Operation] = &[$(Operation {
            name: stringify!($name),
            args: &[$((stringify!($a), stringify!($t))),*],
            result: stringify!($r),
            handler: |$s, args| Box::pin(async move {
                #[derive(serde::Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Args { $($a: $t),* }
                let large_bulk = stringify!($name) == "issues_bulk_update"
                    && args.get("ids").and_then(Value::as_array).is_some_and(|ids| ids.len() > 10);
                let Args { $($a),* } = serde_json::from_value(args)
                    .map_err(|e| Error::Invalid(e.to_string()))?;
                match stringify!($name) {
                    "properties_delete_def" => crate::commands::snapshot($s.db, $s.backup_dir, "delete-property").await?,
                    "properties_change_kind" => crate::commands::snapshot($s.db, $s.backup_dir, "change-kind").await?,
                    "issues_bulk_update" if large_bulk => crate::commands::snapshot($s.db, $s.backup_dir, "bulk-update").await?,
                    _ => {}
                }
                let out: Result<$r> = (async move $body).await;
                serde_json::to_value(out?).map_err(|e| Error::Invalid(e.to_string()))
            }),
        }),*];
    };
}

taskapp_core::with_commands!(define_api, ChangeSource::Cli);

pub fn names() -> Value {
    json!(OPERATIONS.iter().map(|op| op.name).collect::<Vec<_>>())
}

pub fn describe(name: &str) -> Result<Value> {
    let op = operation(name)?;
    Ok(json!({
        "name": op.name,
        "arguments": op.args.iter().map(|(name, ty)| json!({"name": name, "type": ty})).collect::<Vec<_>>(),
        "result": op.result,
    }))
}

fn operation(name: &str) -> Result<&'static Operation> {
    OPERATIONS
        .iter()
        .find(|op| op.name == name)
        .ok_or_else(|| Error::NotFound(format!("command {name}")))
}

pub async fn call(db: &Db, backup_dir: &Path, name: &str, args: Value) -> Result<Value> {
    let op = operation(name)?;
    if !args.is_object() {
        return Err(Error::Invalid("arguments must be a JSON object".into()));
    }
    let state = State {
        db,
        backup_dir,
        tz: std::env::var("TZ")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "UTC".into()),
    };
    (op.handler)(&state, args).await
}

pub fn guide() -> Value {
    json!({
        "start": "Use taskapp --help and taskapp <command> --help to explore common commands.",
        "api": "Use taskapp api list, taskapp api describe <name>, and taskapp api call <name> --args '<JSON object>' for all core-backed app operations.",
        "output": "Use --json for structured output and errors; pass --db PATH for a different database.",
        "examples": ["taskapp issue list --project Cairn --not-status Done,Someday", "taskapp issue create 'A task'", "taskapp --json api describe labels_update"]
    })
}
