use chrono::{DateTime, NaiveDate, Utc};
use serde_json::{Value, json};
use sqlx::SqliteConnection;

use crate::{
    ChangeSource, Db, Error, Milestone, MilestoneId, Project, ProjectId, Result,
    history::{self, Edit},
};

/// Slugs appear in routes such as `/project/<slug>`, so they stay URL-safe.
fn valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 40
        && slug
            .bytes()
            .enumerate()
            .all(|(i, b)| b.is_ascii_lowercase() || b.is_ascii_digit() || (i > 0 && b == b'-'))
}

/// Only `obsidian://` and `https://` links, or a vault-relative `.md` path. Anything with a
/// `:` in a path (`javascript:`, `file:`) or escaping the vault (`/`, `..`) is refused.
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
            "doc link `{link}` must be an obsidian:// or https:// URL, or a vault-relative .md path"
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

async fn find(tx: &mut SqliteConnection, id: ProjectId) -> Result<Project> {
    let r = sqlx::query!(
        r#"SELECT id AS "id!: i64", slug, name, description, doc_link,
                  start_date AS "start_date: NaiveDate", target_date AS "target_date: NaiveDate",
                  archived_at AS "archived_at: DateTime<Utc>"
           FROM projects WHERE id = ?"#,
        id.0
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| Error::NotFound(format!("project {}", id.0)))?;
    Ok(Project {
        id: ProjectId(r.id),
        slug: r.slug,
        name: r.name,
        description: r.description,
        doc_link: r.doc_link,
        start_date: r.start_date,
        target_date: r.target_date,
        archived_at: r.archived_at,
    })
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn list(db: &Db) -> Result<Vec<Project>> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!: i64", slug, name, description, doc_link,
                  start_date AS "start_date: NaiveDate", target_date AS "target_date: NaiveDate",
                  archived_at AS "archived_at: DateTime<Utc>"
           FROM projects ORDER BY id"#
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| Project {
            id: ProjectId(r.id),
            slug: r.slug,
            name: r.name,
            description: r.description,
            doc_link: r.doc_link,
            start_date: r.start_date,
            target_date: r.target_date,
            archived_at: r.archived_at,
        })
        .collect())
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn create(db: &Db, slug: &str, name: &str, src: ChangeSource) -> Result<Project> {
    validate(slug, None)?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let now = Utc::now();
    let id = sqlx::query!(
        "INSERT INTO projects (slug, name, created_at) VALUES (?, ?, ?)",
        slug,
        name,
        now
    )
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();
    let created = json!(slug);
    let edit = Edit {
        field: "created",
        old: None,
        new: Some(&created),
    };
    history::record(&mut tx, src, "project", &id.to_string(), &edit).await?;
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
    edit(tx, src, target, "archived_at", before, after).await
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn update(db: &Db, project: Project, src: ChangeSource) -> Result<Project> {
    validate(&project.slug, project.doc_link.as_deref())?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let old = find(&mut tx, project.id).await?;
    record_edits(&mut tx, src, &old, &project).await?;
    sqlx::query!(
        "UPDATE projects SET slug = ?, name = ?, description = ?, doc_link = ?, start_date = ?,
                target_date = ?, archived_at = ? WHERE id = ?",
        project.slug,
        project.name,
        project.description,
        project.doc_link,
        project.start_date,
        project.target_date,
        project.archived_at,
        project.id.0
    )
    .execute(&mut *tx)
    .await?;
    // Project pins are keyed by slug, so a rename must carry them along.
    sqlx::query("UPDATE pins SET target = ? WHERE kind = 'project' AND target = ?")
        .bind(&project.slug)
        .bind(&old.slug)
        .execute(&mut *tx)
        .await?;
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
    sqlx::query!("DELETE FROM projects WHERE id = ?", id.0)
        .execute(&mut *tx)
        .await?;
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
    sqlx::query!("DELETE FROM milestones WHERE id = ?", mid)
        .execute(&mut *tx)
        .await?;
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
