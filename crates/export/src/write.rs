use std::{collections::BTreeMap, fs, path::Path};

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::FromRow;
use taskapp_core::{Db, Result, attachments, settings};

use crate::files::{
    AttachmentFile, CommentText, CycleRow, DecisionFile, EventFile, FocusRow, GoalFile,
    ImportLogRow, IssueFront, IssueTemplateFile, LabelRow, LinkFile, MilestoneRow, PinRow,
    ProjectFile, ProjectStatusRow, PropertyFile, RelationKindRow, StatusRow, ViewFile,
    ViewFolderFile, ViewTemplateFile, WorkBlockRow, invalid, key, plain_dir, read_plain,
    render_issue, safe_name,
};

pub struct ExportStats {
    pub issues: u32,
    pub files_written: u32,
    pub files_removed: u32,
}

/// Directories whose files come from the database alone, so a file not in the snapshot is stale.
const MANAGED_DIRS: [&str; 3] = ["issues", "projects", "events"];

/// Writes the whole export under `dir`, touching only files whose content changed.
///
/// # Errors
///
/// Fails when the database cannot be read or a file cannot be written.
pub async fn write_snapshot(db: &Db, dir: &Path) -> Result<ExportStats> {
    refuse_empty(dir)?;
    for sub in MANAGED_DIRS {
        plain_dir(dir, sub)?;
    }
    let files = render(db).await?;
    let mut stats = ExportStats {
        issues: u32::try_from(files.keys().filter(|k| k.starts_with("issues/")).count())
            .map_err(invalid)?,
        files_written: 0,
        files_removed: 0,
    };
    for (rel, text) in &files {
        let path = dir.join(rel);
        // A link at a managed path would send the write elsewhere; refuse rather than follow it.
        if fs::symlink_metadata(&path).is_ok_and(|m| !m.is_file()) {
            return Err(taskapp_core::Error::Invalid(format!(
                "{rel} in the export is not a regular file"
            )));
        }
        if read_plain(&path).is_ok_and(|old| old == *text) {
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, text)?;
        stats.files_written += 1;
    }
    for sub in MANAGED_DIRS {
        stats.files_removed += remove_stale(dir, sub, &files)?;
    }
    stats.files_written += write_images(db, dir).await?;
    Ok(stats)
}

/// Copies the images some text still points at into `attachments/` and lists them in `attachments.yaml`; an
/// image nothing points at any more is dropped from the folder (the database keeps it).
async fn write_images(db: &Db, dir: &Path) -> Result<u32> {
    let folder = dir.join("attachments");
    let mut written = 0;
    let mut kept = std::collections::HashSet::new();
    let mut manifest = Vec::new();
    for a in attachments::referenced(db).await? {
        let Ok(data) = attachments::get(db, a.id).await else {
            continue;
        };
        let name = attachments::inspect(&data.bytes)?.file();
        let path = folder.join(&name);
        if fs::symlink_metadata(&path).is_err() {
            plain_dir(dir, "attachments")?;
            fs::create_dir_all(&folder)?;
            fs::write(&path, &data.bytes)?;
            written += 1;
        } else if !fs::symlink_metadata(&path)?.is_file() {
            return Err(taskapp_core::Error::Invalid(format!(
                "attachments/{name} in the export is not a regular file"
            )));
        }
        kept.insert(name);
        manifest.push(AttachmentFile {
            id: a.id.0,
            sha256: a.sha256,
            created_at: a.created_at,
        });
    }
    let text = yaml(&manifest)?;
    let path = dir.join("attachments.yaml");
    if fs::symlink_metadata(&path).is_ok_and(|m| !m.is_file()) {
        return Err(invalid(
            "attachments.yaml in the export is not a regular file",
        ));
    }
    if read_plain(&path).is_ok_and(|old| old == text) {
        return Ok(written);
    }
    fs::write(&path, text)?;
    if plain_dir(dir, "attachments")? {
        for entry in fs::read_dir(&folder)? {
            let entry = entry?;
            if entry.file_type()?.is_file()
                && !kept.contains(entry.file_name().to_string_lossy().as_ref())
            {
                fs::remove_file(entry.path())?;
            }
        }
    }
    Ok(written)
}

fn remove_stale(dir: &Path, sub: &str, files: &BTreeMap<String, String>) -> Result<u32> {
    let folder = dir.join(sub);
    if !plain_dir(dir, sub)? {
        return Ok(0);
    }
    let mut removed = 0;
    for entry in fs::read_dir(&folder)? {
        let entry = entry?;
        let rel = format!("{sub}/{}", entry.file_name().to_string_lossy());
        if entry.file_type()?.is_file() && !files.contains_key(&rel) {
            fs::remove_file(entry.path())?;
            removed += 1;
        }
    }
    Ok(removed)
}

/// Every file of the snapshot, by relative path.
async fn render(db: &Db) -> Result<BTreeMap<String, String>> {
    let prefix: String = settings::get(db, "issue_prefix")
        .await?
        .ok_or_else(|| invalid("issue_prefix setting is missing"))?;
    safe_name(&prefix)?;
    let mut files = BTreeMap::new();
    files.insert("settings.yaml".to_string(), yaml(&settings_map(db).await?)?);
    files.insert("statuses.yaml".to_string(), yaml(&statuses(db).await?)?);
    files.insert("labels.yaml".to_string(), yaml(&labels(db).await?)?);
    files.insert("cycles.yaml".to_string(), yaml(&cycles(db).await?)?);
    files.insert("properties.yaml".to_string(), yaml(&properties(db).await?)?);
    files.insert("views.yaml".to_string(), yaml(&views(db).await?)?);
    files.insert("pins.yaml".to_string(), yaml(&pins(db).await?)?);
    template_files(db, &mut files).await?;
    for project in projects(db).await? {
        let rel = format!("projects/{}.yaml", safe_name(&project.slug)?);
        files.insert(rel, yaml(&project)?);
    }
    event_files(db, &prefix, &mut files).await?;
    for (key, text) in issue_files(db, &prefix).await? {
        files.insert(format!("issues/{key}.md"), text);
    }
    Ok(files)
}

async fn template_files(db: &Db, files: &mut BTreeMap<String, String>) -> Result<()> {
    let view_folders = yaml(&view_folders(db).await?)?;
    files.insert("view_folders.yaml".to_string(), view_folders);
    let view_templates = yaml(&view_templates(db).await?)?;
    files.insert("view_templates.yaml".to_string(), view_templates);
    let issue_templates = yaml(&issue_templates(db).await?)?;
    files.insert("issue_templates.yaml".to_string(), issue_templates);
    files.insert("import_log.yaml".to_string(), yaml(&import_log(db).await?)?);
    files.insert("local.yaml".to_string(), yaml(&local_settings(db).await?)?);
    let project_statuses = yaml(&project_statuses(db).await?)?;
    files.insert("project_statuses.yaml".to_string(), project_statuses);
    files.insert(
        "relation_kinds.yaml".to_string(),
        yaml(&relation_kinds(db).await?)?,
    );
    Ok(())
}

fn yaml<T: Serialize>(value: &T) -> Result<String> {
    serde_yaml_ng::to_string(value).map_err(invalid)
}

fn group<K: Ord, T>(rows: Vec<(K, T)>) -> BTreeMap<K, Vec<T>> {
    let mut map: BTreeMap<K, Vec<T>> = BTreeMap::new();
    for (k, v) in rows {
        map.entry(k).or_default().push(v);
    }
    map
}

/// Machine-local and secret settings stay out: tokens, paths, the remote URL (it may embed a
/// credential) and runtime state.
fn exported_setting(key: &str) -> bool {
    const LOCAL: [&str; 6] = ["token", "secret", "path", "dir", "remote", "sync_status"];
    let key = key.to_ascii_lowercase();
    !LOCAL.iter().any(|word| key.contains(word))
}

/// The per-machine settings that are safe to write down; `restore` offers them back.
pub(crate) const LOCAL_KEYS: [&str; 3] = ["export_dir", "export_remote", "backup_dir"];

async fn local_settings(db: &Db) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for key in LOCAL_KEYS {
        if let Some(v) = settings::get::<String>(db, key)
            .await?
            .filter(|v| !v.is_empty())
        {
            out.insert(key.to_string(), home_relative(&without_credentials(&v)));
        }
    }
    Ok(out)
}

/// A path under the home folder becomes `~/...`, so the file names no one and fits another machine.
fn home_relative(value: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    match value.strip_prefix(&home) {
        Some(rest) if !home.is_empty() && rest.starts_with('/') => format!("~{rest}"),
        _ => value.to_string(),
    }
}

/// `https://user:token@host/x` loses its `user:token@`; a bare `git@host:x` has no secret and stays.
pub(crate) fn without_credentials(url: &str) -> String {
    if let Some((scheme, rest)) = url.split_once("://")
        && let Some((auth, host)) = rest.split_once('@')
        && auth.contains(':')
        && !auth.contains('/')
    {
        return format!("{scheme}://{host}");
    }
    url.to_string()
}

async fn settings_map(db: &Db) -> Result<BTreeMap<String, Value>> {
    let rows = sqlx::query_as::<_, (String, String)>("SELECT key, value FROM settings")
        .fetch_all(&db.pool)
        .await?;
    rows.into_iter()
        .filter(|(k, _)| exported_setting(k))
        .map(|(k, v)| Ok((k, serde_json::from_str(&v).map_err(invalid)?)))
        .collect()
}

async fn statuses(db: &Db) -> Result<Vec<StatusRow>> {
    let rows = sqlx::query_as(
        "SELECT name, category, position, color, icon, definition, flow_kind, nudge_days
         FROM statuses ORDER BY position, id",
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

async fn project_statuses(db: &Db) -> Result<Vec<ProjectStatusRow>> {
    let rows = sqlx::query_as(
        "SELECT name, category, position, color, icon FROM project_statuses ORDER BY position, id",
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

async fn labels(db: &Db) -> Result<Vec<LabelRow>> {
    let rows = sqlx::query_as("SELECT name, color, icon FROM labels ORDER BY name")
        .fetch_all(&db.pool)
        .await?;
    Ok(rows)
}

async fn relation_kinds(db: &Db) -> Result<Vec<RelationKindRow>> {
    let rows = sqlx::query_as(
        "SELECT id, name, forward, reverse, symmetric, position FROM relation_kinds WHERE built_in = 0 ORDER BY position, id",
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

async fn cycles(db: &Db) -> Result<Vec<CycleRow>> {
    let rows = sqlx::query_as(
        "SELECT number, name, start_date, end_date, icon, color, folder_id, position, goal, goal_met, rating, notes FROM cycles ORDER BY number",
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

async fn decisions(db: &Db, prefix: &str) -> Result<Vec<DecisionFile>> {
    type Row = (
        i64,
        String,
        String,
        String,
        Option<String>,
        String,
        String,
        Option<String>,
        String,
        String,
        Option<String>,
        String,
        Option<String>,
    );
    let rows = sqlx::query_as::<_, Row>(
        "SELECT id, question, context, options, recommendation, mode, status, answer, note, source,
                external_id, created_at, answered_at FROM decisions ORDER BY id",
    )
    .fetch_all(&db.pool)
    .await?;
    let mut links = group(
        sqlx::query_as::<_, (i64, i64)>(
            "SELECT d.decision_id, i.number FROM decision_issues d JOIN issues i ON i.id = d.issue_id
             ORDER BY i.number",
        )
        .fetch_all(&db.pool)
        .await?
        .into_iter()
        .map(|(decision, number)| (decision, key(prefix, number)))
        .collect(),
    );
    rows.into_iter()
        .map(|r| {
            Ok(DecisionFile {
                issues: links.remove(&r.0).unwrap_or_default(),
                id: r.0,
                question: r.1,
                context: r.2,
                options: serde_json::from_str(&r.3).map_err(invalid)?,
                recommendation: r.4,
                mode: r.5,
                status: r.6,
                answer: r.7,
                note: r.8,
                source: r.9,
                external_id: r.10,
                created_at: r.11,
                answered_at: r.12,
            })
        })
        .collect()
}

async fn goals(db: &Db) -> Result<Vec<GoalFile>> {
    let rows = sqlx::query_as::<_, (i64, String, String, String, String, String)>(
        "SELECT id, title, quarter, description, status, created_at FROM goals ORDER BY id",
    )
    .fetch_all(&db.pool)
    .await?;
    let mut links = group(
        sqlx::query_as::<_, (i64, String)>(
            "SELECT g.goal_id, p.slug FROM goal_projects g JOIN projects p ON p.id = g.project_id
             ORDER BY p.slug",
        )
        .fetch_all(&db.pool)
        .await?,
    );
    Ok(rows
        .into_iter()
        .map(|r| GoalFile {
            projects: links.remove(&r.0).unwrap_or_default(),
            id: r.0,
            title: r.1,
            quarter: r.2,
            description: r.3,
            status: r.4,
            created_at: r.5,
        })
        .collect())
}

async fn links(db: &Db, prefix: &str) -> Result<Vec<LinkFile>> {
    let rows = sqlx::query_as::<_, (i64, i64, String, String, String, String)>(
        "SELECT l.id, i.number, l.url, l.title, l.kind, l.created_at FROM links l
         JOIN issues i ON i.id = l.issue_id ORDER BY l.id",
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| LinkFile {
            id: r.0,
            issue: key(prefix, r.1),
            url: r.2,
            title: r.3,
            kind: r.4,
            created_at: r.5,
        })
        .collect())
}

async fn event_files(db: &Db, prefix: &str, files: &mut BTreeMap<String, String>) -> Result<()> {
    files.insert("links.yaml".to_string(), yaml(&links(db, prefix).await?)?);
    files.insert("goals.yaml".to_string(), yaml(&goals(db).await?)?);
    files.insert(
        "decisions.yaml".to_string(),
        yaml(&decisions(db, prefix).await?)?,
    );
    files.insert(
        "decisions.yaml".to_string(),
        yaml(&decisions(db, prefix).await?)?,
    );
    for event in events(db, prefix).await? {
        files.insert(format!("events/{}.yaml", event.id), yaml(&event)?);
    }
    Ok(())
}

async fn events(db: &Db, prefix: &str) -> Result<Vec<EventFile>> {
    type Row = (
        i64,
        String,
        String,
        Option<String>,
        bool,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
        bool,
        String,
        String,
    );
    let rows = sqlx::query_as::<_, Row>(
        "SELECT id, title, starts_at, ends_at, all_day, place, notes, icon, color, archived,
                created_at, updated_at FROM events ORDER BY id",
    )
    .fetch_all(&db.pool)
    .await?;
    let mut links = group(
        sqlx::query_as::<_, (i64, i64)>(
            "SELECT e.event_id, i.number FROM event_issues e JOIN issues i ON i.id = e.issue_id
             ORDER BY i.number",
        )
        .fetch_all(&db.pool)
        .await?
        .into_iter()
        .map(|(event, number)| (event, key(prefix, number)))
        .collect(),
    );
    Ok(rows
        .into_iter()
        .map(|r| EventFile {
            issues: links.remove(&r.0).unwrap_or_default(),
            id: r.0,
            title: r.1,
            starts_at: r.2,
            ends_at: r.3,
            all_day: r.4,
            place: r.5,
            notes: r.6,
            icon: r.7,
            color: r.8,
            archived: r.9,
            created_at: r.10,
            updated_at: r.11,
        })
        .collect())
}

async fn pins(db: &Db) -> Result<Vec<PinRow>> {
    let rows = sqlx::query_as(
        "SELECT kind, target, title, position, folder_id FROM pins ORDER BY position, id",
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

async fn properties(db: &Db) -> Result<Vec<PropertyFile>> {
    let rows = sqlx::query_as::<_, (String, String, String, String, Option<String>, i64, String)>(
        "SELECT d.key, d.name, d.kind, d.options, p.slug, d.position, d.applies_to
         FROM property_defs d LEFT JOIN projects p ON p.id = d.project_id
         ORDER BY d.position, d.key",
    )
    .fetch_all(&db.pool)
    .await?;
    rows.into_iter()
        .map(
            |(key, name, kind, options, project, position, applies_to)| {
                Ok(PropertyFile {
                    key,
                    name,
                    kind: serde_json::from_str(&kind).map_err(invalid)?,
                    options: serde_json::from_str(&options).map_err(invalid)?,
                    project,
                    position,
                    applies_to,
                })
            },
        )
        .collect()
}

async fn views(db: &Db) -> Result<Vec<ViewFile>> {
    type Row = (
        String,
        String,
        String,
        i64,
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<i64>,
    );
    let rows = sqlx::query_as::<_, Row>(
        "SELECT name, entity, definition, position, icon, color, template_id, folder_id FROM views ORDER BY position, id",
    )
    .fetch_all(&db.pool)
    .await?;
    rows.into_iter()
        .map(
            |(name, entity, definition, position, icon, color, template_id, folder_id)| {
                Ok(ViewFile {
                    name,
                    entity,
                    position,
                    definition: serde_json::from_str(&definition).map_err(invalid)?,
                    icon,
                    color,
                    template_id,
                    folder_id,
                })
            },
        )
        .collect()
}

async fn view_folders(db: &Db) -> Result<Vec<ViewFolderFile>> {
    let rows = sqlx::query_as::<_, (i64, String, i64, String, Option<i64>)>(
        "SELECT id, name, position, section, parent_id FROM view_folders ORDER BY position, id",
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name, position, section, parent_id)| ViewFolderFile {
            id,
            name,
            position,
            section,
            parent_id,
        })
        .collect())
}

async fn view_templates(db: &Db) -> Result<Vec<ViewTemplateFile>> {
    type Row = (i64, String, String, i64, Option<String>, Option<String>);
    let rows = sqlx::query_as::<_, Row>(
        "SELECT id, name, definition, position, icon, color FROM view_templates ORDER BY position, id",
    )
    .fetch_all(&db.pool)
    .await?;
    rows.into_iter()
        .map(|(id, name, definition, position, icon, color)| {
            Ok(ViewTemplateFile {
                id,
                name,
                position,
                definition: serde_json::from_str(&definition).map_err(invalid)?,
                icon,
                color,
            })
        })
        .collect()
}

async fn issue_templates(db: &Db) -> Result<Vec<IssueTemplateFile>> {
    type Row = (String, String, Option<String>, Option<String>, String);
    let rows = sqlx::query_as::<_, Row>(
        "SELECT name, slug, icon, color, body FROM issue_templates ORDER BY id",
    )
    .fetch_all(&db.pool)
    .await?;
    rows.into_iter()
        .map(|(name, slug, icon, color, body)| {
            Ok(IssueTemplateFile {
                name,
                slug,
                body: serde_json::from_str(&body).map_err(invalid)?,
                icon,
                color,
            })
        })
        .collect()
}

async fn import_log(db: &Db) -> Result<Vec<ImportLogRow>> {
    let rows = sqlx::query_as::<_, (String, String, i64, String, bool)>(
        "SELECT l.external_id, l.source, i.number, l.imported_at, l.source_deleted
         FROM import_log l JOIN issues i ON i.id = l.issue_id ORDER BY l.external_id",
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(external_id, source, issue, imported_at, source_deleted)| ImportLogRow {
                external_id,
                source,
                issue,
                imported_at,
                source_deleted,
            },
        )
        .collect())
}

async fn projects(db: &Db) -> Result<Vec<ProjectFile>> {
    let mut projects: Vec<ProjectFile> = sqlx::query_as(
        "SELECT p.slug, p.name, p.description, p.doc_link, p.start_date, p.target_date, p.archived_at,
                p.created_at, p.updated_at, p.icon, p.color, s.name AS status, p.priority, p.folder_id, p.position,
                (SELECT i.number FROM issues i WHERE i.id = p.next_issue_id) AS next_issue
         FROM projects p JOIN project_statuses s ON s.id = p.status_id ORDER BY p.slug",
    )
    .fetch_all(&db.pool)
    .await?;
    let labels = sqlx::query_as::<_, (String, String)>(
        "SELECT p.slug, l.name FROM project_labels pl JOIN projects p ON p.id = pl.project_id
         JOIN labels l ON l.id = pl.label_id ORDER BY l.name",
    )
    .fetch_all(&db.pool)
    .await?;
    let mut labels = group(labels);
    let values = sqlx::query_as::<_, (String, String)>("SELECT slug, properties FROM projects")
        .fetch_all(&db.pool)
        .await?;
    let mut values: BTreeMap<String, String> = values.into_iter().collect();
    let milestones = sqlx::query_as::<_, (String, String, Option<NaiveDate>)>(
        "SELECT p.slug, m.name, m.target_date FROM milestones m
         JOIN projects p ON p.id = m.project_id ORDER BY m.id",
    )
    .fetch_all(&db.pool)
    .await?;
    let mut by_slug = group(
        milestones
            .into_iter()
            .map(|(slug, name, target_date)| (slug, MilestoneRow { name, target_date }))
            .collect(),
    );
    for project in &mut projects {
        project.milestones = by_slug.remove(&project.slug).unwrap_or_default();
        project.labels = labels.remove(&project.slug).unwrap_or_default();
        let text = values.remove(&project.slug).unwrap_or_default();
        project.properties = serde_json::from_str(&text).map_err(invalid)?;
    }
    Ok(projects)
}

/// The issue row as the file's frontmatter needs it: references are joined to names or keys.
#[derive(FromRow)]
struct IssueRow {
    number: i64,
    title: String,
    description: String,
    status: String,
    priority: i64,
    project: Option<String>,
    milestone: Option<String>,
    parent: Option<i64>,
    cycle: Option<i64>,
    start_date: Option<NaiveDate>,
    due_date: Option<NaiveDate>,
    due_time_utc: Option<DateTime<Utc>>,
    due_tz: Option<String>,
    estimate_minutes: Option<i64>,
    points: Option<f64>,
    hidden_until: Option<NaiveDate>,
    depth: Option<String>,
    repeat_rule: Option<String>,
    properties: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    archived_at: Option<DateTime<Utc>>,
}

const ISSUE_SQL: &str = "SELECT i.number, i.title, i.description, s.name AS status, i.priority,
        p.slug AS project, m.name AS milestone, par.number AS parent, c.number AS cycle,
        i.start_date, i.due_date, i.due_time_utc, i.due_tz, i.estimate_minutes, i.points, i.hidden_until, i.depth, i.repeat_rule, i.properties,
        i.created_at, i.updated_at, i.completed_at, i.archived_at
    FROM issues i
    JOIN statuses s ON s.id = i.status_id
    LEFT JOIN projects p ON p.id = i.project_id
    LEFT JOIN milestones m ON m.id = i.milestone_id
    LEFT JOIN issues par ON par.id = i.parent_id
    LEFT JOIN cycles c ON c.id = i.cycle_id
    ORDER BY i.number";

/// `(kind, target issue number)` pairs, in the order the database returns them.
type Relations = Vec<(String, i64)>;

/// The per-issue lists an issue file needs, keyed by issue number.
struct IssueSide {
    labels: BTreeMap<i64, Vec<String>>,
    relations: BTreeMap<i64, Relations>,
    comments: BTreeMap<i64, Vec<CommentText>>,
    blocks: BTreeMap<i64, Vec<WorkBlockRow>>,
}

async fn issue_side(db: &Db) -> Result<IssueSide> {
    let labels = group(
        sqlx::query_as::<_, (i64, String)>(
            "SELECT i.number, l.name FROM issue_labels il
             JOIN issues i ON i.id = il.issue_id JOIN labels l ON l.id = il.label_id
             ORDER BY l.name",
        )
        .fetch_all(&db.pool)
        .await?,
    );
    let relations = group(
        sqlx::query_as::<_, (i64, String, i64)>(
            "SELECT f.number, r.kind, t.number FROM relations r
             JOIN issues f ON f.id = r.from_id JOIN issues t ON t.id = r.to_id
             ORDER BY f.number, r.kind, t.number",
        )
        .fetch_all(&db.pool)
        .await?
        .into_iter()
        .map(|(from, kind, to)| (from, (kind, to)))
        .collect(),
    );
    let comments = group(
        sqlx::query_as::<_, (i64, String, DateTime<Utc>, DateTime<Utc>)>(
            "SELECT i.number, c.body, c.created_at, c.updated_at FROM comments c
             JOIN issues i ON i.id = c.issue_id ORDER BY c.created_at, c.id",
        )
        .fetch_all(&db.pool)
        .await?
        .into_iter()
        .map(|(number, body, created_at, updated_at)| {
            (
                number,
                CommentText {
                    body,
                    created_at,
                    updated_at,
                },
            )
        })
        .collect(),
    );
    let blocks = group(
        sqlx::query_as::<_, (i64, DateTime<Utc>, DateTime<Utc>)>(
            "SELECT i.number, b.start_utc, b.end_utc FROM work_blocks b
             JOIN issues i ON i.id = b.issue_id ORDER BY b.start_utc, b.end_utc, b.id",
        )
        .fetch_all(&db.pool)
        .await?
        .into_iter()
        .map(|(number, start, end)| (number, WorkBlockRow { start, end }))
        .collect(),
    );
    Ok(IssueSide {
        labels,
        relations,
        comments,
        blocks,
    })
}

async fn focus_sessions(db: &Db) -> Result<BTreeMap<i64, Vec<FocusRow>>> {
    Ok(group(
        sqlx::query_as::<_, (i64, DateTime<Utc>, Option<DateTime<Utc>>, bool)>(
            "SELECT i.number, f.started_at, f.ended_at, f.deep FROM focus_sessions f
             JOIN issues i ON i.id = f.issue_id ORDER BY f.started_at, f.id",
        )
        .fetch_all(&db.pool)
        .await?
        .into_iter()
        .map(|(number, start, end, deep)| (number, FocusRow { start, end, deep }))
        .collect(),
    ))
}

/// `(key, file text)` for every issue, in issue-number order.
async fn issue_files(db: &Db, prefix: &str) -> Result<Vec<(String, String)>> {
    let mut side = issue_side(db).await?;
    let mut focus = focus_sessions(db).await?;
    let mut out = Vec::new();
    for row in sqlx::query_as::<_, IssueRow>(ISSUE_SQL)
        .fetch_all(&db.pool)
        .await?
    {
        let number = row.number;
        let labels = side.labels.remove(&number).unwrap_or_default();
        let rels = side.relations.remove(&number).unwrap_or_default();
        let comments = side.comments.remove(&number).unwrap_or_default();
        let (mut front, description) = front_of(prefix, row, labels, &rels)?;
        front.work_blocks = side.blocks.remove(&number).unwrap_or_default();
        front.focus = focus.remove(&number).unwrap_or_default();
        let text = render_issue(&front, &description, &comments)?;
        out.push((front.key, text));
    }
    Ok(out)
}

/// The frontmatter and description of one issue row.
fn front_of(
    prefix: &str,
    row: IssueRow,
    labels: Vec<String>,
    rels: &[(String, i64)],
) -> Result<(IssueFront, String)> {
    let IssueRow {
        number,
        title,
        description,
        status,
        priority,
        project,
        milestone,
        parent,
        cycle,
        start_date,
        due_date,
        due_time_utc,
        due_tz,
        estimate_minutes,
        points,
        hidden_until,
        depth,
        repeat_rule,
        properties,
        created_at,
        updated_at,
        completed_at,
        archived_at,
    } = row;
    let front = IssueFront {
        key: key(prefix, number),
        title,
        status,
        priority,
        project,
        milestone,
        parent: parent.map(|p| key(prefix, p)),
        cycle,
        labels,
        start_date,
        due_date,
        due_time: due_time_utc,
        due_tz,
        estimate_minutes,
        points,
        hidden_until,
        depth,
        repeat: repeat_rule,
        created_at,
        updated_at,
        completed_at,
        archived_at,
        blocks: keys_of(rels, "blocks", prefix),
        related: keys_of(rels, "related", prefix),
        duplicate: keys_of(rels, "duplicate", prefix),
        relations: other_kinds(rels, prefix),
        properties: serde_json::from_str(&properties).map_err(invalid)?,
        work_blocks: vec![],
        focus: vec![],
    };
    Ok((front, description))
}

fn other_kinds(rels: &[(String, i64)], prefix: &str) -> BTreeMap<String, Vec<String>> {
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (kind, to) in rels {
        if !["blocks", "related", "duplicate"].contains(&kind.as_str()) {
            map.entry(kind.clone()).or_default().push(key(prefix, *to));
        }
    }
    map
}

fn keys_of(rels: &[(String, i64)], kind: &str, prefix: &str) -> Vec<String> {
    rels.iter()
        .filter(|(k, _)| k == kind)
        .map(|(_, to)| key(prefix, *to))
        .collect()
}

/// An empty path means "the current directory" to `fs` and to `git -C`, which would export into the cwd.
pub(crate) fn refuse_empty(dir: &Path) -> Result<()> {
    if dir.as_os_str().is_empty() {
        return Err(taskapp_core::Error::Invalid(
            "export folder is empty".into(),
        ));
    }
    Ok(())
}
