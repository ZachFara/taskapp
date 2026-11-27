use chrono::{NaiveDate, Utc};
use serde_json::{Value, json};
use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};

use crate::{
    ChangeSource, Db, Error, LabelId, Milestone, MilestoneId, Project, ProjectId, ProjectStatusId,
    PropertyAppliesTo, Result,
    history::{self, Edit},
    palette, properties, rows,
};

/// Icon and color are checked the same way on create and update.
fn check_look(icon: Option<&str>, color: Option<&str>) -> Result<Option<String>> {
    if let Some(icon) = icon {
        palette::check_icon(icon)?;
    }
    color
        .map(|c| palette::key_for(c).map(str::to_owned))
        .transpose()
}

/// Slugs appear in routes such as `/project/<slug>`, so they stay URL-safe.
fn valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 40
        && slug
            .bytes()
            .enumerate()
            .all(|(i, b)| b.is_ascii_lowercase() || b.is_ascii_digit() || (i > 0 && b == b'-'))
}

/// Only `obsidian://` and `https://` links, or a `.md` path inside the notes folder. Anything with a
/// `:` in a path (`javascript:`, `file:`) or escaping the notes folder (`/`, `..`) is refused.
fn valid_doc_link(link: &str) -> bool {
    if link.starts_with("obsidian://") || link.starts_with("https://") {
        return true;
    }
    std::path::Path::new(link).extension() == Some("md".as_ref())
        && !link.contains(':')
        && !link.starts_with('/')
        && !link.split('/').any(|part| part == "..")
}

fn validate(slug: &str, doc_link: Option<&str>) -> Result<()> {
    if !valid_slug(slug) {
        return Err(Error::Invalid(format!(
            "slug `{slug}` must be 1 to 40 lowercase letters, digits or hyphens, not starting with a hyphen"
        )));
    }
    if let Some(link) = doc_link.filter(|l| !valid_doc_link(l)) {
        return Err(Error::Invalid(format!(
            "doc link `{link}` must be an obsidian:// or https:// URL, or a .md path inside the notes folder"
        )));
    }
    Ok(())
}

/// Records one field's change, skipping unchanged values.
async fn edit(
    tx: &mut SqliteConnection,
    src: ChangeSource,
    (entity, id): (&str, i64),
    field: &str,
    old: Value,
    new: Value,
) -> Result<()> {
    if old == new {
        return Ok(());
    }
    let edit = Edit {
        field,
        old: Some(&old),
        new: Some(&new),
    };
    history::record(tx, src, entity, &id.to_string(), &edit).await
}

const COLS: &str =
    "id, slug, name, description, doc_link, start_date, target_date, archived_at, icon, color,
    status_id, priority, properties";

fn project_from_row(r: &SqliteRow, label_ids: Vec<LabelId>) -> Result<Project> {
    let props: String = r.get("properties");
    Ok(Project {
        id: ProjectId(r.get("id")),
        slug: r.get("slug"),
        name: r.get("name"),
        description: r.get("description"),
        doc_link: r.get("doc_link"),
        start_date: r.get("start_date"),
        target_date: r.get("target_date"),
        archived_at: r.get("archived_at"),
        icon: r.get("icon"),
        color: r.get("color"),
        status_id: ProjectStatusId(r.get("status_id")),
        priority: r.get("priority"),
        label_ids,
        properties: serde_json::from_str(&props).map_err(|e| Error::Invalid(e.to_string()))?,
    })
}

async fn label_ids(tx: &mut SqliteConnection, id: i64) -> Result<Vec<LabelId>> {
    let ids: Vec<i64> = sqlx::query_scalar(
        "SELECT label_id FROM project_labels WHERE project_id = ? ORDER BY label_id",
    )
    .bind(id)
    .fetch_all(tx)
    .await?;
    Ok(ids.into_iter().map(LabelId).collect())
}

pub(crate) async fn find(tx: &mut SqliteConnection, id: ProjectId) -> Result<Project> {
    let r = sqlx::query(&format!("SELECT {COLS} FROM projects WHERE id = ?"))
        .bind(id.0)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| Error::NotFound(format!("project {}", id.0)))?;
    let labels = label_ids(tx, id.0).await?;
    project_from_row(&r, labels)
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn list(db: &Db) -> Result<Vec<Project>> {
    let mut conn = db.pool.acquire().await?;
    let rows = sqlx::query(&format!("SELECT {COLS} FROM projects ORDER BY id"))
        .fetch_all(&mut *conn)
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in &rows {
        let labels = label_ids(&mut conn, r.get("id")).await?;
        out.push(project_from_row(r, labels)?);
    }
    Ok(out)
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn create(
    db: &Db,
    slug: &str,
    name: &str,
    icon: Option<String>,
    color: Option<String>,
    src: ChangeSource,
) -> Result<Project> {
    validate(slug, None)?;
    let color = check_look(icon.as_deref(), color.as_deref())?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let now = Utc::now();
    let id = sqlx::query!(
        "INSERT INTO projects (slug, name, created_at, icon, color) VALUES (?, ?, ?, ?, ?)",
        slug,
        name,
        now,
        icon,
        color
    )
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();
    rows::created(&mut tx, src, "projects", id).await?;
    let created = json!(slug);
    let edit = Edit {
        field: "created",
        old: None,
        new: Some(&created),
    };
    history::record(&mut tx, src, "project", &id.to_string(), &edit).await?;
    let status_id: i64 = sqlx::query_scalar("SELECT status_id FROM projects WHERE id = ?")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Project {
        id: ProjectId(id),
        slug: slug.to_owned(),
        name: name.to_owned(),
        description: None,
        doc_link: None,
        start_date: None,
        target_date: None,
        archived_at: None,
        icon,
        color,
        status_id: ProjectStatusId(status_id),
        priority: 0,
        label_ids: vec![],
        properties: serde_json::Map::new(),
    })
}

async fn record_edits(
    tx: &mut SqliteConnection,
    src: ChangeSource,
    old: &Project,
    new: &Project,
) -> Result<()> {
    let target = ("project", old.id.0);
    edit(tx, src, target, "slug", json!(old.slug), json!(new.slug)).await?;
    edit(tx, src, target, "name", json!(old.name), json!(new.name)).await?;
    let (before, after) = (json!(old.description), json!(new.description));
    edit(tx, src, target, "description", before, after).await?;
    let (before, after) = (json!(old.doc_link), json!(new.doc_link));
    edit(tx, src, target, "doc_link", before, after).await?;
    let (before, after) = (json!(old.start_date), json!(new.start_date));
    edit(tx, src, target, "start_date", before, after).await?;
    let (before, after) = (json!(old.target_date), json!(new.target_date));
    edit(tx, src, target, "target_date", before, after).await?;
    let (before, after) = (json!(old.archived_at), json!(new.archived_at));
    edit(tx, src, target, "archived_at", before, after).await?;
    let (before, after) = (json!(old.icon), json!(new.icon));
    edit(tx, src, target, "icon", before, after).await?;
    let (before, after) = (json!(old.color), json!(new.color));
    edit(tx, src, target, "color", before, after).await?;
    let (before, after) = (json!(old.status_id.0), json!(new.status_id.0));
    edit(tx, src, target, "status_id", before, after).await?;
    let (before, after) = (json!(old.priority), json!(new.priority));
    edit(tx, src, target, "priority", before, after).await?;
    let (before, after) = (json!(old.label_ids), json!(new.label_ids));
    edit(tx, src, target, "label_ids", before, after).await?;
    let (before, after) = (json!(old.properties), json!(new.properties));
    edit(tx, src, target, "properties", before, after).await
}

/// Priority, status and every custom property value must exist and fit.
async fn check_fields(tx: &mut SqliteConnection, p: &Project) -> Result<()> {
    if p.priority > 4 {
        return Err(Error::Invalid("priority is 0 to 4".into()));
    }
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM project_statuses WHERE id = ?")
        .bind(p.status_id.0)
        .fetch_one(&mut *tx)
        .await?;
    if n == 0 {
        return Err(Error::NotFound(format!("project status {}", p.status_id.0)));
    }
    for (key, value) in &p.properties {
        let def = properties::get_def(tx, key).await?;
        if def.applies_to == PropertyAppliesTo::Issues {
            return Err(Error::Invalid(format!("{key} does not apply to projects")));
        }
        properties::validate(&def, value)?;
    }
    Ok(())
}

/// Brings `project_labels` to `want`: removed rows are deleted, added ones inserted, each recorded for undo.
async fn set_labels(
    tx: &mut SqliteConnection,
    src: ChangeSource,
    id: i64,
    want: &[LabelId],
) -> Result<()> {
    let have = label_ids(tx, id).await?;
    let before = rows::snap(tx, "project_labels", "project_id = ?", id).await?;
    for l in have.iter().filter(|l| !want.contains(l)) {
        sqlx::query("DELETE FROM project_labels WHERE project_id = ? AND label_id = ?")
            .bind(id)
            .bind(l.0)
            .execute(&mut *tx)
            .await?;
    }
    rows::changed(tx, src, before).await?;
    for l in want.iter().filter(|l| !have.contains(l)) {
        let row = sqlx::query(
            "INSERT OR IGNORE INTO project_labels (project_id, label_id) VALUES (?, ?)",
        )
        .bind(id)
        .bind(l.0)
        .execute(&mut *tx)
        .await?;
        if row.rows_affected() == 1 {
            rows::created(tx, src, "project_labels", row.last_insert_rowid()).await?;
        }
    }
    Ok(())
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn update(db: &Db, mut project: Project, src: ChangeSource) -> Result<Project> {
    validate(&project.slug, project.doc_link.as_deref())?;
    project.color = check_look(project.icon.as_deref(), project.color.as_deref())?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let old = find(&mut tx, project.id).await?;
    check_fields(&mut tx, &project).await?;
    project.label_ids.sort_by_key(|l| l.0);
    project.label_ids.dedup();
    let before = rows::by_key(&mut tx, "projects", project.id.0).await?;
    let pins = rows::snap(
        &mut tx,
        "pins",
        "kind = 'project' AND target = ?",
        &old.slug,
    )
    .await?;
    record_edits(&mut tx, src, &old, &project).await?;
    sqlx::query(
        "UPDATE projects SET slug = ?, name = ?, description = ?, doc_link = ?, start_date = ?,
                target_date = ?, archived_at = ?, icon = ?, color = ?, status_id = ?, priority = ?,
                properties = ? WHERE id = ?",
    )
    .bind(&project.slug)
    .bind(&project.name)
    .bind(&project.description)
    .bind(&project.doc_link)
    .bind(project.start_date)
    .bind(project.target_date)
    .bind(project.archived_at)
    .bind(&project.icon)
    .bind(&project.color)
    .bind(project.status_id.0)
    .bind(project.priority)
    .bind(serde_json::to_string(&project.properties).map_err(|e| Error::Invalid(e.to_string()))?)
    .bind(project.id.0)
    .execute(&mut *tx)
    .await?;
    set_labels(&mut tx, src, project.id.0, &project.label_ids).await?;
    // Project pins are keyed by slug, so a rename must carry them along.
    sqlx::query("UPDATE pins SET target = ? WHERE kind = 'project' AND target = ?")
        .bind(&project.slug)
        .bind(&old.slug)
        .execute(&mut *tx)
        .await?;
    rows::changed(&mut tx, src, pins).await?;
    rows::changed(&mut tx, src, before).await?;
    tx.commit().await?;
    Ok(project)
}

/// Clears the project and any of its milestones from every issue that has them, recording each.
async fn unlink_issues(tx: &mut SqliteConnection, src: ChangeSource, pid: i64) -> Result<()> {
    let affected = sqlx::query!(
        r#"SELECT id AS "id!: i64", project_id, milestone_id FROM issues WHERE project_id = ?
           OR milestone_id IN (SELECT id FROM milestones WHERE project_id = ?)"#,
        pid,
        pid
    )
    .fetch_all(&mut *tx)
    .await?;
    for issue in affected {
        let target = ("issue", issue.id);
        let (before, after) = (json!(issue.project_id), Value::Null);
        edit(tx, src, target, "project_id", before, after).await?;
        let (before, after) = (json!(issue.milestone_id), Value::Null);
        edit(tx, src, target, "milestone_id", before, after).await?;
    }
    sqlx::query!(
        "UPDATE issues SET project_id = NULL, milestone_id = NULL WHERE project_id = ?
         OR milestone_id IN (SELECT id FROM milestones WHERE project_id = ?)",
        pid,
        pid
    )
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Issues keep existing; they lose the project and any of its milestones.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn delete(db: &Db, id: ProjectId, src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let old = find(&mut tx, id).await?;
    let defs: Vec<String> =
        sqlx::query_scalar("SELECT key FROM property_defs WHERE project_id = ? ORDER BY key")
            .bind(id.0)
            .fetch_all(&mut *tx)
            .await?;
    if !defs.is_empty() {
        return Err(Error::Conflict(format!(
            "project {} has property definitions: {}",
            old.slug,
            defs.join(", ")
        )));
    }
    unlink_issues(&mut tx, src, id.0).await?;
    let milestones = rows::snap(&mut tx, "milestones", "project_id = ?", id.0).await?;
    let labels = rows::snap(&mut tx, "project_labels", "project_id = ?", id.0).await?;
    let before = rows::by_key(&mut tx, "projects", id.0).await?;
    sqlx::query!("DELETE FROM projects WHERE id = ?", id.0)
        .execute(&mut *tx)
        .await?;
    rows::changed(&mut tx, src, labels).await?;
    rows::changed(&mut tx, src, milestones).await?;
    rows::changed(&mut tx, src, before).await?;
    let deleted = json!(old.slug);
    let edit = Edit {
        field: "deleted",
        old: Some(&deleted),
        new: None,
    };
    history::record(&mut tx, src, "project", &id.0.to_string(), &edit).await?;
    tx.commit().await?;
    Ok(())
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn list_milestones(db: &Db, project: ProjectId) -> Result<Vec<Milestone>> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!: i64", project_id, name, target_date AS "target_date: NaiveDate"
           FROM milestones WHERE project_id = ? ORDER BY id"#,
        project.0
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| Milestone {
            id: MilestoneId(r.id),
            project_id: ProjectId(r.project_id),
            name: r.name,
            target_date: r.target_date,
        })
        .collect())
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn create_milestone(
    db: &Db,
    project: ProjectId,
    name: &str,
    target_date: Option<NaiveDate>,
    src: ChangeSource,
) -> Result<Milestone> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let id = sqlx::query!(
        "INSERT INTO milestones (project_id, name, target_date) VALUES (?, ?, ?)",
        project.0,
        name,
        target_date
    )
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();
    rows::created(&mut tx, src, "milestones", id).await?;
    let created = json!(name);
    let edit = Edit {
        field: "created",
        old: None,
        new: Some(&created),
    };
    history::record(&mut tx, src, "milestone", &id.to_string(), &edit).await?;
    tx.commit().await?;
    Ok(Milestone {
        id: MilestoneId(id),
        project_id: project,
        name: name.to_owned(),
        target_date,
    })
}

/// The milestone's project is fixed at creation.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn update_milestone(
    db: &Db,
    milestone: Milestone,
    src: ChangeSource,
) -> Result<Milestone> {
    let id = milestone.id.0;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let old = sqlx::query!(
        r#"SELECT name, target_date AS "target_date: NaiveDate" FROM milestones WHERE id = ?"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| Error::NotFound(format!("milestone {id}")))?;
    let before_row = rows::by_key(&mut tx, "milestones", id).await?;
    let target = ("milestone", id);
    edit(
        &mut tx,
        src,
        target,
        "name",
        json!(old.name),
        json!(milestone.name),
    )
    .await?;
    let (before, after) = (json!(old.target_date), json!(milestone.target_date));
    edit(&mut tx, src, target, "target_date", before, after).await?;
    sqlx::query!(
        "UPDATE milestones SET name = ?, target_date = ? WHERE id = ?",
        milestone.name,
        milestone.target_date,
        id
    )
    .execute(&mut *tx)
    .await?;
    rows::changed(&mut tx, src, before_row).await?;
    tx.commit().await?;
    Ok(milestone)
}

/// Issues keep existing and lose the milestone.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn delete_milestone(db: &Db, id: MilestoneId, src: ChangeSource) -> Result<()> {
    let mid = id.0;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let name: String = sqlx::query_scalar("SELECT name FROM milestones WHERE id = ?")
        .bind(mid)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| Error::NotFound(format!("milestone {mid}")))?;
    let affected: Vec<i64> = sqlx::query_scalar("SELECT id FROM issues WHERE milestone_id = ?")
        .bind(mid)
        .fetch_all(&mut *tx)
        .await?;
    for issue_id in affected {
        let (before, after) = (json!(mid), Value::Null);
        edit(
            &mut tx,
            src,
            ("issue", issue_id),
            "milestone_id",
            before,
            after,
        )
        .await?;
    }
    sqlx::query!(
        "UPDATE issues SET milestone_id = NULL WHERE milestone_id = ?",
        mid
    )
    .execute(&mut *tx)
    .await?;
    let before = rows::by_key(&mut tx, "milestones", mid).await?;
    sqlx::query!("DELETE FROM milestones WHERE id = ?", mid)
        .execute(&mut *tx)
        .await?;
    rows::changed(&mut tx, src, before).await?;
    let deleted = json!(name);
    let edit = Edit {
        field: "deleted",
        old: Some(&deleted),
        new: None,
    };
    history::record(&mut tx, src, "milestone", &mid.to_string(), &edit).await?;
    tx.commit().await?;
    Ok(())
}

/// `(completed, total)` issue counts, for project pages. Archived issues are not counted.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn progress(db: &Db, id: ProjectId) -> Result<(u32, u32)> {
    let row = sqlx::query!(
        r#"SELECT count(*) AS "total!: i64",
                  coalesce(sum(s.category = 'completed'), 0) AS "completed!: i64"
           FROM issues i JOIN statuses s ON s.id = i.status_id
           WHERE i.project_id = ? AND i.archived_at IS NULL"#,
        id.0
    )
    .fetch_one(&db.pool)
    .await?;
    let to_u32 = |n: i64| u32::try_from(n).map_err(|e| Error::Invalid(e.to_string()));
    Ok((to_u32(row.completed)?, to_u32(row.total)?))
}
