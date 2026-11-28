//! One function per subcommand group: validates arguments, calls core, returns the output value.

use std::path::Path;
use std::str::FromStr;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::Serialize;
use serde_json::{Map, Value, json};
use taskapp_core::{
    AttachmentId, ChangeSource, CycleId, Db, Due, Error, EventId, EventInput, FlowKind, IssueId,
    IssuePatch, Measure, NewIssue, PinId, PinKind, ProgressScope, ProjectId, PropertyDef,
    PropertyKind, Relation, RelationKind, Result, SelectOption, TemplateId, ViewDef, ViewId,
    ViewTemplateId, attachments, backup, comments, cycles, events, history, import, insights,
    issues, labels, pins, project_statuses, projects, properties, quickadd, relations, search,
    statuses, templates, undo, view_templates, views,
};

use crate::{
    AttachmentCmd, Cmd, CommentCmd, CycleCmd, EventCmd, ImportCmd, InsightsCmd, IssueCmd, LabelCmd,
    PinCmd, ProjectCmd, ProjectEdit, PropertyCmd, RelationCmd, ScopeArgs, StatusCmd, SyncCmd,
    TemplateCmd, ViewCmd, look,
};

const SRC: ChangeSource = ChangeSource::Cli;

pub async fn run(db: &Db, dir: &Path, cmd: Cmd) -> Result<Value> {
    match cmd {
        Cmd::Api(crate::ApiCmd::Call { name, args }) => {
            let args = serde_json::from_str(&args)
                .map_err(|e| Error::Invalid(format!("arguments must be a JSON object: {e}")))?;
            crate::api::call(db, dir, &name, args).await
        }
        Cmd::Undo => val(undo::undo(db).await?),
        Cmd::Setup { name, key, tz } => {
            val(taskapp_core::workspace::setup(db, &name, &key, tz.as_deref()).await?)
        }
        Cmd::Redo => val(undo::redo(db).await?),
        Cmd::Template(cmd) => template(db, cmd).await,
        Cmd::Insights(cmd) => insights(db, cmd).await,
        Cmd::Event(cmd) => event(db, cmd).await,
        Cmd::Status(cmd) => status(db, cmd).await,
        Cmd::Attachment(cmd) => attachment(db, cmd).await,
        Cmd::Sync(SyncCmd::Now { helper }) => sync_now(db, helper).await,
        Cmd::Decision(cmd) => crate::decision::run(db, cmd).await,
        Cmd::Icons => Ok(json!(look::ICONS)),
        Cmd::Colors => Ok(json!(
            taskapp_core::palette::COLORS
                .iter()
                .map(|(key, _)| *key)
                .collect::<Vec<_>>()
        )),
        cmd => run_group(db, dir, cmd).await,
    }
}

async fn run_group(db: &Db, dir: &Path, cmd: Cmd) -> Result<Value> {
    match cmd {
        Cmd::Add { text, tz } => add(db, &text, tz.as_deref()).await,
        Cmd::Issue(cmd) => issue(db, cmd).await,
        Cmd::View(cmd) => view(db, cmd).await,
        Cmd::Property(cmd) => property(db, dir, cmd).await,
        Cmd::Project(cmd) => project(db, cmd).await,
        Cmd::Cycle(cmd) => cycle(db, cmd).await,
        Cmd::Label(cmd) => label(db, cmd).await,
        Cmd::Comment(cmd) => comment(db, cmd).await,
        Cmd::Relation(cmd) => relation(db, cmd).await,
        Cmd::Pin(cmd) => pin(db, cmd).await,
        Cmd::Import(cmd) => import(db, cmd).await,
        Cmd::Backup(_) | Cmd::Restore { .. } | Cmd::Guide | Cmd::Api(_) => Err(Error::Invalid(
            "this command runs before the database opens".into(),
        )),
        Cmd::Undo
        | Cmd::Redo
        | Cmd::Setup { .. }
        | Cmd::Template(_)
        | Cmd::Insights(_)
        | Cmd::Metrics(_)
        | Cmd::Forecast(_)
        | Cmd::Status(_)
        | Cmd::Attachment(_)
        | Cmd::Event(_)
        | Cmd::Decision(_)
        | Cmd::Icons
        | Cmd::Colors
        | Cmd::Sync(_) => handled_in_run(),
        Cmd::History { key } => {
            let id = issue_id(db, &key).await?;
            val(history::list_for(db, "issue", &id.0.to_string()).await?)
        }
    }
}

/// Undo, redo and templates are answered in [`run`] before the group commands.
fn handled_in_run() -> Result<Value> {
    Err(Error::Invalid("handled by run".into()))
}

async fn add(db: &Db, text: &str, tz: Option<&str>) -> Result<Value> {
    if text.trim().is_empty() {
        return Err(Error::Invalid("quick-add text is empty".into()));
    }
    let zone = tz
        .map(str::to_owned)
        .or_else(|| std::env::var("TZ").ok().filter(|z| !z.is_empty()))
        .unwrap_or_else(|| "UTC".to_string());
    let parsed = chrono_tz::Tz::from_str(&zone)
        .map_err(|_| Error::Invalid(format!("unknown time zone: {zone}")))?;
    let today = Utc::now().with_timezone(&parsed).date_naive();
    val(quickadd::create_from(db, text, today, &zone, SRC).await?)
}

async fn issue(db: &Db, cmd: IssueCmd) -> Result<Value> {
    match cmd {
        IssueCmd::Create {
            title,
            description,
            priority,
            status,
            project,
            due,
            estimate,
            points,
            created_at,
            completed_at,
            properties,
            parent,
            cycle,
            labels,
            migration,
        } => {
            let parent_id = match parent {
                Some(key) => Some(issue_id(db, &key).await?),
                None => None,
            };
            let status_id = crate::references::status(db, status.as_deref()).await?;
            let project_id = crate::references::project(db, project.as_deref()).await?;
            let cycle_id = crate::references::cycle(db, cycle.as_deref()).await?;
            let label_ids = crate::references::labels(db, &labels).await?;
            let new = NewIssue {
                created_at: created_at.as_deref().map(timestamp).transpose()?,
                completed_at: completed_at.as_deref().map(timestamp).transpose()?,
                properties: property_map(&properties)?,
                parent_id,
                cycle_id,
                label_ids: (!labels.is_empty()).then_some(label_ids),
                title,
                description,
                priority: check_priority(priority)?,
                status_id,
                project_id,
                due: due.as_deref().map(date).transpose()?.map(Due::Date),
                estimate_minutes: estimate,
                points,
                ..NewIssue::default()
            };
            val(issues::create(db, new, source(migration)).await?)
        }
        update @ IssueCmd::Update { .. } => issue_update(db, update).await,
        IssueCmd::List(args) => crate::list::run(db, &args).await,
        IssueCmd::Get { key } => crate::list::get(db, &key).await,
        IssueCmd::Archive { key } => {
            let id = issue_id(db, &key).await?;
            issues::archive(db, id, SRC).await?;
            Ok(json!({"archived": key}))
        }
        IssueCmd::Search { text, limit } => val(search::search(db, &text, limit).await?),
    }
}

async fn issue_update(db: &Db, cmd: IssueCmd) -> Result<Value> {
    let IssueCmd::Update {
        key,
        title,
        description,
        priority,
        status,
        due,
        clear_due,
        points,
        clear_points,
        start,
        estimate,
        project,
    } = cmd
    else {
        unreachable!("issue_update is only called with Update");
    };
    let id = issue_id(db, &key).await?;
    let status_id = crate::references::status(db, status.as_deref()).await?;
    let project_id = crate::references::project(db, project.as_deref()).await?;
    let due = if clear_due {
        Some(None)
    } else {
        due.as_deref()
            .map(date)
            .transpose()?
            .map(|d| Some(Due::Date(d)))
    };
    let patch = IssuePatch {
        title,
        description,
        priority: check_priority(priority)?,
        status_id,
        due,
        points: if clear_points {
            Some(None)
        } else {
            points.map(Some)
        },
        start_date: start.as_deref().map(date).transpose()?.map(Some),
        estimate_minutes: estimate.map(Some),
        project_id: project_id.map(Some),
        ..IssuePatch::default()
    };
    val(issues::update(db, id, patch, SRC).await?)
}

fn scope(a: &ScopeArgs) -> Result<(ProgressScope, Measure)> {
    let scope = match (a.project, a.cycle, a.view) {
        (Some(p), None, None) => ProgressScope::Project(ProjectId(p)),
        (None, Some(c), None) => ProgressScope::Cycle(CycleId(c)),
        (None, None, Some(v)) => ProgressScope::View(ViewId(v)),
        _ => {
            return Err(Error::Invalid(
                "pass exactly one of --project, --cycle or --view".into(),
            ));
        }
    };
    let measure = serde_json::from_value(json!(a.measure)).map_err(|_| {
        Error::Invalid(format!(
            "measure {} must be points, count or time",
            a.measure
        ))
    })?;
    Ok((scope, measure))
}

async fn insights(db: &Db, cmd: InsightsCmd) -> Result<Value> {
    match cmd {
        InsightsCmd::Progress { scope: s, from, to } => {
            let (scope, measure) = scope(&s)?;
            let (from, to) = (
                from.as_deref().map(date).transpose()?,
                to.as_deref().map(date).transpose()?,
            );
            val(insights::progress(db, scope, measure, from, to).await?)
        }
        InsightsCmd::Velocity { scope: s, periods } => {
            let (scope, measure) = scope(&s)?;
            val(insights::velocity(db, scope, measure, periods).await?)
        }
    }
}

/// A category or other unit enum from its serialised name.
fn named<T: serde::de::DeserializeOwned>(text: &str) -> Result<T> {
    serde_json::from_value(json!(text)).map_err(|e| Error::Invalid(e.to_string()))
}

async fn status(db: &Db, cmd: StatusCmd) -> Result<Value> {
    match cmd {
        StatusCmd::List => val(statuses::list(db).await?),
        StatusCmd::Create {
            name,
            category,
            color,
        } => val(statuses::create(db, &name, named(&category)?, &color, None, SRC).await?),
        StatusCmd::Update {
            name,
            definition,
            kind,
            nudge_days,
        } => {
            let id = crate::references::status(db, Some(&name))
                .await?
                .ok_or_else(|| Error::Invalid("status name is empty".into()))?;
            let kind = kind
                .map(|k| match k.to_lowercase().as_str() {
                    "active" => Ok(FlowKind::Active),
                    "waiting" => Ok(FlowKind::Waiting),
                    "idle" => Ok(FlowKind::Idle),
                    _ => Err(Error::Invalid("kind is active, waiting or idle".into())),
                })
                .transpose()?;
            val(statuses::set_flow(db, id, definition, kind, nudge_days, SRC).await?)
        }
    }
}

/// Creates a saved view, or replaces the one with `id` keeping the icon and colour the user gave it.
async fn save_view(
    db: &Db,
    name: &str,
    def: &str,
    id: Option<i64>,
    entity: String,
) -> Result<Value> {
    let def: ViewDef = serde_json::from_str(def).map_err(|e| Error::Invalid(e.to_string()))?;
    let entity = serde_json::from_value(Value::String(entity))
        .map_err(|_| Error::Invalid("entity is `issue` or `project`".into()))?;
    let look = match id {
        Some(id) => views::list(db)
            .await?
            .into_iter()
            .find(|v| v.id == ViewId(id))
            .map(|v| taskapp_core::palette::Look {
                icon: v.icon,
                color: v.color,
            }),
        None => None,
    };
    let saved = views::save_as(
        db,
        id.map(ViewId),
        (name, entity),
        def,
        look.unwrap_or_default(),
        SRC,
    );
    val(saved.await?)
}

async fn view(db: &Db, cmd: ViewCmd) -> Result<Value> {
    let (def, id) = match cmd {
        ViewCmd::Query { def, id } => (def, id),
        ViewCmd::Templates => return val(view_templates::list(db).await?),
        ViewCmd::List => return val(views::list(db).await?),
        ViewCmd::Save {
            name,
            def,
            id,
            entity,
        } => return save_view(db, &name, &def, id, entity).await,
        ViewCmd::SaveAsTemplate { view_id, name } => {
            let t = view_templates::from_view(db, ViewId(view_id), name.as_deref(), SRC);
            return val(t.await?);
        }
        ViewCmd::FromTemplate { template_id, name } => {
            let id = ViewTemplateId(template_id);
            return val(view_templates::create_view(db, id, name.as_deref(), SRC).await?);
        }
        ViewCmd::Reset { view_id } => {
            return val(view_templates::reset_view(db, ViewId(view_id), SRC).await?);
        }
        ViewCmd::DeleteTemplate { id } => {
            view_templates::delete(db, ViewTemplateId(id), SRC).await?;
            return Ok(json!({ "deleted": true }));
        }
    };
    let def: ViewDef = match (def, id) {
        (Some(text), None) => serde_json::from_str(&text)
            .map_err(|e| Error::Invalid(format!("view definition: {e}")))?,
        (None, Some(id)) => {
            views::list(db)
                .await?
                .into_iter()
                .find(|v| v.id == ViewId(id))
                .ok_or_else(|| Error::NotFound(format!("view {id}")))?
                .def
        }
        _ => return Err(Error::Invalid("pass exactly one of --def or --id".into())),
    };
    val(views::query(db, &def).await?)
}

async fn property(db: &Db, dir: &Path, cmd: PropertyCmd) -> Result<Value> {
    match cmd {
        PropertyCmd::List => val(properties::list_defs(db).await?),
        PropertyCmd::Create {
            key,
            name,
            kind: kind_name,
            options,
            applies_to,
        } => {
            let def = PropertyDef {
                key,
                name,
                kind: property_kind(&kind_name)?,
                options: select_options(&options),
                project_id: None,
                position: 0,
                applies_to: applies_to_of(&applies_to)?,
            };
            val(properties::create_def(db, def, SRC).await?)
        }
        PropertyCmd::Rename { key, name } => {
            val(properties::rename_def(db, &key, &name, SRC).await?)
        }
        PropertyCmd::ChangeKind {
            key,
            kind: kind_name,
            options,
        } => {
            snapshot(db, dir, "change-kind").await?;
            let kind = property_kind(&kind_name)?;
            val(properties::change_kind(db, &key, kind, select_options(&options), SRC).await?)
        }
        PropertyCmd::Delete { key } => {
            snapshot(db, dir, "delete-property").await?;
            properties::delete_def(db, &key, SRC).await?;
            Ok(json!({"deleted": key}))
        }
        PropertyCmd::Set { issue, key, value } => {
            let id = issue_id(db, &issue).await?;
            let parsed = serde_json::from_str::<Value>(&value)
                .map_err(|e| Error::Invalid(format!("value is not JSON: {e}")))?;
            let value = if parsed.is_null() { None } else { Some(parsed) };
            val(properties::set_value(db, id, &key, value, SRC).await?)
        }
    }
}

fn applies_to_of(s: &str) -> Result<taskapp_core::PropertyAppliesTo> {
    use taskapp_core::PropertyAppliesTo as A;
    match s {
        "issues" => Ok(A::Issues),
        "projects" => Ok(A::Projects),
        "both" => Ok(A::Both),
        other => Err(Error::Invalid(format!(
            "applies-to {other}: use issues, projects or both"
        ))),
    }
}

/// The name, description, look and dates `project set` takes; each one is kept unless given.
fn set_details(p: &mut taskapp_core::Project, edit: &ProjectEdit) -> Result<()> {
    if let Some(name) = &edit.name {
        p.name.clone_from(name);
    }
    if let Some(description) = &edit.description {
        p.description = Some(description.clone());
    }
    if let Some(icon) = &edit.icon {
        p.icon = Some(look::icon(icon)?);
    }
    if let Some(color) = &edit.color {
        p.color = Some(look::color(color)?);
    }
    if let Some(start) = &edit.start {
        p.start_date = Some(date(start)?);
    }
    if let Some(target) = &edit.target {
        p.target_date = Some(date(target)?);
    }
    Ok(())
}

async fn project_set(db: &Db, slug: &str, edit: ProjectEdit) -> Result<Value> {
    let mut p = projects::list(db)
        .await?
        .into_iter()
        .find(|p| p.slug == slug)
        .ok_or_else(|| Error::NotFound(format!("project {slug}")))?;
    set_details(&mut p, &edit)?;
    let ProjectEdit {
        status,
        priority,
        labels: label_names,
        property,
        ..
    } = edit;
    if let Some(name) = status {
        let all = project_statuses::list(db).await?;
        let found = all.iter().find(|s| s.name.eq_ignore_ascii_case(&name));
        p.status_id = found
            .ok_or_else(|| Error::NotFound(format!("project status {name}")))?
            .id;
    }
    if let Some(n) = priority {
        p.priority = n;
    }
    if !label_names.is_empty() {
        let all = labels::list(db).await?;
        p.label_ids = label_names
            .iter()
            .map(|n| {
                all.iter()
                    .find(|l| l.name.eq_ignore_ascii_case(n))
                    .map(|l| l.id)
                    .ok_or_else(|| Error::NotFound(format!("label {n}")))
            })
            .collect::<Result<_>>()?;
    }
    for kv in property.chunks(2) {
        let value = serde_json::from_str::<Value>(&kv[1])
            .map_err(|e| Error::Invalid(format!("value is not JSON: {e}")))?;
        if value.is_null() {
            p.properties.remove(&kv[0]);
        } else {
            p.properties.insert(kv[0].clone(), value);
        }
    }
    val(projects::update(db, p, SRC).await?)
}

async fn project(db: &Db, cmd: ProjectCmd) -> Result<Value> {
    match cmd {
        ProjectCmd::List => val(projects::list(db).await?),
        ProjectCmd::Statuses => val(project_statuses::list(db).await?),
        ProjectCmd::StatusCreate {
            name,
            category,
            color,
        } => val(project_statuses::create(db, &name, named(&category)?, &color, None, SRC).await?),
        ProjectCmd::Set { slug, edit } => project_set(db, &slug, edit).await,
        ProjectCmd::Create {
            slug,
            name,
            icon,
            color,
        } => val(projects::create(db, &slug, &name, icon, color, SRC).await?),
    }
}

async fn cycle(db: &Db, cmd: CycleCmd) -> Result<Value> {
    match cmd {
        CycleCmd::List => val(cycles::list(db).await?),
        CycleCmd::Create {
            start,
            end,
            name,
            icon,
            color,
        } => {
            let (start, end) = (date(&start)?, date(&end)?);
            val(cycles::create(
                db,
                name,
                start,
                end,
                taskapp_core::palette::Look { icon, color },
                SRC,
            )
            .await?)
        }
    }
}

async fn event(db: &Db, cmd: EventCmd) -> Result<Value> {
    match cmd {
        EventCmd::List { from, to } => {
            let (from, to) = (from.as_deref().map(date), to.as_deref().map(date));
            val(events::list(db, from.transpose()?, to.transpose()?).await?)
        }
        EventCmd::Create {
            title,
            start,
            end,
            all_day,
            place,
            notes,
        } => {
            let input = EventInput {
                title,
                starts_at: local_time(&start)?,
                ends_at: end.as_deref().map(local_time).transpose()?,
                all_day,
                place,
                notes: notes.unwrap_or_default(),
                icon: None,
                color: None,
                archived: false,
            };
            val(events::create(db, input, SRC).await?)
        }
        EventCmd::Link { id, key } => {
            events::link(db, EventId(id), issue_id(db, &key).await?, SRC).await?;
            Ok(json!({ "ok": true }))
        }
    }
}

fn local_time(text: &str) -> Result<NaiveDateTime> {
    NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M")
        .or_else(|_| date(text).map(|d| d.and_time(chrono::NaiveTime::MIN)))
        .map_err(|_| Error::Invalid(format!("not a time (YYYY-MM-DDTHH:MM): {text}")))
}

async fn template(db: &Db, cmd: TemplateCmd) -> Result<Value> {
    Ok(match cmd {
        TemplateCmd::List => val(templates::list(db).await?)?,
        TemplateCmd::Create { t } => val(templates::create(db, t.input()?, SRC).await?)?,
        TemplateCmd::Update { id, t } => {
            val(templates::update(db, TemplateId(id), t.input()?, SRC).await?)?
        }
        TemplateCmd::Delete { id } => {
            templates::delete(db, TemplateId(id), SRC).await?;
            json!({ "deleted": id })
        }
        TemplateCmd::Apply { id, title } => {
            let overrides = NewIssue {
                title: title.unwrap_or_default(),
                ..NewIssue::default()
            };
            val(templates::apply(db, TemplateId(id), overrides, SRC).await?)?
        }
    })
}

async fn label(db: &Db, cmd: LabelCmd) -> Result<Value> {
    match cmd {
        LabelCmd::List => val(labels::list(db).await?),
        LabelCmd::Create { name, color, icon } => {
            val(labels::create(db, &name, &color, icon, SRC).await?)
        }
        LabelCmd::Add { key, label } => {
            let found = labels::list(db)
                .await?
                .into_iter()
                .find(|l| l.name.eq_ignore_ascii_case(&label));
            let found = found.ok_or_else(|| Error::NotFound(format!("label {label}")))?;
            labels::add_label(db, issue_id(db, &key).await?, found.id, SRC).await?;
            val(issues::get_by_key(db, &key).await?)
        }
    }
}

async fn relation(db: &Db, cmd: RelationCmd) -> Result<Value> {
    let (add, from, kind, to) = match cmd {
        RelationCmd::Add { from, kind, to } => (true, from, kind, to),
        RelationCmd::Remove { from, kind, to } => (false, from, kind, to),
    };
    let rel = Relation {
        from: issue_id(db, &from).await?,
        to: issue_id(db, &to).await?,
        kind: RelationKind::new(&kind),
    };
    if add {
        relations::add(db, rel, SRC).await?;
        Ok(json!({"added": {"from": from, "kind": kind, "to": to}}))
    } else {
        relations::remove(db, rel, SRC).await?;
        Ok(json!({"removed": {"from": from, "kind": kind, "to": to}}))
    }
}

async fn attachment(db: &Db, cmd: AttachmentCmd) -> Result<Value> {
    match cmd {
        AttachmentCmd::Path { id } => {
            let (_, path) = attachments::locate(db, AttachmentId(id)).await?;
            Ok(Value::String(path.display().to_string()))
        }
        AttachmentCmd::List { key } => {
            let issue = issues::get_by_key(db, &key).await?;
            let rows = attachments::in_text(db, &issue.description)
                .await?
                .into_iter()
                .map(|(a, path)| {
                    json!({"id": a.id.0, "path": path.display().to_string(), "mime": a.mime, "bytes": a.bytes})
                })
                .collect();
            Ok(Value::Array(rows))
        }
    }
}

async fn pin(db: &Db, cmd: PinCmd) -> Result<Value> {
    match cmd {
        PinCmd::List => val(pins::list_resolved(db).await?),
        PinCmd::Add {
            kind,
            target,
            title,
        } => val(pins::pin(db, pin_kind(&kind)?, &target, title).await?),
        PinCmd::Remove { id } => {
            pins::unpin(db, PinId(id)).await?;
            Ok(json!({"removed": id}))
        }
    }
}

/// Snapshot before a destructive property change, as the MCP server does.
pub(crate) async fn snapshot(db: &Db, dir: &Path, reason: &str) -> Result<()> {
    backup::snapshot(db, dir, reason).await?;
    Ok(())
}

async fn issue_id(db: &Db, key: &str) -> Result<IssueId> {
    Ok(issues::get_by_key(db, key).await?.id)
}

fn val<T: Serialize>(value: T) -> Result<Value> {
    serde_json::to_value(value).map_err(|e| Error::Invalid(e.to_string()))
}

fn date(text: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d")
        .map_err(|_| Error::Invalid(format!("not a date (YYYY-MM-DD): {text}")))
}

fn source(migration: bool) -> ChangeSource {
    if migration {
        ChangeSource::Migration
    } else {
        SRC
    }
}

fn timestamp(text: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(text)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|_| Error::Invalid(format!("not an RFC 3339 timestamp: {text}")))
}

/// `key=value` pairs; a value that is not JSON is taken as text.
fn property_map(pairs: &[String]) -> Result<Option<Map<String, Value>>> {
    let mut map = Map::new();
    for pair in pairs {
        let (key, raw) = pair
            .split_once('=')
            .ok_or_else(|| Error::Invalid(format!("property must be key=value: {pair}")))?;
        let value = serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_owned()));
        map.insert(key.to_owned(), value);
    }
    Ok((!map.is_empty()).then_some(map))
}

fn check_priority(priority: Option<u8>) -> Result<Option<u8>> {
    match priority {
        Some(p) if p > 4 => Err(Error::Invalid("priority must be 0 to 4".into())),
        other => Ok(other),
    }
}

fn property_kind(name: &str) -> Result<PropertyKind> {
    Ok(match name {
        "text" => PropertyKind::Text,
        "number" => PropertyKind::Number,
        "date" => PropertyKind::Date,
        "select" => PropertyKind::Select,
        "multiselect" => PropertyKind::MultiSelect,
        "checkbox" => PropertyKind::Checkbox,
        "url" => PropertyKind::Url,
        "notelink" => PropertyKind::NoteLink,
        _ => return Err(Error::Invalid(format!("unknown property kind: {name}"))),
    })
}

fn pin_kind(name: &str) -> Result<PinKind> {
    Ok(match name {
        "issue" => PinKind::Issue,
        "project" => PinKind::Project,
        "view" => PinKind::View,
        "cycle" => PinKind::Cycle,
        "note" => PinKind::Note,
        "url" => PinKind::Url,
        "folder" => PinKind::Folder,
        _ => return Err(Error::Invalid(format!("unknown pin kind: {name}"))),
    })
}

/// Options from `--option` values; every option gets the default color.
fn select_options(values: &[String]) -> Vec<SelectOption> {
    values
        .iter()
        .map(|value| SelectOption {
            value: value.clone(),
            color: "gray".to_string(),
        })
        .collect()
}

async fn import(db: &Db, cmd: ImportCmd) -> Result<Value> {
    let (plan, apply) = match cmd {
        ImportCmd::Csv { file, map, apply } => {
            let mut mapping = std::collections::BTreeMap::new();
            for pair in &map {
                let (col, field) = pair.split_once('=').ok_or_else(|| {
                    Error::Invalid(format!("--map {pair}: expected COLUMN=FIELD"))
                })?;
                mapping.insert(col.to_owned(), field.to_owned());
            }
            let text = std::fs::read_to_string(file)?;
            let mapping = (!mapping.is_empty()).then_some(mapping);
            (import::csv::plan(db, &text, mapping.as_ref()).await?, apply)
        }
        ImportCmd::Linear { apply } => {
            let api = import::linear::CurlLinear::new(import::linear::Token::from_env()?);
            (import::linear::plan(db, &api).await?, apply)
        }
    };
    if apply {
        let done = import::apply(db, &plan).await?;
        return Ok(json!({ "plan": plan, "applied": done }));
    }
    val(plan)
}

async fn comment(db: &Db, cmd: CommentCmd) -> Result<Value> {
    let CommentCmd::Add {
        key,
        body,
        created_at,
        migration,
    } = cmd;
    let id = issue_id(db, &key).await?;
    let at = created_at.as_deref().map(timestamp).transpose()?;
    val(comments::add_comment(db, id, &body, at, source(migration)).await?)
}

const APP_HELPER: &str = "/Applications/taskapp.app/Contents/MacOS/reminders-helper";

/// The app's own sync steps for Reminders and Google Calendar, once. The CLI cannot reach a running app,
/// so it does the work itself; both steps do nothing when their setting is off.
async fn sync_now(db: &Db, helper: Option<std::path::PathBuf>) -> Result<Value> {
    let helper = helper.unwrap_or_else(|| APP_HELPER.into());
    let tz: String = taskapp_core::settings::get(db, "timezone")
        .await?
        .unwrap_or_else(|| "UTC".into());
    let zone: chrono_tz::Tz = tz
        .parse()
        .map_err(|_| Error::Invalid(format!("unknown time zone {tz}")))?;
    let configured = taskapp_core::settings::get::<String>(db, "reminders_list_id")
        .await?
        .is_some_and(|l| !l.is_empty());
    if configured && !helper.exists() {
        return Err(Error::Invalid(format!(
            "reminders helper not found at {}; pass --helper <path>",
            helper.display()
        )));
    }
    let today = Utc::now().with_timezone(&zone).date_naive();
    let r = taskapp_reminders::import::import(db, &helper, today, &tz).await?;
    let g = taskapp_gcal::reconcile::reconcile(db).await?;
    Ok(json!({
        "reminders": {"imported": r.imported, "skipped_duplicates": r.skipped_duplicates, "failed": r.failed},
        "gcal": {"created": g.created, "updated": g.updated, "deleted": g.deleted},
    }))
}
