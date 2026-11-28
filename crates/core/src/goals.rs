//! Goals: a few per quarter that projects link to. Every write is a row snapshot (rows.rs), so undo covers
//! goals and their links.

use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};

use crate::{ChangeSource, Db, Error, Goal, GoalId, GoalStatus, ProjectId, Result, projects, rows};

const COLS: &str = "id, title, quarter, description, status, created_at";

fn bad(e: &str) -> Error {
    Error::Invalid(e.to_owned())
}

fn status_word(s: GoalStatus) -> &'static str {
    match s {
        GoalStatus::Active => "active",
        GoalStatus::Achieved => "achieved",
        GoalStatus::Dropped => "dropped",
    }
}

fn check(title: &str, quarter: &str) -> Result<()> {
    if title.trim().is_empty() {
        return Err(bad("a goal needs a title"));
    }
    let b = quarter.as_bytes();
    let ok = b.len() == 7
        && b[..4].iter().all(u8::is_ascii_digit)
        && &b[4..6] == b"-Q"
        && matches!(b[6], b'1'..=b'4');
    if !ok {
        return Err(bad("quarter looks like 2027-Q1"));
    }
    Ok(())
}

async fn from_row(tx: &mut SqliteConnection, r: &SqliteRow) -> Result<Goal> {
    let id: i64 = r.try_get("id")?;
    let project_ids = sqlx::query_scalar(
        "SELECT project_id FROM goal_projects WHERE goal_id = ? ORDER BY project_id",
    )
    .bind(id)
    .fetch_all(tx)
    .await?
    .into_iter()
    .map(ProjectId)
    .collect();
    let status: String = r.try_get("status")?;
    Ok(Goal {
        id: GoalId(id),
        title: r.try_get("title")?,
        quarter: r.try_get("quarter")?,
        description: r.try_get("description")?,
        status: serde_json::from_value(serde_json::Value::String(status))
            .map_err(|e| Error::Invalid(e.to_string()))?,
        created_at: r.try_get("created_at")?,
        project_ids,
    })
}

async fn fetch(tx: &mut SqliteConnection, id: GoalId) -> Result<Goal> {
    let row = sqlx::query(&format!("SELECT {COLS} FROM goals WHERE id = ?"))
        .bind(id.0)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| Error::NotFound(format!("goal {}", id.0)))?;
    from_row(tx, &row).await
}

/// Every goal, by quarter then id.
///
/// # Errors
///
/// Fails on a database error.
pub async fn list(db: &Db) -> Result<Vec<Goal>> {
    let mut conn = db.pool.acquire().await?;
    let rows = sqlx::query(&format!("SELECT {COLS} FROM goals ORDER BY quarter, id"))
        .fetch_all(&mut *conn)
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in &rows {
        out.push(from_row(&mut conn, r).await?);
    }
    Ok(out)
}

/// # Errors
///
/// `NotFound` for an unknown id, or a database error.
pub async fn get(db: &Db, id: GoalId) -> Result<Goal> {
    fetch(&mut *db.pool.acquire().await?, id).await
}

/// # Errors
///
/// `Invalid` for an empty title or a quarter that is not like `2027-Q1`.
pub async fn create(
    db: &Db,
    title: &str,
    quarter: &str,
    description: Option<String>,
    src: ChangeSource,
) -> Result<Goal> {
    check(title, quarter)?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let id = sqlx::query("INSERT INTO goals (title, quarter, description) VALUES (?, ?, ?)")
        .bind(title.trim())
        .bind(quarter)
        .bind(description.unwrap_or_default())
        .execute(&mut *tx)
        .await?
        .last_insert_rowid();
    rows::created(&mut tx, src, "goals", id).await?;
    let goal = fetch(&mut tx, GoalId(id)).await?;
    tx.commit().await?;
    Ok(goal)
}

/// Saves title, quarter, description and status; the links change only through `link` and `unlink`.
///
/// # Errors
///
/// `Invalid` as `create`; `NotFound` for an unknown goal.
pub async fn update(db: &Db, goal: Goal, src: ChangeSource) -> Result<Goal> {
    check(&goal.title, &goal.quarter)?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    fetch(&mut tx, goal.id).await?;
    let before = rows::by_key(&mut tx, "goals", goal.id.0).await?;
    sqlx::query(
        "UPDATE goals SET title = ?, quarter = ?, description = ?, status = ? WHERE id = ?",
    )
    .bind(goal.title.trim())
    .bind(&goal.quarter)
    .bind(&goal.description)
    .bind(status_word(goal.status))
    .bind(goal.id.0)
    .execute(&mut *tx)
    .await?;
    rows::changed(&mut tx, src, before).await?;
    let out = fetch(&mut tx, goal.id).await?;
    tx.commit().await?;
    Ok(out)
}

/// # Errors
///
/// `NotFound` for an unknown goal.
pub async fn delete(db: &Db, id: GoalId, src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    fetch(&mut tx, id).await?;
    let links = rows::snap(&mut tx, "goal_projects", "goal_id = ?", id.0).await?;
    let before = rows::by_key(&mut tx, "goals", id.0).await?;
    sqlx::query("DELETE FROM goals WHERE id = ?")
        .bind(id.0)
        .execute(&mut *tx)
        .await?;
    rows::changed(&mut tx, src, links).await?;
    rows::changed(&mut tx, src, before).await?;
    tx.commit().await?;
    Ok(())
}

/// Idempotent: linking an already linked pair succeeds and records nothing.
///
/// # Errors
///
/// `NotFound` for an unknown goal or project, or a database error.
pub async fn link(db: &Db, goal: GoalId, project: ProjectId, src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    fetch(&mut tx, goal).await?;
    let done =
        sqlx::query("INSERT OR IGNORE INTO goal_projects (goal_id, project_id) VALUES (?, ?)")
            .bind(goal.0)
            .bind(project.0)
            .execute(&mut *tx)
            .await
            .map_err(|e| match &e {
                sqlx::Error::Database(d) if d.is_foreign_key_violation() => {
                    Error::NotFound(format!("project {}", project.0))
                }
                _ => Error::Db(e),
            })?;
    if done.rows_affected() == 1 {
        rows::created(&mut tx, src, "goal_projects", done.last_insert_rowid()).await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Removing a link that does not exist is not an error.
///
/// # Errors
///
/// Fails on a database error.
pub async fn unlink(db: &Db, goal: GoalId, project: ProjectId, src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let id: Option<i64> =
        sqlx::query_scalar("SELECT id FROM goal_projects WHERE goal_id = ? AND project_id = ?")
            .bind(goal.0)
            .bind(project.0)
            .fetch_optional(&mut *tx)
            .await?;
    let before = rows::by_key(&mut tx, "goal_projects", id.unwrap_or(0)).await?;
    sqlx::query("DELETE FROM goal_projects WHERE goal_id = ? AND project_id = ?")
        .bind(goal.0)
        .bind(project.0)
        .execute(&mut *tx)
        .await?;
    rows::changed(&mut tx, src, before).await?;
    tx.commit().await?;
    Ok(())
}

/// `(completed, total)` issues over the linked projects, from the project progress.
///
/// # Errors
///
/// `NotFound` for an unknown goal.
pub async fn progress(db: &Db, id: GoalId) -> Result<(u32, u32)> {
    let mut sum = (0, 0);
    for p in get(db, id).await?.project_ids {
        let (done, total) = projects::progress(db, p).await?;
        sum = (sum.0 + done, sum.1 + total);
    }
    Ok(sum)
}
