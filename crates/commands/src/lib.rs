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
    Applied, AppliedTemplate, Attachment, AttachmentData, AttachmentId, Bar, BulkPatch, Change,
    ChangeSource, Comment, CommentId, Cycle, CycleId, Db, Error, Event, EventId, EventInput,
    FolderSection, ImportPlan, Issue, IssueId, IssuePatch, IssueStats, IssueTemplate, Label,
    LabelId, Measure, Milestone, MilestoneId, NewIssue, ParsedQuickAdd, Pin, PinId, PinKind,
    Placement, ProgressScope, ProgressSeries, Project, ProjectId, ProjectRow, ProjectStatus,
    ProjectStatusCategory, ProjectStatusId, PropertyAppliesTo, PropertyDef, PropertyKind, Relation,
    RelationKind, RelationKindDef, Reschedule, ResolvedPin, Result, SavedView, SelectOption,
    Status, StatusCategory, StatusId, SyncState, SyncStatus, TemplateId, TemplateInput, UndoStatus,
    VelocityPeriod, ViewDef, ViewEntity, ViewFolder, ViewFolderId, ViewId, ViewTemplate,
    ViewTemplateId, WorkBlock, WorkBlockId, WorkspaceStatus, attachments, backup, blocks, bulk,
    comments, cycles, events, history, import, insights, issues, labels, pins, project_statuses,
    projects, properties, quickadd, relation_kinds, relations, schedule, search, settings,
    statuses, templates, undo, view_folders, view_templates, views, workspace,
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

pub use taskapp_core::with_commands;

with_commands!(define);
