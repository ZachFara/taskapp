use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::de::DeserializeOwned;
use serde_json::Value;
use sqlx::{Sqlite, Transaction};
use taskapp_core::{Db, Error, IssueId, Result, ViewDef, attachments, search};

use crate::files::{
    AttachmentFile, CommentText, CycleRow, EventFile, ImportLogRow, IssueFront, IssueTemplateFile,
    LabelRow, PinRow, ProjectFile, ProjectStatusRow, PropertyFile, RelationKindRow, StatusRow,
    ViewFile, ViewFolderFile, ViewTemplateFile, invalid, parse_issue, plain_dir, read_plain,
};

type Tx<'a> = Transaction<'a, Sqlite>;

/// Rebuilds a database at `db_path` from an export directory. The file must not exist yet: a
/// rebuild never overwrites data.
///
/// # Errors
///
/// Fails when `db_path` exists, a file cannot be parsed, a reference names nothing in the export,
/// or the database cannot be written. A failed rebuild leaves a partial database at `db_path`.
pub async fn rebuild(dir: &Path, db_path: &Path) -> Result<Db> {
    if db_path.exists() {
        return Err(Error::Invalid(format!(
            "{} already exists",
            db_path.display()
        )));
    }
    let db = Db::open(db_path).await?;
    restore_into(&db, dir).await?;
    Ok(db)
}

/// Replaces everything in an open database with the export's content, in one transaction: a folder
/// that fails to parse leaves the database as it was. Settings the export does not carry (this
/// machine's own) stay; the change history goes, because its row ids name rows that no longer exist.
///
/// # Errors
///
/// As [`rebuild`]; nothing is changed then.
pub async fn restore_into(db: &Db, dir: &Path) -> Result<()> {
    let mut tx = db.pool.begin().await?;
    wipe(&mut tx).await?;
    restore_tables(db, &mut tx, dir).await?;
    restore_issues(&mut tx, dir).await?;
    tx.commit().await?;
    // reindex rebuilds every search row at once; the id argument is not used.
    search::reindex(db, IssueId(0)).await?;
    Ok(())
}

/// Per-machine settings in `local.yaml` (the export folder, the remote, the backup folder): written
/// by every export, applied only when the person asks.
///
/// # Errors
///
/// Fails when the file is malformed or a setting cannot be written.
pub async fn apply_local(db: &Db, dir: &Path) -> Result<()> {
    let local: BTreeMap<String, String> = if fs::symlink_metadata(dir.join("local.yaml")).is_ok() {
        read_yaml(&dir.join("local.yaml"))?
    } else {
        BTreeMap::new()
    };
    for key in crate::write::LOCAL_KEYS {
        if let Some(v) = local.get(key) {
            taskapp_core::settings::set(db, key, v).await?;
        }
    }
    Ok(())
}

/// Empties every table the export fills. Order follows the foreign keys.
async fn wipe(tx: &mut Tx<'_>) -> Result<()> {
    for sql in [
        "DELETE FROM changes",
        "DELETE FROM attachments",
        "DELETE FROM event_issues",
        "DELETE FROM events",
        "DELETE FROM relations",
        "DELETE FROM issue_labels",
        "DELETE FROM comments",
        "DELETE FROM work_blocks",
        "DELETE FROM import_log",
        "DELETE FROM issues",
        "DELETE FROM property_defs",
        "DELETE FROM milestones",
        "DELETE FROM project_labels",
        "DELETE FROM projects",
        "DELETE FROM cycles",
        "DELETE FROM labels",
        "DELETE FROM pins",
        "DELETE FROM views",
        "DELETE FROM view_folders",
        "DELETE FROM view_templates",
        "DELETE FROM issue_templates",
        "DELETE FROM statuses",
        "DELETE FROM relation_kinds WHERE built_in = 0",
    ] {
        sqlx::query(sql).execute(&mut **tx).await?;
    }
    Ok(())
}

/// `(number, frontmatter, description, comments)` for one issue file.
type IssueEntry = (i64, IssueFront, String, Vec<CommentText>);

async fn restore_tables(db: &Db, tx: &mut Tx<'_>, dir: &Path) -> Result<()> {
    restore_settings(tx, dir).await?;
    restore_statuses(tx, dir).await?;
    restore_labels(tx, dir).await?;
    restore_project_statuses(tx, dir).await?;
    restore_relation_kinds(tx, dir).await?;
    restore_cycles(tx, dir).await?;
    restore_projects(tx, dir).await?;
    restore_properties(tx, dir).await?;
    restore_view_templates(tx, dir).await?;
    restore_view_folders(tx, dir).await?;
    restore_views(tx, dir).await?;
    restore_pins(tx, dir).await?;
    restore_issue_templates(tx, dir).await?;
    restore_attachments(db, tx, dir).await?;
    Ok(())
}

/// Inserts every issue first, so parents, relations and labels can refer to any of them.
async fn restore_issues(tx: &mut Tx<'_>, dir: &Path) -> Result<()> {
    let prefix = prefix(tx).await?;
    let issues = read_issues(dir, &prefix)?;
    for (number, front, description, _) in &issues {
        insert_issue(tx, *number, front, description).await?;
    }
    for (number, front, _, comments) in &issues {
        link_issue(tx, &prefix, *number, front, comments).await?;
    }
    restore_events(tx, dir, &prefix).await?;
    restore_import_log(tx, dir).await
}

/// After the issues, so the links can name them.
async fn restore_events(tx: &mut Tx<'_>, dir: &Path, prefix: &str) -> Result<()> {
    for path in files_in(dir, "events", "yaml")? {
        let e: EventFile = read_yaml(&path)?;
        sqlx::query(
            "INSERT INTO events (id, title, starts_at, ends_at, all_day, place, notes, icon, color,
               archived, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(e.id)
        .bind(e.title)
        .bind(e.starts_at)
        .bind(e.ends_at)
        .bind(e.all_day)
        .bind(e.place)
        .bind(e.notes)
        .bind(e.icon)
        .bind(e.color)
        .bind(e.archived)
        .bind(e.created_at)
        .bind(e.updated_at)
        .execute(&mut **tx)
        .await?;
        for key in &e.issues {
            sqlx::query(
                "INSERT INTO event_issues (event_id, issue_id)
                 VALUES (?, (SELECT id FROM issues WHERE number = ?))",
            )
            .bind(e.id)
            .bind(number_of(prefix, key)?)
            .execute(&mut **tx)
            .await?;
        }
    }
    Ok(())
}

pub(crate) fn read_yaml<T: DeserializeOwned>(path: &Path) -> Result<T> {
    serde_yaml_ng::from_str(&read_plain(path)?).map_err(invalid)
}

/// A list file that older exports do not have reads as empty.
pub(crate) fn read_list<T: DeserializeOwned>(path: &Path) -> Result<Vec<T>> {
    if fs::symlink_metadata(path).is_ok() {
        read_yaml(path)
    } else {
        Ok(vec![])
    }
}

/// The sorted files in `dir/sub` with the given extension.
pub(crate) fn files_in(dir: &Path, sub: &str, ext: &str) -> Result<Vec<PathBuf>> {
    let folder = dir.join(sub);
    if !plain_dir(dir, sub)? {
        return Ok(vec![]);
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(&folder)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == ext) {
            if !entry.file_type()?.is_file() {
                return Err(Error::Invalid(format!(
                    "{} is not a regular file",
                    path.display()
                )));
            }
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

async fn prefix(tx: &mut Tx<'_>) -> Result<String> {
    let json: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'issue_prefix'")
        .fetch_one(&mut **tx)
        .await?;
    serde_json::from_str(&json).map_err(invalid)
}

async fn restore_settings(tx: &mut Tx<'_>, dir: &Path) -> Result<()> {
    let settings: BTreeMap<String, Value> = read_yaml(&dir.join("settings.yaml"))?;
    for (key, value) in settings {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(key)
        .bind(value.to_string())
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn restore_statuses(tx: &mut Tx<'_>, dir: &Path) -> Result<()> {
    let statuses: Vec<StatusRow> = read_yaml(&dir.join("statuses.yaml"))?;
    for s in statuses {
        sqlx::query(
            "INSERT INTO statuses (name, category, position, color, icon) VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(name) DO UPDATE SET category = excluded.category,
               position = excluded.position, color = excluded.color, icon = excluded.icon",
        )
        .bind(s.name)
        .bind(s.category)
        .bind(s.position)
        .bind(s.color)
        .bind(s.icon)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn restore_project_statuses(tx: &mut Tx<'_>, dir: &Path) -> Result<()> {
    let statuses: Vec<ProjectStatusRow> = read_list(&dir.join("project_statuses.yaml"))?;
    if statuses.is_empty() {
        return Ok(());
    }
    // The export is the whole list, so it replaces the seeded one.
    sqlx::query("DELETE FROM project_statuses")
        .execute(&mut **tx)
        .await?;
    for s in statuses {
        sqlx::query(
            "INSERT INTO project_statuses (name, category, position, color, icon) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(s.name)
        .bind(s.category)
        .bind(s.position)
        .bind(s.color)
        .bind(s.icon)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn restore_labels(tx: &mut Tx<'_>, dir: &Path) -> Result<()> {
    let labels: Vec<LabelRow> = read_yaml(&dir.join("labels.yaml"))?;
    for l in labels {
        sqlx::query("INSERT INTO labels (name, color, icon) VALUES (?, ?, ?)")
            .bind(l.name)
            .bind(l.color)
            .bind(l.icon)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

async fn restore_relation_kinds(tx: &mut Tx<'_>, dir: &Path) -> Result<()> {
    let kinds: Vec<RelationKindRow> = read_list(&dir.join("relation_kinds.yaml"))?;
    for k in kinds {
        sqlx::query(
            "INSERT INTO relation_kinds (id, name, forward, reverse, symmetric, built_in, position)
             VALUES (?, ?, ?, ?, ?, 0, ?)",
        )
        .bind(k.id)
        .bind(k.name)
        .bind(k.forward)
        .bind(k.reverse)
        .bind(k.symmetric)
        .bind(k.position)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn restore_cycles(tx: &mut Tx<'_>, dir: &Path) -> Result<()> {
    let cycles: Vec<CycleRow> = read_yaml(&dir.join("cycles.yaml"))?;
    for c in cycles {
        sqlx::query(
            "INSERT INTO cycles (number, name, start_date, end_date, icon, color, folder_id, position) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(c.number)
        .bind(c.name)
        .bind(c.start_date)
        .bind(c.end_date)
        .bind(c.icon)
        .bind(c.color)
        .bind(c.folder_id)
        .bind(c.position)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

async fn restore_projects(tx: &mut Tx<'_>, dir: &Path) -> Result<()> {
    for path in files_in(dir, "projects", "yaml")? {
        let p: ProjectFile = read_yaml(&path)?;
        let id = sqlx::query(
            "INSERT INTO projects (slug, name, description, doc_link, start_date, target_date,
               archived_at, created_at, updated_at, icon, color, status_id, priority, properties, folder_id, position)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
               COALESCE((SELECT id FROM project_statuses WHERE name = ?),
                        (SELECT id FROM project_statuses ORDER BY position, id LIMIT 1)), ?, ?, ?, ?)",
        )
        .bind(&p.slug)
        .bind(p.name)
        .bind(p.description)
        .bind(p.doc_link)
        .bind(p.start_date)
        .bind(p.target_date)
        .bind(p.archived_at)
        .bind(p.created_at)
        .bind(p.updated_at.unwrap_or(p.created_at))
        .bind(p.icon)
        .bind(p.color)
        .bind(p.status)
        .bind(p.priority)
        .bind(Value::Object(p.properties).to_string())
        .bind(p.folder_id)
        .bind(p.position)
        .execute(&mut **tx)
        .await?
        .last_insert_rowid();
        for label in p.labels {
            sqlx::query(
                "INSERT INTO project_labels (project_id, label_id)
                 VALUES (?, (SELECT id FROM labels WHERE name = ?))",
            )
            .bind(id)
            .bind(label)
            .execute(&mut **tx)
            .await?;
        }
        for m in p.milestones {
            sqlx::query("INSERT INTO milestones (project_id, name, target_date) VALUES (?, ?, ?)")
                .bind(id)
                .bind(m.name)
                .bind(m.target_date)
                .execute(&mut **tx)
                .await?;
        }
    }
    Ok(())
}

async fn restore_properties(tx: &mut Tx<'_>, dir: &Path) -> Result<()> {
    let props: Vec<PropertyFile> = read_yaml(&dir.join("properties.yaml"))?;
    for p in props {
        sqlx::query(
            "INSERT INTO property_defs (key, name, kind, options, project_id, position, applies_to)
             VALUES (?, ?, ?, ?, (SELECT id FROM projects WHERE slug = ?), ?, ?)",
        )
        .bind(p.key)
        .bind(p.name)
        .bind(p.kind.to_string())
        .bind(p.options.to_string())
        .bind(p.project)
        .bind(p.position)
        .bind(p.applies_to)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn restore_views(tx: &mut Tx<'_>, dir: &Path) -> Result<()> {
    let views: Vec<ViewFile> = read_yaml(&dir.join("views.yaml"))?;
    // The migration seeds default views; the export is the whole list, so it replaces them.
    sqlx::query("DELETE FROM views").execute(&mut **tx).await?;
    for v in views {
        // Parsing here rejects a definition the app could not run.
        serde_json::from_value::<ViewDef>(v.definition.clone()).map_err(invalid)?;
        sqlx::query(
            "INSERT INTO views (name, entity, definition, position, icon, color, template_id, folder_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(v.name)
        .bind(v.entity)
        .bind(v.definition.to_string())
        .bind(v.position)
        .bind(v.icon)
        .bind(v.color)
        .bind(v.template_id)
        .bind(v.folder_id)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn restore_view_folders(tx: &mut Tx<'_>, dir: &Path) -> Result<()> {
    let folders: Vec<ViewFolderFile> = read_list(&dir.join("view_folders.yaml"))?;
    for f in folders {
        sqlx::query("INSERT INTO view_folders (id, name, position, section, parent_id) VALUES (?, ?, ?, ?, ?)")
            .bind(f.id)
            .bind(f.name)
            .bind(f.position)
            .bind(f.section)
            .bind(f.parent_id)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

async fn restore_view_templates(tx: &mut Tx<'_>, dir: &Path) -> Result<()> {
    let templates: Vec<ViewTemplateFile> = read_list(&dir.join("view_templates.yaml"))?;
    for t in templates {
        serde_json::from_value::<ViewDef>(t.definition.clone()).map_err(invalid)?;
        sqlx::query(
            "INSERT INTO view_templates (id, name, definition, position, icon, color) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(t.id)
        .bind(t.name)
        .bind(t.definition.to_string())
        .bind(t.position)
        .bind(t.icon)
        .bind(t.color)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn restore_issue_templates(tx: &mut Tx<'_>, dir: &Path) -> Result<()> {
    let templates: Vec<IssueTemplateFile> = read_list(&dir.join("issue_templates.yaml"))?;
    for t in templates {
        sqlx::query(
            "INSERT INTO issue_templates (name, slug, icon, color, body) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(t.name)
        .bind(t.slug)
        .bind(t.icon)
        .bind(t.color)
        .bind(t.body.to_string())
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn restore_import_log(tx: &mut Tx<'_>, dir: &Path) -> Result<()> {
    let log: Vec<ImportLogRow> = read_list(&dir.join("import_log.yaml"))?;
    for l in log {
        sqlx::query(
            "INSERT INTO import_log (external_id, source, issue_id, imported_at, source_deleted)
             VALUES (?, ?, (SELECT id FROM issues WHERE number = ?), ?, ?)",
        )
        .bind(l.external_id)
        .bind(l.source)
        .bind(l.issue)
        .bind(l.imported_at)
        .bind(l.source_deleted)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn restore_pins(tx: &mut Tx<'_>, dir: &Path) -> Result<()> {
    let pins: Vec<PinRow> = read_yaml(&dir.join("pins.yaml"))?;
    for p in pins {
        sqlx::query(
            "INSERT INTO pins (kind, target, title, position, folder_id) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(p.kind)
        .bind(p.target)
        .bind(p.title)
        .bind(p.position)
        .bind(p.folder_id)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

fn read_issues(dir: &Path, prefix: &str) -> Result<Vec<IssueEntry>> {
    files_in(dir, "issues", "md")?
        .iter()
        .map(|path| {
            let (front, description, comments) = parse_issue(&read_plain(path)?)?;
            Ok((number_of(prefix, &front.key)?, front, description, comments))
        })
        .collect()
}

/// The number in a key such as `ABC-12`; the prefix must be the database's.
fn number_of(prefix: &str, key: &str) -> Result<i64> {
    key.strip_prefix(prefix)
        .and_then(|rest| rest.strip_prefix('-'))
        .and_then(|n| n.parse().ok())
        .ok_or_else(|| Error::Invalid(format!("{key:?} is not an issue key for prefix {prefix}")))
}

/// Inserts the issue row; the parent and relations are linked in a second pass.
async fn insert_issue(
    tx: &mut Tx<'_>,
    number: i64,
    f: &IssueFront,
    description: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO issues (number, title, description, status_id, priority, project_id,
           milestone_id, cycle_id, start_date, due_date, due_time_utc, due_tz, estimate_minutes,
           points, properties, created_at, updated_at, completed_at, archived_at)
         VALUES (?, ?, ?,
           (SELECT id FROM statuses WHERE name = ?), ?,
           (SELECT id FROM projects WHERE slug = ?),
           (SELECT m.id FROM milestones m JOIN projects p ON p.id = m.project_id
              WHERE p.slug = ? AND m.name = ? ORDER BY m.id LIMIT 1),
           (SELECT id FROM cycles WHERE number = ?),
           ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(number)
    .bind(&f.title)
    .bind(description)
    .bind(&f.status)
    .bind(f.priority)
    .bind(&f.project)
    .bind(&f.project)
    .bind(&f.milestone)
    .bind(f.cycle)
    .bind(f.start_date)
    .bind(f.due_date)
    .bind(f.due_time)
    .bind(&f.due_tz)
    .bind(f.estimate_minutes)
    .bind(f.points)
    .bind(serde_json::to_string(&f.properties).map_err(invalid)?)
    .bind(f.created_at)
    .bind(f.updated_at)
    .bind(f.completed_at)
    .bind(f.archived_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn link_issue(
    tx: &mut Tx<'_>,
    prefix: &str,
    number: i64,
    f: &IssueFront,
    comments: &[CommentText],
) -> Result<()> {
    if let Some(parent) = &f.parent {
        sqlx::query(
            "UPDATE issues SET parent_id = (SELECT id FROM issues WHERE number = ?) WHERE number = ?",
        )
        .bind(number_of(prefix, parent)?)
        .bind(number)
        .execute(&mut **tx)
        .await?;
    }
    for name in &f.labels {
        let label: i64 = sqlx::query_scalar("SELECT id FROM labels WHERE name = ?")
            .bind(name)
            .fetch_one(&mut **tx)
            .await?;
        sqlx::query(
            "INSERT INTO issue_labels (issue_id, label_id)
             VALUES ((SELECT id FROM issues WHERE number = ?), ?)",
        )
        .bind(number)
        .bind(label)
        .execute(&mut **tx)
        .await?;
    }
    let legacy = [
        ("blocks", &f.blocks),
        ("related", &f.related),
        ("duplicate", &f.duplicate),
    ];
    let others = f.relations.iter().map(|(k, v)| (k.as_str(), v));
    for (kind, targets) in legacy.into_iter().chain(others) {
        for target in targets {
            sqlx::query(
                "INSERT INTO relations (from_id, to_id, kind) VALUES (
                   (SELECT id FROM issues WHERE number = ?), (SELECT id FROM issues WHERE number = ?), ?)",
            )
            .bind(number)
            .bind(number_of(prefix, target)?)
            .bind(kind)
            .execute(&mut **tx)
            .await?;
        }
    }
    for c in comments {
        insert_comment(tx, number, c).await?;
    }
    for b in &f.work_blocks {
        sqlx::query(
            "INSERT INTO work_blocks (issue_id, start_utc, end_utc)
             VALUES ((SELECT id FROM issues WHERE number = ?), ?, ?)",
        )
        .bind(number)
        .bind(b.start)
        .bind(b.end)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn insert_comment(tx: &mut Tx<'_>, number: i64, c: &CommentText) -> Result<()> {
    sqlx::query(
        "INSERT INTO comments (issue_id, body, created_at, updated_at)
         VALUES ((SELECT id FROM issues WHERE number = ?), ?, ?, ?)",
    )
    .bind(number)
    .bind(&c.body)
    .bind(c.created_at)
    .bind(c.updated_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Brings the exported images back: each is read from `attachments/`, checked by its bytes and hash, and
/// stored under its old id, because markdown points at the id. The file name comes from the hash, not the manifest.
pub(crate) async fn restore_attachments(db: &Db, tx: &mut Tx<'_>, dir: &Path) -> Result<()> {
    let list: Vec<AttachmentFile> = read_list(&dir.join("attachments.yaml"))?;
    for a in list {
        let bytes = read_image(dir, &a.sha256)?;
        let img = attachments::inspect(&bytes)?;
        if img.sha256 != a.sha256 {
            return Err(invalid(format!(
                "attachment {} does not match its hash",
                a.id
            )));
        }
        attachments::write_file(db, &img, &bytes)?;
        sqlx::query("INSERT OR IGNORE INTO attachments (id, sha256, file, mime, bytes, created_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(a.id)
            .bind(&img.sha256)
            .bind(img.file())
            .bind(img.mime)
            .bind(i64::try_from(bytes.len()).map_err(invalid)?)
            .bind(a.created_at)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

/// The bytes of `attachments/<sha256>.<ext>`; the hash is checked to be plain hex so it cannot name another path.
fn read_image(dir: &Path, sha256: &str) -> Result<Vec<u8>> {
    if sha256.len() != 64 || !sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(invalid(format!("{sha256:?} is not an image hash")));
    }
    for ext in ["png", "jpg", "gif", "webp"] {
        let path = dir.join("attachments").join(format!("{sha256}.{ext}"));
        if fs::symlink_metadata(&path).is_ok_and(|m| m.is_file()) {
            return Ok(fs::read(path)?);
        }
    }
    Err(invalid(format!(
        "attachments/{sha256} is missing from the export"
    )))
}
