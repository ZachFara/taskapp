//! Events: dated things (a meeting, a deadline day) that issues can be linked to, many to many.
//! Every write is a row snapshot (rows.rs), so undo covers events and their links without a handler here.

use chrono::NaiveDate;
use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};

use crate::{ChangeSource, Db, Error, Event, EventId, EventInput, IssueId, Result, palette, rows};

const COLS: &str = "id, title, starts_at, ends_at, all_day, place, notes, icon, color, archived";

async fn from_row(tx: &mut SqliteConnection, r: &SqliteRow) -> Result<Event> {
    let id: i64 = r.try_get("id")?;
    let issue_ids = sqlx::query_scalar(
        "SELECT issue_id FROM event_issues WHERE event_id = ? ORDER BY issue_id",
    )
    .bind(id)
    .fetch_all(tx)
    .await?
    .into_iter()
    .map(IssueId)
    .collect();
    Ok(Event {
        id: EventId(id),
        title: r.try_get("title")?,
        starts_at: r.try_get("starts_at")?,
        ends_at: r.try_get("ends_at")?,
        all_day: r.try_get("all_day")?,
        place: r.try_get("place")?,
        notes: r.try_get("notes")?,
        icon: r.try_get("icon")?,
        color: r.try_get("color")?,
        archived: r.try_get("archived")?,
        issue_ids,
    })
}

async fn fetch(tx: &mut SqliteConnection, id: EventId) -> Result<Event> {
    let row = sqlx::query(&format!("SELECT {COLS} FROM events WHERE id = ?"))
        .bind(id.0)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| Error::NotFound(format!("event {}", id.0)))?;
    from_row(tx, &row).await
}

/// Checks the input and returns the color as a palette key.
fn check(input: &EventInput) -> Result<Option<String>> {
    if input.title.trim().is_empty() {
        return Err(Error::Invalid("an event needs a title".into()));
    }
    if input.ends_at.is_some_and(|end| end < input.starts_at) {
        return Err(Error::Invalid(
            "an event cannot end before it starts".into(),
        ));
    }
    if let Some(icon) = &input.icon {
        palette::check_icon(icon)?;
    }
    input
        .color
        .as_deref()
        .map(|c| palette::key_for(c).map(str::to_owned))
        .transpose()
}

/// Events overlapping `from..=to` (either end open), oldest first. Archived events are left out.
///
/// # Errors
///
/// Fails on a database error.
pub async fn list(db: &Db, from: Option<NaiveDate>, to: Option<NaiveDate>) -> Result<Vec<Event>> {
    let mut conn = db.pool.acquire().await?;
    let rows = sqlx::query(&format!(
        "SELECT {COLS} FROM events WHERE archived = 0
           AND (?1 IS NULL OR date(coalesce(ends_at, starts_at)) >= ?1)
           AND (?2 IS NULL OR date(starts_at) <= ?2)
         ORDER BY starts_at, id"
    ))
    .bind(from)
    .bind(to)
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
pub async fn get(db: &Db, id: EventId) -> Result<Event> {
    fetch(&mut *db.pool.acquire().await?, id).await
}

/// # Errors
///
/// `Invalid` for an empty title, an end before the start or a bad icon or color; or a database error.
pub async fn create(db: &Db, input: EventInput, src: ChangeSource) -> Result<Event> {
    let color = check(&input)?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let id = sqlx::query(
        "INSERT INTO events (title, starts_at, ends_at, all_day, place, notes, icon, color, archived)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&input.title)
    .bind(input.starts_at)
    .bind(input.ends_at)
    .bind(input.all_day)
    .bind(&input.place)
    .bind(&input.notes)
    .bind(&input.icon)
    .bind(color)
    .bind(input.archived)
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();
    rows::created(&mut tx, src, "events", id).await?;
    let event = fetch(&mut tx, EventId(id)).await?;
    tx.commit().await?;
    Ok(event)
}

/// Replaces every field; the links are kept.
///
/// # Errors
///
/// As [`create`], and `NotFound` for an unknown id.
pub async fn update(db: &Db, id: EventId, input: EventInput, src: ChangeSource) -> Result<Event> {
    let color = check(&input)?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    fetch(&mut tx, id).await?;
    let before = rows::by_key(&mut tx, "events", id.0).await?;
    sqlx::query(
        "UPDATE events SET title = ?, starts_at = ?, ends_at = ?, all_day = ?, place = ?, notes = ?,
           icon = ?, color = ?, archived = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(&input.title)
    .bind(input.starts_at)
    .bind(input.ends_at)
    .bind(input.all_day)
    .bind(&input.place)
    .bind(&input.notes)
    .bind(&input.icon)
    .bind(color)
    .bind(input.archived)
    .bind(id.0)
    .execute(&mut *tx)
    .await?;
    rows::changed(&mut tx, src, before).await?;
    let event = fetch(&mut tx, id).await?;
    tx.commit().await?;
    Ok(event)
}

/// Deletes the event and its links; the issues stay.
///
/// # Errors
///
/// `NotFound` for an unknown id, or a database error.
pub async fn delete(db: &Db, id: EventId, src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    fetch(&mut tx, id).await?;
    let links = rows::snap(&mut tx, "event_issues", "event_id = ?", id.0).await?;
    let event = rows::by_key(&mut tx, "events", id.0).await?;
    sqlx::query("DELETE FROM events WHERE id = ?")
        .bind(id.0)
        .execute(&mut *tx)
        .await?;
    // Links first, so undo (newest first) restores the event before its links.
    rows::changed(&mut tx, src, links).await?;
    rows::changed(&mut tx, src, event).await?;
    tx.commit().await?;
    Ok(())
}

/// Idempotent: linking an already linked pair succeeds and records nothing.
///
/// # Errors
///
/// `NotFound` for an unknown event or issue, or a database error.
pub async fn link(db: &Db, event: EventId, issue: IssueId, src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    fetch(&mut tx, event).await?;
    let done = sqlx::query("INSERT OR IGNORE INTO event_issues (event_id, issue_id) VALUES (?, ?)")
        .bind(event.0)
        .bind(issue.0)
        .execute(&mut *tx)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(d) if d.is_foreign_key_violation() => {
                Error::NotFound(format!("issue {}", issue.0))
            }
            _ => Error::Db(e),
        })?;
    if done.rows_affected() == 1 {
        rows::created(&mut tx, src, "event_issues", done.last_insert_rowid()).await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Removing a link that does not exist is not an error.
///
/// # Errors
///
/// Fails on a database error.
pub async fn unlink(db: &Db, event: EventId, issue: IssueId, src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let id: Option<i64> =
        sqlx::query_scalar("SELECT id FROM event_issues WHERE event_id = ? AND issue_id = ?")
            .bind(event.0)
            .bind(issue.0)
            .fetch_optional(&mut *tx)
            .await?;
    let before = rows::by_key(&mut tx, "event_issues", id.unwrap_or(0)).await?;
    sqlx::query("DELETE FROM event_issues WHERE event_id = ? AND issue_id = ?")
        .bind(event.0)
        .bind(issue.0)
        .execute(&mut *tx)
        .await?;
    rows::changed(&mut tx, src, before).await?;
    tx.commit().await?;
    Ok(())
}
