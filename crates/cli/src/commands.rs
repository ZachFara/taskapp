//! One function per subcommand group: validates arguments, calls core, returns the output value.

use std::path::Path;
use std::str::FromStr;

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use serde_json::{Map, Value, json};
use taskapp_core::{
    ChangeSource, CycleId, Db, Due, Error, IssueId, IssuePatch, LabelId, NewIssue, PinId, PinKind,
    ProjectId, PropertyDef, PropertyKind, Relation, RelationKind, Result, SelectOption, StatusId,
    ViewDef, ViewId, backup, comments, cycles, history, issues, labels, pins, projects, properties,
    quickadd, relations, search, views,
};

use crate::{
    Cmd, CommentCmd, CycleCmd, IssueCmd, LabelCmd, PinCmd, ProjectCmd, PropertyCmd, RelationCmd,
    ViewCmd,
};

const SRC: ChangeSource = ChangeSource::Cli;

pub async fn run(db: &Db, dir: &Path, cmd: Cmd) -> Result<Value> {
    match cmd {
        Cmd::Add { text, tz } => add(db, &text, tz.as_deref()).await,
        Cmd::Issue(cmd) => issue(db, cmd).await,
        Cmd::View(cmd) => view(db, cmd).await,
        Cmd::Property(cmd) => property(db, dir, cmd).await,
        Cmd::Project(cmd) => project(db, cmd).await,
        Cmd::Cycle(cmd) => cycle(db, cmd).await,
        Cmd::Label(cmd) => label(db, cmd).await,
        Cmd::Comment(CommentCmd::Add {
            key,
            body,
            created_at,
            migration,
        }) => {
            let id = issue_id(db, &key).await?;
            let at = created_at.as_deref().map(timestamp).transpose()?;
            val(comments::add_comment(db, id, &body, at, source(migration)).await?)
        }
        Cmd::Relation(cmd) => relation(db, cmd).await,
        Cmd::Pin(cmd) => pin(db, cmd).await,
        Cmd::Backup(_) => Err(Error::Invalid(
            "backup runs before the database opens".into(),
        )),
        Cmd::History { key } => {
            let id = issue_id(db, &key).await?;
            val(history::list_for(db, "issue", &id.0.to_string()).await?)
        }
    }
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
            let new = NewIssue {
                created_at: created_at.as_deref().map(timestamp).transpose()?,
                completed_at: completed_at.as_deref().map(timestamp).transpose()?,
                properties: property_map(&properties)?,
                parent_id,
                cycle_id: cycle.map(CycleId),
                label_ids: (!labels.is_empty()).then(|| labels.into_iter().map(LabelId).collect()),
                title,
                description,
                priority: check_priority(priority)?,
                status_id: status.map(StatusId),
                project_id: project.map(ProjectId),
                due: due.as_deref().map(date).transpose()?.map(Due::Date),
                estimate_minutes: estimate,
                ..NewIssue::default()
            };
            val(issues::create(db, new, source(migration)).await?)
        }
        update @ IssueCmd::Update { .. } => issue_update(db, update).await,
        IssueCmd::Get { key } => val(issues::get_by_key(db, &key).await?),
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
    } = cmd
    else {
        unreachable!("issue_update is only called with Update");
    };
    let id = issue_id(db, &key).await?;
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
        status_id: status.map(StatusId),
        due,
        ..IssuePatch::default()
    };
    val(issues::update(db, id, patch, SRC).await?)
}

async fn view(db: &Db, cmd: ViewCmd) -> Result<Value> {
    let ViewCmd::Query { def, id } = cmd;
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
        } => {
            let def = PropertyDef {
                key,
                name,
                kind: property_kind(&kind_name)?,
                options: select_options(&options),
                project_id: None,
                position: 0,
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

async fn project(db: &Db, cmd: ProjectCmd) -> Result<Value> {
    match cmd {
        ProjectCmd::List => val(projects::list(db).await?),
        ProjectCmd::Create { slug, name } => val(projects::create(db, &slug, &name, SRC).await?),
    }
}

async fn cycle(db: &Db, cmd: CycleCmd) -> Result<Value> {
    match cmd {
        CycleCmd::List => val(cycles::list(db).await?),
        CycleCmd::Create { start, end, name } => {
            let (start, end) = (date(&start)?, date(&end)?);
            val(cycles::create(db, name, start, end, SRC).await?)
        }
    }
}

async fn label(db: &Db, cmd: LabelCmd) -> Result<Value> {
    match cmd {
        LabelCmd::List => val(labels::list(db).await?),
        LabelCmd::Create { name, color } => val(labels::create(db, &name, &color, SRC).await?),
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
        kind: relation_kind(&kind)?,
    };
    if add {
        relations::add(db, rel, SRC).await?;
        Ok(json!({"added": {"from": from, "kind": kind, "to": to}}))
    } else {
        relations::remove(db, rel, SRC).await?;
        Ok(json!({"removed": {"from": from, "kind": kind, "to": to}}))
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
async fn snapshot(db: &Db, dir: &Path, reason: &str) -> Result<()> {
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

fn relation_kind(name: &str) -> Result<RelationKind> {
    Ok(match name {
        "blocks" => RelationKind::Blocks,
        "related" => RelationKind::Related,
        "duplicate" => RelationKind::Duplicate,
        _ => return Err(Error::Invalid(format!("unknown relation kind: {name}"))),
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
