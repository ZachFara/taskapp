//! Every type shared across crates. Ids and enums derive `Copy`/`Eq`/`Hash` where cheap, for use as map keys.

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
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
    ProjectStatusId,
    ProjectId,
    MilestoneId,
    CycleId,
    LabelId,
    LabelGroupId,
    ViewId,
    ViewTemplateId,
    ViewFolderId,
    CommentId,
    WorkBlockId,
    PinId,
    TemplateId,
    EventId,
    DecisionId,
    GoalId,
    LinkId,
    AttachmentId
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
    Undo,
    Redo,
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

/// The id of a row in `relation_kinds`: "blocks", "related", "duplicate", or any other kind's slug.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(transparent)]
pub struct RelationKind(pub String);

impl RelationKind {
    #[must_use]
    pub fn new(id: &str) -> Self {
        Self(id.to_owned())
    }
}

/// A kind of relation: `forward` reads from the `from` issue ("blocks"), `reverse` from the `to` issue ("blocked by").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct RelationKindDef {
    pub id: RelationKind,
    pub name: String,
    pub forward: String,
    pub reverse: String,
    pub symmetric: bool,
    pub built_in: bool,
    pub position: i32,
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
    /// At least 0, one decimal at most (checked on write).
    pub points: Option<f64>,
    /// Deferred: views hide the issue until this day (the Process Inbox "Defer" choice).
    pub hidden_until: Option<NaiveDate>,
    /// Deep or shallow work; `None` until chosen.
    pub depth: Option<Depth>,
    /// Canonical repeat rule (`every 1 week on mon from due`); completing the issue creates the next one.
    pub repeat: Option<String>,
    /// Who the issue waits on; with `follow_up` it puts the issue on the Waiting page.
    pub waiting_on: Option<String>,
    /// The day to check back on the issue; once it has come, the issue is a due follow-up.
    pub follow_up: Option<NaiveDate>,
    pub properties: Map<String, Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub archived_at: Option<DateTime<Utc>>,
}

/// Everything but `title` is optional; `created_at` and `completed_at` exist for migrations.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(default, deny_unknown_fields)]
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
    pub points: Option<f64>,
    /// A repeat rule in any accepted form; stored canonical.
    pub repeat: Option<String>,
    pub properties: Option<Map<String, Value>>,
    pub created_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// `None` leaves a field alone; for nullable fields `Some(None)` clears it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(default, deny_unknown_fields)]
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
    #[specta(type = Option<Depth>)]
    pub depth: Clearable<Depth>,
    #[serde(
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[specta(type = Option<String>)]
    pub repeat: Clearable<String>,
    #[serde(
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[specta(type = Option<String>)]
    pub waiting_on: Clearable<String>,
    #[serde(
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[specta(type = Option<NaiveDate>)]
    pub follow_up: Clearable<NaiveDate>,
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
    #[serde(
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[specta(type = Option<f64>)]
    pub points: Clearable<f64>,
    #[serde(
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[specta(type = Option<NaiveDate>)]
    pub hidden_until: Clearable<NaiveDate>,
    pub properties: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Status {
    pub id: StatusId,
    pub name: String,
    pub category: StatusCategory,
    pub position: i32,
    /// A palette key.
    pub color: String,
    /// The reference hex for `color`, for showing it without the palette.
    pub color_hex: String,
    pub icon: Option<String>,
    /// What the status means, in a sentence or two.
    pub definition: String,
    /// How time in this status counts for flow efficiency.
    pub flow_kind: FlowKind,
    /// Issues sitting here this many days are stale (for nudges); none means never.
    pub nudge_days: Option<i32>,
}

/// Whether time in a status is work (`Active`), blocked on someone or something (`Waiting`) or neither (`Idle`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum FlowKind {
    Active,
    Waiting,
    Idle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum ProjectStatusCategory {
    Planned,
    Started,
    Paused,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct ProjectStatus {
    pub id: ProjectStatusId,
    pub name: String,
    pub category: ProjectStatusCategory,
    pub position: i32,
    /// A palette key.
    pub color: String,
    /// The reference hex for `color`, for showing it without the palette.
    pub color_hex: String,
    pub icon: Option<String>,
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
    /// Set by the database; an update ignores what the caller sends.
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub status_id: ProjectStatusId,
    /// The issue scale: 0 none, 1 urgent, 2 high, 3 medium, 4 low.
    pub priority: u8,
    pub label_ids: Vec<LabelId>,
    pub properties: Map<String, Value>,
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
    pub icon: Option<String>,
    pub color: Option<String>,
    /// The retrospective: a one-sentence goal, whether it was met (`None` until decided), a 1 to 5 rating, notes.
    #[serde(default)]
    pub goal: Option<String>,
    #[serde(default)]
    pub goal_met: Option<bool>,
    #[serde(default)]
    pub rating: Option<u8>,
    #[serde(default)]
    pub notes: String,
}

/// Times are local wall-clock (no zone); an all-day event keeps midnight and the UI ignores the time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Event {
    pub id: EventId,
    pub title: String,
    pub starts_at: NaiveDateTime,
    pub ends_at: Option<NaiveDateTime>,
    pub all_day: bool,
    pub place: Option<String>,
    pub notes: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub archived: bool,
    pub issue_ids: Vec<IssueId>,
}

/// What create and update take: everything on [`Event`] except the id and the links.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct EventInput {
    pub title: String,
    pub starts_at: NaiveDateTime,
    pub ends_at: Option<NaiveDateTime>,
    pub all_day: bool,
    pub place: Option<String>,
    pub notes: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Label {
    pub id: LabelId,
    pub name: String,
    /// A palette key.
    pub color: String,
    /// The reference hex for `color`, for showing it without the palette.
    pub color_hex: String,
    pub icon: Option<String>,
    /// The label group this label belongs to, if any.
    pub group_id: Option<LabelGroupId>,
}

/// A named set of labels; in an exclusive group an issue holds at most one of them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct LabelGroup {
    pub id: LabelGroupId,
    pub name: String,
    pub exclusive: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Comment {
    pub id: CommentId,
    pub issue_id: IssueId,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A stored image; its bytes come from `attachments::get`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Attachment {
    pub id: AttachmentId,
    pub sha256: String,
    pub mime: String,
    pub bytes: i64,
    pub created_at: DateTime<Utc>,
}

/// An image's bytes and type, as `attachments_get` returns them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct AttachmentData {
    pub mime: String,
    pub bytes: Vec<u8>,
}

/// One edit for a whole selection: the patch, labels to add or remove, and a relation from each issue to `add_relation.1`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct BulkPatch {
    pub patch: IssuePatch,
    pub add_labels: Vec<LabelId>,
    pub remove_labels: Vec<LabelId>,
    pub add_relation: Option<(RelationKind, IssueId)>,
    /// `Some(true)` archives, `Some(false)` restores.
    pub archive: Option<bool>,
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

/// Which entities a custom property applies to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum PropertyAppliesTo {
    #[default]
    Issues,
    Projects,
    Both,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct PropertyDef {
    pub key: String,
    pub name: String,
    pub kind: PropertyKind,
    pub options: Vec<SelectOption>,
    /// Limits the issue side to one project; never set for a projects-only property.
    pub project_id: Option<ProjectId>,
    pub position: i32,
    #[serde(default)]
    pub applies_to: PropertyAppliesTo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum PinKind {
    Issue,
    Project,
    View,
    Cycle,
    Note,
    Url,
    Folder,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum Depth {
    Deep,
    Shallow,
}

impl Depth {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Deep => "deep",
            Self::Shallow => "shallow",
        }
    }
}

/// A timer on one issue. `ended_at` is `None` while it runs; at most one session does.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct FocusSession {
    pub id: i64,
    pub issue_id: IssueId,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub deep: bool,
}

/// Deep focus hours per bucket of the range, the total, and the run of days (ending today) with some.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct DeepHours {
    pub buckets: Vec<DeepBucket>,
    pub total_hours: f64,
    pub streak_days: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct DeepBucket {
    pub start: NaiveDate,
    pub hours: f64,
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
    Points,
    /// Issues only: the day a deferred issue comes back.
    HiddenUntil,
    Depth,
    /// Issues only: the repeat rule, empty when the issue does not repeat.
    Repeat,
    /// Issues only: who the issue waits on.
    WaitingOn,
    /// Issues only: the day to check back on a waiting issue.
    FollowUp,
    CreatedAt,
    UpdatedAt,
    CompletedAt,
    Archived,
    Key,
    SubIssueProgress,
    RelationCount,
    CommentCount,
    /// Shows `done/total` of the description's task lines; not sortable or filterable.
    Checklist,
    /// Filter only: the issue has a relation of the given kind id(s), on either end.
    RelationKind,
    /// Filter only: the issue has a label in the given label group id(s).
    LabelGroup,
    /// An issue's project status; on a project view, use `Status`.
    ProjectStatus,
    /// Project views only.
    TargetDate,
    /// Project views only: the percent of the project's issues that are completed.
    Progress,
    /// Project views only: the project's issues, archived ones left out.
    IssueCount,
    /// Project views only: shows progress as a percent with a scope and completed sparkline; sorts and filters as `Progress`.
    ProgressChart,
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

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type,
)]
pub enum Layout {
    #[default]
    List,
    Table,
    Board,
    Gantt,
    Calendar,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum Density {
    #[default]
    Comfortable,
    Compact,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum Wrap {
    #[default]
    Off,
    On,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum Deferred {
    #[default]
    Hide,
    Show,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum SubIssues {
    #[default]
    Nest,
    Flat,
    Hide,
}

const fn yes() -> bool {
    true
}

/// `columns` is the ordered list of displayed fields in every layout; empty means the layout's default.
/// The display settings default so views stored before they existed still load.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct ViewDef {
    pub filter: Option<Filter>,
    pub sorts: Vec<Sort>,
    pub group_by: Option<FieldRef>,
    pub layout: Layout,
    pub columns: Vec<FieldRef>,
    pub include_archived: bool,
    /// Whether issues deferred to a later day (`hidden_until`) are listed.
    #[serde(default)]
    pub deferred: Deferred,
    #[serde(default)]
    pub density: Density,
    #[serde(default)]
    pub show_empty_groups: bool,
    #[serde(default)]
    pub show_sub_issues: SubIssues,
    /// The board's card fields; empty means `columns`.
    #[serde(default)]
    pub card_fields: Vec<FieldRef>,
    #[serde(default = "yes")]
    pub show_weekends: bool,
    /// Long titles, labels and values wrap onto more lines instead of being cut off.
    #[serde(default)]
    pub wrap: Wrap,
}

impl Default for ViewDef {
    fn default() -> Self {
        Self {
            filter: None,
            sorts: vec![],
            group_by: None,
            layout: Layout::default(),
            columns: vec![],
            include_archived: false,
            deferred: Deferred::default(),
            density: Density::default(),
            show_empty_groups: false,
            show_sub_issues: SubIssues::default(),
            card_fields: vec![],
            show_weekends: true,
            wrap: Wrap::default(),
        }
    }
}

/// What a saved view lists. A project view's fields are project fields.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum ViewEntity {
    #[default]
    Issue,
    Project,
}

/// A project as a project view lists it, with its issue counts (archived issues left out).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct ProjectRow {
    pub project: Project,
    pub issue_total: u32,
    pub issue_done: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct SavedView {
    pub id: ViewId,
    pub name: String,
    pub entity: ViewEntity,
    pub def: ViewDef,
    pub position: i32,
    pub icon: Option<String>,
    pub color: Option<String>,
    /// The template the view was made from, for "Reset to template".
    pub template_id: Option<ViewTemplateId>,
    /// The sidebar folder the view sits in; `None` is the top level.
    pub folder_id: Option<ViewFolderId>,
}

/// The sidebar section a folder belongs to; its items live in the table of the same name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum FolderSection {
    Pin,
    View,
    Project,
    Cycle,
}

/// A sidebar folder. It holds items of its own section and folders of that section, nested by `parent_id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct ViewFolder {
    pub id: ViewFolderId,
    pub name: String,
    pub position: i32,
    pub section: FolderSection,
    pub parent_id: Option<ViewFolderId>,
}

/// Where one pin, view, project or cycle sits: the folder (`None` is the section's top level) and its place there.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Placement {
    pub section: FolderSection,
    pub id: i64,
    pub folder_id: Option<ViewFolderId>,
    pub position: i32,
}

/// A saved view definition that new views start from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct ViewTemplate {
    pub id: ViewTemplateId,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub def: ViewDef,
    pub position: i32,
}

/// The counts a view can display that an issue row does not carry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct IssueStats {
    pub issue_id: IssueId,
    pub sub_total: u32,
    pub sub_done: u32,
    pub relations: u32,
    pub comments: u32,
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
    pub points: Option<f64>,
    /// A canonical repeat rule from an `every <rule>` phrase.
    pub repeat: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, specta::Type)]
pub enum SyncState {
    #[default]
    Off,
    /// The step's feature is switched off in Settings > Features, so it did not run.
    Skipped,
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

/// What undo and redo would do next: the label of the step, or `None` when the stack is empty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct UndoStatus {
    pub undo: Option<String>,
    pub redo: Option<String>,
}

/// What a progress chart is about: the issues of a project or cycle, or those a saved view shows now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum ProgressScope {
    Project(ProjectId),
    Cycle(CycleId),
    View(ViewId),
}

/// What is summed: story points, issue count, or estimated time (minutes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum Measure {
    Points,
    Count,
    Time,
}

/// The totals at the end of one local day.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct ProgressDay {
    pub date: NaiveDate,
    /// Every issue that existed and is not canceled.
    pub scope: f64,
    /// Those whose status category is started or completed.
    pub started: f64,
    pub completed: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct ProgressSeries {
    pub days: Vec<ProgressDay>,
    pub measure: Measure,
    pub from: NaiveDate,
    pub to: NaiveDate,
}

/// Completed in one cycle or week.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct VelocityPeriod {
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub label: String,
    pub completed: f64,
}

/// What the metrics cover: a run of days, or one cycle's own dates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum MetricsScope {
    Range { from: NaiveDate, to: NaiveDate },
    Cycle(CycleId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum Bucket {
    Day,
    Week,
    Month,
}

/// `measure` is `count` or `points` (time is refused); it picks what the label breakdown ranks by.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct MetricsQuery {
    pub scope: MetricsScope,
    pub bucket: Bucket,
    pub filter: Option<Filter>,
    pub measure: Measure,
}

/// What to forecast: the issues the filter lets through, or one cycle's own members.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct ForecastQuery {
    pub filter: Option<Filter>,
    pub cycle: Option<CycleId>,
}

/// When the open issues are likely done at the recent pace. `p50` and `p85` are the days by which half and
/// 85 in 100 simulated futures are finished; both are `None` when `enough` is false (too little history) or
/// nothing is open. `completions` is what finished in the `window_days` before today.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Forecast {
    pub open: u32,
    pub completions: u32,
    pub window_days: u32,
    pub enough: bool,
    pub p50: Option<NaiveDate>,
    pub p85: Option<NaiveDate>,
}

/// One bucket; `scope` is open plus finished-in-range work at the bucket's end, `done_total` the finished part.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct MetricsBucket {
    pub start: NaiveDate,
    pub created: u32,
    pub completed: u32,
    pub completed_points: f64,
    pub scope: u32,
    pub done_total: u32,
    pub wip: u32,
}

/// Calendar days in the workspace zone, nearest-rank percentiles; `None` without a finished issue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct LeadTime {
    pub p50_days: Option<f64>,
    pub p85_days: Option<f64>,
    pub n: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct CycleTimePoint {
    pub issue_id: IssueId,
    pub key: String,
    pub title: String,
    pub completed: NaiveDate,
    pub days: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct CycleTime {
    pub p50_days: Option<f64>,
    pub p85_days: Option<f64>,
    pub n: u32,
    pub points: Vec<CycleTimePoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct LabelBreakdown {
    pub label_id: Option<i64>,
    pub name: String,
    pub completed: u32,
    pub points: f64,
}

/// `source` is a [`ChangeSource`] name: Ui, Cli, Agent, Migration and so on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct SourceBreakdown {
    pub source: String,
    pub created: u32,
    pub completed: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct VelocityCycle {
    pub cycle_id: CycleId,
    pub name: String,
    pub points: f64,
    pub count: u32,
}

/// An issue past its status's nudge age: where it sits, since when, and for how many whole days.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct StaleIssue {
    pub issue: Issue,
    pub status_name: String,
    pub nudge_days: i32,
    pub since: DateTime<Utc>,
    pub days: i32,
}

/// A project with its open next action; `stuck` when the project is active and has none.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct NextAction {
    pub project: Project,
    pub issue: Option<Issue>,
    pub stuck: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Metrics {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub bucket: Bucket,
    pub buckets: Vec<MetricsBucket>,
    pub lead_time: LeadTime,
    pub cycle_time: CycleTime,
    pub wip_now: u32,
    pub by_label: Vec<LabelBreakdown>,
    pub by_source: Vec<SourceBreakdown>,
    /// The last six finished cycles.
    pub velocity: Vec<VelocityCycle>,
    /// Fewer than five issues finished in the range: read the numbers as anecdotes.
    pub thin: bool,
    /// Present for a cycle scope only.
    #[serde(default)]
    pub cycle: Option<CycleCommit>,
    #[serde(default)]
    pub blocked: Option<Blocked>,
    #[serde(default)]
    pub flow: Option<FlowEfficiency>,
    #[serde(default)]
    pub deep: Option<DeepHours>,
}

/// A cycle's commitment: its members at the end of its start day against what it holds and finished by its end
/// (or today, for a cycle still running). Percentages are `None` when nothing was committed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct CycleCommit {
    pub committed: u32,
    pub committed_points: f64,
    /// Committed issues finished.
    pub done: u32,
    pub done_points: f64,
    pub say_do_pct: Option<f64>,
    pub say_do_points_pct: Option<f64>,
    /// Issues that joined or left the cycle after its start day.
    pub added: u32,
    pub added_points: f64,
    pub removed: u32,
    pub removed_points: f64,
    /// (added + removed) as a percentage of what was committed.
    pub scope_change_pct: Option<f64>,
    pub scope_change_points_pct: Option<f64>,
}

/// An open issue with an unfinished `blocks` blocker; `days` since the newest such link began.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct BlockedItem {
    pub issue_id: IssueId,
    pub key: String,
    pub title: String,
    pub blocked_by: Vec<String>,
    pub days: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct BlockedTime {
    pub issue_id: IssueId,
    pub key: String,
    pub title: String,
    pub days: f64,
}

/// `time` is approximate: whole days, from creation to finish, whose end found an unfinished blocker linked.
/// `avg_days` spreads it over every issue finished in the range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Blocked {
    pub now: Vec<BlockedItem>,
    pub time: Vec<BlockedTime>,
    pub avg_days: Option<f64>,
}

/// Days the finished issues spent in one Waiting status between starting and finishing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct WaitingStatus {
    pub status_id: StatusId,
    pub name: String,
    pub days: f64,
    pub issues: u32,
}

/// Active-kind days over cycle-time days, per finished issue that has a cycle time above zero (`n` of them).
/// `distribution` counts them in five 20% bins, lowest first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct FlowEfficiency {
    pub avg_pct: Option<f64>,
    pub n: u32,
    pub distribution: Vec<u32>,
    pub waiting: Vec<WaitingStatus>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum LinkKind {
    Solution,
    Reference,
    Pr,
    Commit,
    Doc,
    Other,
}

/// Where something about an issue lives: a URL or a `taskapp://` link, with a title and what it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Link {
    pub id: LinkId,
    pub issue_id: IssueId,
    pub url: String,
    pub title: String,
    pub kind: LinkKind,
    pub created_at: String,
}

/// One choice of a [`Decision`]; `key` is what an answer and the recommendation name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DecisionOption {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub tradeoff: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum DecisionMode {
    /// Whoever raised it waits for the answer.
    Blocking,
    /// Whoever raised it carries on with its recommendation.
    Assume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum DecisionStatus {
    Open,
    Answered,
    Withdrawn,
}

/// A question for the owner. `answer` is an option key, or free text when the options did not fit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Decision {
    pub id: DecisionId,
    pub question: String,
    pub context: String,
    pub options: Vec<DecisionOption>,
    pub recommendation: Option<String>,
    pub mode: DecisionMode,
    pub status: DecisionStatus,
    pub answer: Option<String>,
    pub note: String,
    pub source: String,
    pub external_id: Option<String>,
    pub created_at: String,
    pub answered_at: Option<String>,
    pub issue_ids: Vec<IssueId>,
}

/// What create takes. `source` defaults to the caller's own (`cli`, `ui`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DecisionInput {
    pub question: String,
    #[serde(default)]
    pub context: String,
    #[serde(default)]
    pub options: Vec<DecisionOption>,
    pub recommendation: Option<String>,
    pub mode: DecisionMode,
    pub source: Option<String>,
    pub external_id: Option<String>,
    #[serde(default)]
    pub issue_ids: Vec<IssueId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum GoalStatus {
    Active,
    Achieved,
    Dropped,
}

/// A goal for a quarter (`quarter` is like `2027-Q1`) with the projects that work towards it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Goal {
    pub id: GoalId,
    pub title: String,
    pub quarter: String,
    pub description: String,
    pub status: GoalStatus,
    pub created_at: String,
    pub project_ids: Vec<ProjectId>,
}

/// One Process Inbox choice for one issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum InboxChoice {
    /// The two-minute rule: status Done.
    Done,
    /// Status Delegated, and a comment "Waiting on: <note>".
    Delegate {
        note: String,
    },
    /// Hide the issue until a later day.
    Defer {
        until: NaiveDate,
    },
    /// Status Next, in `project` when given; `as_next_action` also flags it as the project's next action.
    Next {
        project: Option<ProjectId>,
        #[serde(default)]
        as_next_action: bool,
    },
    Someday,
    /// Archive.
    Trash,
}
