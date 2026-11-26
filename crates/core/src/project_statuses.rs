//! The status list projects choose from, separate from the issue statuses.

use serde_json::json;
use sqlx::SqliteConnection;

use crate::{
    ChangeSource, Db, Error, ProjectStatus, ProjectStatusCategory, ProjectStatusId, Result,
    history,
    palette::{self, Look},
    rows,
};

fn category_str(c: ProjectStatusCategory) -> &'static str {
    match c {
        ProjectStatusCategory::Planned => "planned",
        ProjectStatusCategory::Started => "started",
        ProjectStatusCategory::Paused => "paused",
        ProjectStatusCategory::Completed => "completed",
        ProjectStatusCategory::Cancelled => "cancelled",
    }
}

fn parse_category(s: &str) -> Result<ProjectStatusCategory> {
    Ok(match s {
        "planned" => ProjectStatusCategory::Planned,
        "started" => ProjectStatusCategory::Started,
        "paused" => ProjectStatusCategory::Paused,
        "completed" => ProjectStatusCategory::Completed,
        "cancelled" => ProjectStatusCategory::Cancelled,
        other => return Err(Error::Invalid(format!("project status category {other}"))),
    })
}

fn check_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        return Err(Error::Invalid("project status name is empty".into()));
    }
    Ok(())
}

fn unique_to_conflict(e: sqlx::Error, name: &str) -> Error {
    if e.as_database_error()
        .is_some_and(sqlx::error::DatabaseError::is_unique_violation)
    {
        return Error::Conflict(format!("project status {name} already exists"));
    }
    e.into()
}

async fn fetch_all(conn: &mut SqliteConnection) -> Result<Vec<ProjectStatus>> {
    let rows = sqlx::query_as::<_, (i64, String, String, i32, String, Option<String>)>(
        "SELECT id, name, category, position, color, icon FROM project_statuses ORDER BY position, id",
    )
    .fetch_all(conn)
    .await?;
    rows.into_iter()
        .map(|(id, name, category, position, color, icon)| {
            Ok(ProjectStatus {
                id: ProjectStatusId(id),
                name,
                category: parse_category(&category)?,
                position,
                color_hex: palette::resolve(&color),
                color,
                icon,
            })
        })
        .collect()
}

async fn fetch(conn: &mut SqliteConnection, id: ProjectStatusId) -> Result<ProjectStatus> {
    fetch_all(conn)
        .await?
        .into_iter()
        .find(|s| s.id == id)
        .ok_or_else(|| Error::NotFound(format!("project status {}", id.0)))
}

/// # Errors
///
/// Fails on a database error.
pub async fn list(db: &Db) -> Result<Vec<ProjectStatus>> {
    fetch_all(&mut *db.pool.acquire().await?).await
}

/// # Errors
///
/// `Invalid` for an empty name or a bad color or icon, `Conflict` for a duplicate name.
pub async fn create(
    db: &Db,
    name: &str,
    category: ProjectStatusCategory,
    color: &str,
    icon: Option<String>,
    src: ChangeSource,
) -> Result<ProjectStatus> {
    check_name(name)?;
    let color = palette::key_for(color)?;
    if let Some(icon) = &icon {
        palette::check_icon(icon)?;
    }
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let id = sqlx::query(
        "INSERT INTO project_statuses (name, category, position, color, icon)
         VALUES (?, ?, (SELECT COALESCE(MAX(position), -1) + 1 FROM project_statuses), ?, ?)",
    )
    .bind(name)
    .bind(category_str(category))
    .bind(color)
    .bind(icon)
    .execute(&mut *tx)
    .await
    .map_err(|e| unique_to_conflict(e, name))?
    .last_insert_rowid();
    rows::created(&mut tx, src, "project_statuses", id).await?;
    let created = json!(name);
    let edit = history::Edit {
        field: "created",
        old: None,
        new: Some(&created),
    };
    history::record(&mut tx, src, "project_status", &id.to_string(), &edit).await?;
    let status = fetch(&mut tx, ProjectStatusId(id)).await?;
    tx.commit().await?;
    Ok(status)
}

/// A `None` field keeps its current value.
///
/// # Errors
///
/// `NotFound` for an unknown id, `Invalid` for an empty name or bad color or icon, `Conflict` for a duplicate name.
pub async fn update(
    db: &Db,
    id: ProjectStatusId,
    name: Option<String>,
    look: Look,
    src: ChangeSource,
) -> Result<ProjectStatus> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let old = fetch(&mut tx, id).await?;
    if let Some(n) = &name {
        check_name(n)?;
    }
    let color = match &look.color {
        Some(c) => palette::key_for(c)?.to_owned(),
        None => old.color.clone(),
    };
    if let Some(i) = &look.icon {
        palette::check_icon(i)?;
    }
    let name = name.unwrap_or(old.name);
    let icon = look.icon.or(old.icon);
    let before = rows::by_key(&mut tx, "project_statuses", id.0).await?;
    sqlx::query("UPDATE project_statuses SET name = ?, color = ?, icon = ? WHERE id = ?")
        .bind(&name)
        .bind(color)
        .bind(icon)
        .bind(id.0)
        .execute(&mut *tx)
        .await
        .map_err(|e| unique_to_conflict(e, &name))?;
    rows::changed(&mut tx, src, before).await?;
    let status = fetch(&mut tx, id).await?;
    tx.commit().await?;
    Ok(status)
}

/// Projects in the deleted status move to `move_projects_to`.
///
/// # Errors
///
/// `Invalid` when both ids are the same, `NotFound` when either status does not exist.
pub async fn delete(
    db: &Db,
    id: ProjectStatusId,
    move_projects_to: ProjectStatusId,
    src: ChangeSource,
) -> Result<()> {
    if id == move_projects_to {
        return Err(Error::Invalid(
            "cannot move projects into the deleted status".into(),
        ));
    }
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    fetch(&mut tx, id).await?;
    fetch(&mut tx, move_projects_to).await?;
    let moved = rows::snap(&mut tx, "projects", "status_id = ?", id.0).await?;
    sqlx::query("UPDATE projects SET status_id = ? WHERE status_id = ?")
        .bind(move_projects_to.0)
        .bind(id.0)
        .execute(&mut *tx)
        .await?;
    let gone = rows::by_key(&mut tx, "project_statuses", id.0).await?;
    sqlx::query("DELETE FROM project_statuses WHERE id = ?")
        .bind(id.0)
        .execute(&mut *tx)
        .await?;
    // Children first, so undo (newest first) restores the status before the projects that point at it.
    rows::changed(&mut tx, src, moved).await?;
    rows::changed(&mut tx, src, gone).await?;
    tx.commit().await?;
    Ok(())
}

/// Sets every status's position to its index in `ids_in_order`; one history step.
///
/// # Errors
///
/// `Invalid` unless the list names every status exactly once.
pub async fn reorder(db: &Db, ids_in_order: &[ProjectStatusId], src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let mut have: Vec<i64> = fetch_all(&mut tx).await?.iter().map(|s| s.id.0).collect();
    let mut want: Vec<i64> = ids_in_order.iter().map(|i| i.0).collect();
    have.sort_unstable();
    want.sort_unstable();
    if have != want {
        return Err(Error::Invalid(
            "reorder must list every project status exactly once".into(),
        ));
    }
    let before = rows::all(&mut tx, "project_statuses").await?;
    for (position, id) in (0_i64..).zip(ids_in_order) {
        sqlx::query("UPDATE project_statuses SET position = ? WHERE id = ?")
            .bind(position)
            .bind(id.0)
            .execute(&mut *tx)
            .await?;
    }
    rows::changed(&mut tx, src, before).await?;
    tx.commit().await?;
    Ok(())
}
