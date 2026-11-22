//! Every type shared across crates. Ids and enums derive `Copy`/`Eq`/`Hash` where cheap, for use as map keys.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

macro_rules! ids {
    ($($name:ident),*) => {$(
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
        pub struct $name(pub i64);
    )*};
}
ids!(
    IssueId,
    StatusId,
    ProjectId,
    MilestoneId,
    CycleId,
    LabelId,
    ViewId,
    CommentId,
    WorkBlockId,
    PinId,
    TemplateId
);

/// A patch field that can also be cleared: absent is `None`, set to null is `Some(None)`.
pub type Clearable<T> = Option<Option<T>>;

fn double_option<'de, D, T>(d: D) -> Result<Clearable<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub enum Due {
    Date(NaiveDate),
    /// `tz` is an IANA name.
    DateTime {
        utc: DateTime<Utc>,
        tz: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum StatusCategory {
    Backlog,
    Unstarted,
    Started,
    Completed,
    Canceled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum ChangeSource {
    Ui,
    Agent,
    Cli,
    RemindersImport,
    Migration,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum PropertyKind {
    Text,
    Number,
    Date,
    Select,
    MultiSelect,
    Checkbox,
    Url,
    NoteLink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum RelationKind {
    Blocks,
    Related,
    Duplicate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Issue {
    pub id: IssueId,
    pub key: String,
    pub title: String,
    pub description: String,
    pub status_id: StatusId,
    pub priority: u8,
    pub project_id: Option<ProjectId>,
    pub milestone_id: Option<MilestoneId>,
    pub parent_id: Option<IssueId>,
    pub cycle_id: Option<CycleId>,
    pub label_ids: Vec<LabelId>,
    pub start_date: Option<NaiveDate>,
    pub due: Option<Due>,
    pub estimate_minutes: Option<u32>,
    pub properties: Map<String, Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub archived_at: Option<DateTime<Utc>>,
}

/// Everything but `title` is optional; `created_at` and `completed_at` exist for migrations.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct NewIssue {
    pub title: String,
    pub description: Option<String>,
    pub status_id: Option<StatusId>,
    pub priority: Option<u8>,
    pub project_id: Option<ProjectId>,
    pub milestone_id: Option<MilestoneId>,
    pub parent_id: Option<IssueId>,
    pub cycle_id: Option<CycleId>,
    pub label_ids: Option<Vec<LabelId>>,
    pub start_date: Option<NaiveDate>,
    pub due: Option<Due>,
    pub estimate_minutes: Option<u32>,
    pub properties: Option<Map<String, Value>>,
    pub created_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// `None` leaves a field alone; for nullable fields `Some(None)` clears it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct IssuePatch {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status_id: Option<StatusId>,
    pub priority: Option<u8>,
    #[serde(
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[specta(type = Option<ProjectId>)]
    pub project_id: Clearable<ProjectId>,
    #[serde(
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[specta(type = Option<MilestoneId>)]
    pub milestone_id: Clearable<MilestoneId>,
    #[serde(
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[specta(type = Option<IssueId>)]
    pub parent_id: Clearable<IssueId>,
    #[serde(
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[specta(type = Option<CycleId>)]
    pub cycle_id: Clearable<CycleId>,
    pub label_ids: Option<Vec<LabelId>>,
    #[serde(
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[specta(type = Option<NaiveDate>)]
    pub start_date: Clearable<NaiveDate>,
    #[serde(
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[specta(type = Option<Due>)]
    pub due: Clearable<Due>,
    #[serde(
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[specta(type = Option<u32>)]
    pub estimate_minutes: Clearable<u32>,
    pub properties: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Status {
    pub id: StatusId,
    pub name: String,
    pub category: StatusCategory,
    pub position: i32,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Project {
    pub id: ProjectId,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub doc_link: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub target_date: Option<NaiveDate>,
    pub archived_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Milestone {
    pub id: MilestoneId,
    pub project_id: ProjectId,
    pub name: String,
    pub target_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Cycle {
    pub id: CycleId,
    pub number: u32,
    pub name: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Label {
    pub id: LabelId,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Comment {
    pub id: CommentId,
    pub issue_id: IssueId,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Relation {
    pub from: IssueId,
    pub to: IssueId,
    pub kind: RelationKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct SelectOption {
    pub value: String,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct PropertyDef {
    pub key: String,
    pub name: String,
    pub kind: PropertyKind,
    pub options: Vec<SelectOption>,
    pub project_id: Option<ProjectId>,
    pub position: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum PinKind {
    Issue,
    Project,
    View,
    Cycle,
    Note,
    Url,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Pin {
    pub id: PinId,
    pub kind: PinKind,
    pub target: String,
    pub title: Option<String>,
    pub position: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct ResolvedPin {
    pub pin: Pin,
    pub label: String,
    pub archived: bool,
    /// A UI route, or the external URL.
    pub route: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct WorkBlock {
    pub id: WorkBlockId,
    pub issue_id: IssueId,
    pub start_utc: DateTime<Utc>,
    pub end_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum FixedField {
    Title,
    Status,
    StatusCategory,
    Priority,
    Project,
    Milestone,
    Cycle,
    Label,
    Parent,
    StartDate,
    DueDate,
    Estimate,
    CreatedAt,
    UpdatedAt,
    CompletedAt,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum FieldRef {
    Fixed(FixedField),
    Property(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum Op {
    Eq,
    Neq,
    In,
    NotIn,
    Lt,
    Lte,
    Gt,
    Gte,
    Contains,
    IsEmpty,
    IsNotEmpty,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub enum Filter {
    And(Vec<Filter>),
    Or(Vec<Filter>),
    Not(Box<Filter>),
    Cond {
        field: FieldRef,
        op: Op,
        value: Value,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Sort {
    pub field: FieldRef,
    pub descending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum Layout {
    List,
    Table,
    Board,
    Gantt,
    Calendar,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct ViewDef {
    pub filter: Option<Filter>,
    pub sorts: Vec<Sort>,
    pub group_by: Option<FieldRef>,
    pub layout: Layout,
    pub columns: Vec<FieldRef>,
    pub include_archived: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct SavedView {
    pub id: ViewId,
    pub name: String,
    pub def: ViewDef,
    pub position: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Bar {
    pub issue_id: IssueId,
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub from_estimate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Reschedule {
    pub issue_id: IssueId,
    pub start_date: NaiveDate,
    pub due_date: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct ParsedQuickAdd {
    pub title: String,
    pub due: Option<Due>,
    pub start_date: Option<NaiveDate>,
    pub priority: Option<u8>,
    pub project_slug: Option<String>,
    pub label_names: Vec<String>,
    pub estimate_minutes: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, specta::Type)]
pub enum SyncState {
    #[default]
    Off,
    Ok {
        at: DateTime<Utc>,
    },
    Error {
        at: DateTime<Utc>,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct SyncStatus {
    pub reminders: SyncState,
    pub gcal: SyncState,
    pub export: SyncState,
    /// Absent in statuses saved before backups reported here.
    #[serde(default)]
    pub backup: SyncState,
}

/// One row of change history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Change {
    pub id: i64,
    pub at: DateTime<Utc>,
    pub source: ChangeSource,
    pub entity: String,
    pub entity_id: String,
    pub field: String,
    pub old_value: Option<Value>,
    pub new_value: Option<Value>,
}
