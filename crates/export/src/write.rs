use std::{collections::BTreeMap, fs, path::Path};

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::FromRow;
use taskapp_core::{Db, Result, settings};

use crate::files::{
    CommentText, CycleRow, ImportLogRow, IssueFront, IssueTemplateFile, LabelRow, MilestoneRow,
    PinRow, ProjectFile, PropertyFile, StatusRow, ViewFile, ViewTemplateFile, invalid, key,
    render_issue, safe_name,
};

pub struct ExportStats {
    pub issues: u32,
    pub files_written: u32,
    pub files_removed: u32,
}

/// Directories whose files come from the database alone, so a file not in the snapshot is stale.
const MANAGED_DIRS: [&str; 2] = ["issues", "projects"];

/// Writes the whole export under `dir`, touching only files whose content changed.
///
/// # Errors
///
/// Fails when the database cannot be read or a file cannot be written.
pub async fn write_snapshot(db: &Db, dir: &Path) -> Result<ExportStats> {
    refuse_empty(dir)?;
    let files = render(db).await?;
    let mut stats = ExportStats {
        issues: u32::try_from(files.keys().filter(|k| k.starts_with("issues/")).count())
            .map_err(invalid)?,
        files_written: 0,
        files_removed: 0,
    };
    for (rel, text) in &files {
        let path = dir.join(rel);
        if fs::read_to_string(&path).is_ok_and(|old| old == *text) {
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
    Ok(stats)
}

fn remove_stale(dir: &Path, sub: &str, files: &BTreeMap<String, String>) -> Result<u32> {
    let folder = dir.join(sub);
    if !folder.is_dir() {
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
    for (key, text) in issue_files(db, &prefix).await? {
        files.insert(format!("issues/{key}.md"), text);
    }
    Ok(files)
}

async fn template_files(db: &Db, files: &mut BTreeMap<String, String>) -> Result<()> {
    let view_templates = yaml(&view_templates(db).await?)?;
    files.insert("view_templates.yaml".to_string(), view_templates);
    let issue_templates = yaml(&issue_templates(db).await?)?;
    files.insert("issue_templates.yaml".to_string(), issue_templates);
    files.insert("import_log.yaml".to_string(), yaml(&import_log(db).await?)?);
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
        "SELECT name, category, position, color, icon FROM statuses ORDER BY position, id",
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

async fn cycles(db: &Db) -> Result<Vec<CycleRow>> {
    let rows = sqlx::query_as(
        "SELECT number, name, start_date, end_date, icon, color FROM cycles ORDER BY number",
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

async fn pins(db: &Db) -> Result<Vec<PinRow>> {
    let rows =
        sqlx::query_as("SELECT kind, target, title, position FROM pins ORDER BY position, id")
            .fetch_all(&db.pool)
            .await?;
    Ok(rows)
}

async fn properties(db: &Db) -> Result<Vec<PropertyFile>> {
    let rows = sqlx::query_as::<_, (String, String, String, String, Option<String>, i64)>(
        "SELECT d.key, d.name, d.kind, d.options, p.slug, d.position
         FROM property_defs d LEFT JOIN projects p ON p.id = d.project_id
         ORDER BY d.position, d.key",
    )
    .fetch_all(&db.pool)
    .await?;
    rows.into_iter()
        .map(|(key, name, kind, options, project, position)| {
            Ok(PropertyFile {
                key,
                name,
                kind: serde_json::from_str(&kind).map_err(invalid)?,
                options: serde_json::from_str(&options).map_err(invalid)?,
                project,
                position,
            })
        })
        .collect()
}

async fn views(db: &Db) -> Result<Vec<ViewFile>> {
    type Row = (
        String,
        String,
        i64,
        Option<String>,
        Option<String>,
        Option<i64>,
    );
    let rows = sqlx::query_as::<_, Row>(
        "SELECT name, definition, position, icon, color, template_id FROM views ORDER BY position, id",
    )
    .fetch_all(&db.pool)
    .await?;
    rows.into_iter()
        .map(|(name, definition, position, icon, color, template_id)| {
            Ok(ViewFile {
                name,
                position,
                definition: serde_json::from_str(&definition).map_err(invalid)?,
                icon,
                color,
                template_id,
            })
        })
        .collect()
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
    let rows = sqlx::query_as::<_, (String, String, i64, String)>(
        "SELECT l.external_id, l.source, i.number, l.imported_at
         FROM import_log l JOIN issues i ON i.id = l.issue_id ORDER BY l.external_id",
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(external_id, source, issue, imported_at)| ImportLogRow {
            external_id,
            source,
            issue,
            imported_at,
        })
        .collect())
}

async fn projects(db: &Db) -> Result<Vec<ProjectFile>> {
    let mut projects: Vec<ProjectFile> = sqlx::query_as(
        "SELECT slug, name, description, doc_link, start_date, target_date, archived_at, created_at, icon, color
         FROM projects ORDER BY slug",
    )
    .fetch_all(&db.pool)
    .await?;
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
    properties: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    archived_at: Option<DateTime<Utc>>,
}

const ISSUE_SQL: &str = "SELECT i.number, i.title, i.description, s.name AS status, i.priority,
        p.slug AS project, m.name AS milestone, par.number AS parent, c.number AS cycle,
        i.start_date, i.due_date, i.due_time_utc, i.due_tz, i.estimate_minutes, i.points, i.properties,
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
    Ok(IssueSide {
        labels,
        relations,
        comments,
    })
}

/// `(key, file text)` for every issue, in issue-number order.
async fn issue_files(db: &Db, prefix: &str) -> Result<Vec<(String, String)>> {
    let mut side = issue_side(db).await?;
    let mut out = Vec::new();
    for row in sqlx::query_as::<_, IssueRow>(ISSUE_SQL)
        .fetch_all(&db.pool)
        .await?
    {
        let number = row.number;
        let labels = side.labels.remove(&number).unwrap_or_default();
        let rels = side.relations.remove(&number).unwrap_or_default();
        let comments = side.comments.remove(&number).unwrap_or_default();
        let (front, description) = front_of(prefix, row, labels, &rels)?;
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
        created_at,
        updated_at,
        completed_at,
        archived_at,
        blocks: keys_of(rels, "blocks", prefix),
        related: keys_of(rels, "related", prefix),
        duplicate: keys_of(rels, "duplicate", prefix),
        properties: serde_json::from_str(&properties).map_err(invalid)?,
    };
    Ok((front, description))
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
