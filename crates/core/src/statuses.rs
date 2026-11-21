use sqlx::SqliteConnection;

use crate::{ChangeSource, Db, Error, Result, Status, StatusCategory, StatusId, history};

pub(crate) fn category_str(c: StatusCategory) -> &'static str {
    match c {
        StatusCategory::Backlog => "backlog",
        StatusCategory::Unstarted => "unstarted",
        StatusCategory::Started => "started",
        StatusCategory::Completed => "completed",
        StatusCategory::Canceled => "canceled",
    }
}

pub(crate) fn parse_category(s: &str) -> Result<StatusCategory> {
    Ok(match s {
        "backlog" => StatusCategory::Backlog,
        "unstarted" => StatusCategory::Unstarted,
        "started" => StatusCategory::Started,
        "completed" => StatusCategory::Completed,
        "canceled" => StatusCategory::Canceled,
        other => return Err(Error::Invalid(format!("status category {other}"))),
    })
}

fn check_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        return Err(Error::Invalid("status name is empty".into()));
    }
    Ok(())
}

fn check_color(color: &str) -> Result<()> {
    let hex = color.strip_prefix('#').unwrap_or("");
    if hex.len() == 6 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Ok(());
    }
    Err(Error::Invalid(format!("color {color} is not #rrggbb")))
}

fn unique_to_conflict(e: sqlx::Error, name: &str) -> Error {
    if e.as_database_error()
        .is_some_and(sqlx::error::DatabaseError::is_unique_violation)
    {
        return Error::Conflict(format!("status {name} already exists"));
    }
    e.into()
}

async fn fetch(conn: &mut SqliteConnection, id: StatusId) -> Result<Status> {
    let r = sqlx::query!(
        r#"SELECT id AS "id!: i64", name, category, position, color FROM statuses WHERE id = ?"#,
        id.0
    )
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| Error::NotFound(format!("status {}", id.0)))?;
    Ok(Status {
        id: StatusId(r.id),
        name: r.name,
        category: parse_category(&r.category)?,
        position: i32::try_from(r.position).map_err(|e| Error::Invalid(e.to_string()))?,
        color: r.color,
    })
}

/// # Errors
///
/// Fails on a database error.
pub async fn list(db: &Db) -> Result<Vec<Status>> {
    let ids = sqlx::query_scalar!(r#"SELECT id AS "id!: i64" FROM statuses ORDER BY position, id"#)
        .fetch_all(&db.pool)
        .await?;
    let mut conn = db.pool.acquire().await?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        out.push(fetch(&mut conn, StatusId(id)).await?);
    }
    Ok(out)
}

/// # Errors
///
/// `Invalid` for an empty name or a color that is not `#rrggbb`, `Conflict` for a duplicate name.
pub async fn create(db: &Db, name: &str, category: StatusCategory, color: &str) -> Result<Status> {
    check_name(name)?;
    check_color(color)?;
    let cat = category_str(category);
    let mut conn = db.pool.acquire().await?;
    let id = sqlx::query!(
        "INSERT INTO statuses (name, category, position, color)
         VALUES (?, ?, (SELECT COALESCE(MAX(position), -1) + 1 FROM statuses), ?)",
        name,
        cat,
        color
    )
    .execute(&mut *conn)
    .await
    .map_err(|e| unique_to_conflict(e, name))?
    .last_insert_rowid();
    fetch(&mut conn, StatusId(id)).await
}

/// # Errors
///
/// `NotFound` for an unknown id, `Invalid` for an empty name or bad color, `Conflict` for a duplicate name.
pub async fn update(
    db: &Db,
    id: StatusId,
    name: Option<String>,
    color: Option<String>,
    position: Option<i32>,
) -> Result<Status> {
    let mut conn = db.pool.acquire().await?;
    let old = fetch(&mut conn, id).await?;
    if let Some(n) = &name {
        check_name(n)?;
    }
    if let Some(c) = &color {
        check_color(c)?;
    }
    let name = name.unwrap_or(old.name);
    let color = color.unwrap_or(old.color);
    let position = position.unwrap_or(old.position);
    sqlx::query!(
        "UPDATE statuses SET name = ?, color = ?, position = ? WHERE id = ?",
        name,
        color,
        position,
        id.0
    )
    .execute(&mut *conn)
    .await
    .map_err(|e| unique_to_conflict(e, &name))?;
    fetch(&mut conn, id).await
}

/// Issues in the deleted status move to `move_issues_to`.
///
/// shortcut: a moved issue keeps its `completed_at` even when the target is in another category,
/// upgrade when statuses can be deleted from the UI across categories.
///
/// # Errors
///
/// `Invalid` when both ids are the same, `NotFound` when either status does not exist.
pub async fn delete(db: &Db, id: StatusId, move_issues_to: StatusId) -> Result<()> {
    if id == move_issues_to {
        return Err(Error::Invalid(
            "cannot move issues into the deleted status".into(),
        ));
    }
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    fetch(&mut tx, id).await?;
    fetch(&mut tx, move_issues_to).await?;
    let moved = sqlx::query_scalar!(
        r#"SELECT id AS "id!: i64" FROM issues WHERE status_id = ?"#,
        id.0
    )
    .fetch_all(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE issues SET status_id = ? WHERE status_id = ?",
        move_issues_to.0,
        id.0
    )
    .execute(&mut *tx)
    .await?;
    for issue in moved {
        let (old, new) = (serde_json::json!(id.0), serde_json::json!(move_issues_to.0));
        let edit = history::Edit {
            field: "status_id",
            old: Some(&old),
            new: Some(&new),
        };
        history::record(
            &mut tx,
            ChangeSource::System,
            "issue",
            &issue.to_string(),
            &edit,
        )
        .await?;
    }
    sqlx::query!("DELETE FROM statuses WHERE id = ?", id.0)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}
