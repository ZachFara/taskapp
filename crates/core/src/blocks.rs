use chrono::{DateTime, Utc};

use crate::{ChangeSource, Db, Error, IssueId, Result, WorkBlock, WorkBlockId, rows};

/// A block ends strictly after it starts.
fn check_span(start: DateTime<Utc>, end: DateTime<Utc>) -> Result<()> {
    if end <= start {
        return Err(Error::Invalid(format!(
            "work block ends at {end}, not after {start}"
        )));
    }
    Ok(())
}

/// Work blocks take no `ChangeSource`, so they write no change history.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn create_block(
    db: &Db,
    issue: IssueId,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<WorkBlock> {
    check_span(start, end)?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let id = sqlx::query!(
        "INSERT INTO work_blocks (issue_id, start_utc, end_utc) VALUES (?, ?, ?)",
        issue.0,
        start,
        end
    )
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();
    rows::created(&mut tx, ChangeSource::Ui, "work_blocks", id).await?;
    tx.commit().await?;
    Ok(WorkBlock {
        id: WorkBlockId(id),
        issue_id: issue,
        start_utc: start,
        end_utc: end,
    })
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn move_block(
    db: &Db,
    id: WorkBlockId,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<WorkBlock> {
    check_span(start, end)?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let before = rows::by_key(&mut tx, "work_blocks", id.0).await?;
    let done = sqlx::query!(
        "UPDATE work_blocks SET start_utc = ?, end_utc = ? WHERE id = ?",
        start,
        end,
        id.0
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if done == 0 {
        return Err(Error::NotFound(format!("work block {}", id.0)));
    }
    let issue = sqlx::query_scalar!(
        r#"SELECT issue_id AS "issue_id!: i64" FROM work_blocks WHERE id = ?"#,
        id.0
    )
    .fetch_one(&mut *tx)
    .await?;
    rows::changed(&mut tx, ChangeSource::Ui, before).await?;
    tx.commit().await?;
    Ok(WorkBlock {
        id,
        issue_id: IssueId(issue),
        start_utc: start,
        end_utc: end,
    })
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn delete_block(db: &Db, id: WorkBlockId) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let before = rows::by_key(&mut tx, "work_blocks", id.0).await?;
    let done = sqlx::query!("DELETE FROM work_blocks WHERE id = ?", id.0)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if done == 0 {
        return Err(Error::NotFound(format!("work block {}", id.0)));
    }
    rows::changed(&mut tx, ChangeSource::Ui, before).await?;
    tx.commit().await?;
    Ok(())
}

/// Blocks that overlap `[from, to)`. Compared as instants (julianday), because the stored text
/// can differ in fractional digits, which breaks text order.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn blocks_between(
    db: &Db,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<WorkBlock>> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!: i64", issue_id, start_utc AS "start_utc: DateTime<Utc>",
                  end_utc AS "end_utc: DateTime<Utc>"
           FROM work_blocks
           WHERE julianday(start_utc) < julianday(?) AND julianday(end_utc) > julianday(?)
           ORDER BY julianday(start_utc), id"#,
        to,
        from
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| WorkBlock {
            id: WorkBlockId(r.id),
            issue_id: IssueId(r.issue_id),
            start_utc: r.start_utc,
            end_utc: r.end_utc,
        })
        .collect())
}
