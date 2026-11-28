//! Issues that have sat in their status longer than its nudge age (`nudge_days`; none means never). The clock is
//! the issue's last status change, or its last update when it has no status history (imports have none).

use chrono::{DateTime, Duration, Utc};
use sqlx::Row;

use crate::{Db, Result, StaleIssue, issues, issues::ISSUE_COLS};

/// Stale issues, longest-waiting first.
///
/// # Errors
///
/// Fails on a database error.
pub async fn list(db: &Db) -> Result<Vec<StaleIssue>> {
    list_at(db, Utc::now()).await
}

async fn list_at(db: &Db, now: DateTime<Utc>) -> Result<Vec<StaleIssue>> {
    let rows = sqlx::query(&format!(
        "SELECT {ISSUE_COLS}, statuses.name AS status_name, statuses.nudge_days AS nudge_days,
            COALESCE((SELECT MAX(at) FROM changes WHERE entity = 'issue' AND field = 'status_id'
                AND entity_id = CAST(issues.id AS TEXT)), issues.updated_at) AS since
         FROM issues JOIN statuses ON statuses.id = issues.status_id
         WHERE statuses.nudge_days IS NOT NULL AND issues.archived_at IS NULL"
    ))
    .fetch_all(&db.pool)
    .await?;
    let mut conn = db.pool.acquire().await?;
    let found = issues::issues_from_rows(&mut conn, &rows).await?;
    let mut out = Vec::new();
    for (issue, r) in found.into_iter().zip(&rows) {
        let nudge: i32 = r.try_get("nudge_days")?;
        let since: DateTime<Utc> = r.try_get("since")?;
        let waited = now - since;
        if waited > Duration::days(i64::from(nudge)) {
            out.push(StaleIssue {
                issue,
                status_name: r.try_get("status_name")?,
                nudge_days: nudge,
                since,
                days: i32::try_from(waited.num_days()).unwrap_or(i32::MAX),
            });
        }
    }
    out.sort_by_key(|s| s.since);
    Ok(out)
}
