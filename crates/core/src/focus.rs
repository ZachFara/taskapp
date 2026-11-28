//! Focus sessions: a timer on one issue, written as a row when it ends. Only one session runs at a time, and a
//! running one survives a reload because it is the row with no end.

use std::collections::HashSet;

use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;
use sqlx::SqliteConnection;

use crate::{
    Bucket, ChangeSource, Db, DeepBucket, DeepHours, Error, FocusSession, IssueId, Result,
    insights::{local, zone},
    metrics::{bucket_start, next_start},
    rows,
};

/// Ends the open session, if there is one, and returns its id.
async fn close_open(tx: &mut SqliteConnection, src: ChangeSource) -> Result<Option<i64>> {
    let Some(id) = open_id(tx).await? else {
        return Ok(None);
    };
    let before = rows::snap(
        tx,
        "focus_sessions",
        "ended_at IS NULL AND ? IS NOT NULL",
        1,
    )
    .await?;
    let now = Utc::now();
    sqlx::query!(
        "UPDATE focus_sessions SET ended_at = ? WHERE ended_at IS NULL",
        now
    )
    .execute(&mut *tx)
    .await?;
    rows::changed(tx, src, before).await?;
    Ok(Some(id))
}

async fn open_id(conn: &mut SqliteConnection) -> Result<Option<i64>> {
    Ok(
        sqlx::query_scalar!(
            r#"SELECT id AS "id!: i64" FROM focus_sessions WHERE ended_at IS NULL"#
        )
        .fetch_optional(conn)
        .await?,
    )
}

async fn session(conn: &mut SqliteConnection, id: i64) -> Result<FocusSession> {
    let r = sqlx::query!(
        r#"SELECT id AS "id!: i64", issue_id, started_at AS "started_at: DateTime<Utc>",
                  ended_at AS "ended_at: DateTime<Utc>", deep
           FROM focus_sessions WHERE id = ?"#,
        id
    )
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| Error::NotFound(format!("focus session {id}")))?;
    Ok(FocusSession {
        id: r.id,
        issue_id: IssueId(r.issue_id),
        started_at: r.started_at,
        ended_at: r.ended_at,
        deep: r.deep != 0,
    })
}

/// Starts a session on `issue`. A session already running is ended first. `deep` defaults to the issue's depth.
///
/// # Errors
///
/// `NotFound` for an unknown issue; fails on a database error.
pub async fn start(
    db: &Db,
    issue: IssueId,
    deep: Option<bool>,
    src: ChangeSource,
) -> Result<FocusSession> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let depth = sqlx::query_scalar!("SELECT depth FROM issues WHERE id = ?", issue.0)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| Error::NotFound(format!("issue {}", issue.0)))?;
    let deep = deep.unwrap_or(depth.as_deref() == Some("deep"));
    close_open(&mut tx, src).await?;
    let now = Utc::now();
    let id = sqlx::query!(
        "INSERT INTO focus_sessions (issue_id, started_at, deep) VALUES (?, ?, ?)",
        issue.0,
        now,
        deep
    )
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();
    rows::created(&mut tx, src, "focus_sessions", id).await?;
    let s = session(&mut tx, id).await?;
    tx.commit().await?;
    Ok(s)
}

/// Ends the running session. `None` when nothing runs.
///
/// # Errors
///
/// Fails on a database error.
pub async fn stop(db: &Db, src: ChangeSource) -> Result<Option<FocusSession>> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let Some(id) = close_open(&mut tx, src).await? else {
        return Ok(None);
    };
    let s = session(&mut tx, id).await?;
    tx.commit().await?;
    Ok(Some(s))
}

/// The running session, if one runs.
///
/// # Errors
///
/// Fails on a database error.
pub async fn running(db: &Db) -> Result<Option<FocusSession>> {
    let mut conn = db.pool.acquire().await?;
    match open_id(&mut conn).await? {
        Some(id) => Ok(Some(session(&mut conn, id).await?)),
        None => Ok(None),
    }
}

/// Sessions, newest first, for one issue or all; `from` and `to` are local days, both ends included.
///
/// # Errors
///
/// Fails on a database error.
pub async fn log(
    db: &Db,
    issue: Option<IssueId>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
) -> Result<Vec<FocusSession>> {
    let tz = zone(db).await?;
    let issue = issue.map(|i| i.0);
    let rows = sqlx::query!(
        r#"SELECT id AS "id!: i64", issue_id, started_at AS "started_at: DateTime<Utc>",
                  ended_at AS "ended_at: DateTime<Utc>", deep
           FROM focus_sessions WHERE ? IS NULL OR issue_id = ?
           ORDER BY started_at DESC, id DESC"#,
        issue,
        issue
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| FocusSession {
            id: r.id,
            issue_id: IssueId(r.issue_id),
            started_at: r.started_at,
            ended_at: r.ended_at,
            deep: r.deep != 0,
        })
        .filter(|s| {
            let day = local(s.started_at, tz);
            from.is_none_or(|f| day >= f) && to.is_none_or(|t| day <= t)
        })
        .collect())
}

/// Hours of deep sessions per bucket from `from` to `to` (local days), their total, and the run of days ending
/// today with some deep time. A day with none does not break the run until today is over.
pub(crate) async fn deep_hours(
    db: &Db,
    from: NaiveDate,
    to: NaiveDate,
    bucket: Bucket,
    now: DateTime<Utc>,
    tz: Tz,
) -> Result<DeepHours> {
    let rows = sqlx::query!(
        r#"SELECT started_at AS "started_at: DateTime<Utc>", ended_at AS "ended_at: DateTime<Utc>"
           FROM focus_sessions WHERE deep = 1"#
    )
    .fetch_all(&db.pool)
    .await?;
    let mut days: Vec<(NaiveDate, f64)> = Vec::new();
    for r in rows {
        let secs = (r.ended_at.unwrap_or(now) - r.started_at)
            .num_seconds()
            .max(0);
        let hours = f64::from(i32::try_from(secs).unwrap_or(i32::MAX)) / 3600.0;
        days.push((local(r.started_at, tz), hours));
    }
    let mut buckets = Vec::new();
    let mut start = bucket_start(from, bucket);
    while start <= to {
        buckets.push(DeepBucket { start, hours: 0.0 });
        start = next_start(start, bucket);
    }
    for (day, hours) in days.iter().filter(|(d, _)| *d >= from && *d <= to) {
        let at = bucket_start(*day, bucket);
        if let Some(b) = buckets.iter_mut().find(|b| b.start == at) {
            b.hours += *hours;
        }
    }
    let worked: HashSet<NaiveDate> = days
        .iter()
        .filter(|(_, h)| *h > 0.0)
        .map(|(d, _)| *d)
        .collect();
    let mut day = local(now, tz);
    if !worked.contains(&day) {
        day = day.pred_opt().unwrap_or(day);
    }
    let mut streak = 0;
    while worked.contains(&day) {
        streak += 1;
        day = day.pred_opt().unwrap_or(day);
    }
    Ok(DeepHours {
        total_hours: buckets.iter().map(|b| b.hours).sum(),
        buckets,
        streak_days: streak,
    })
}
