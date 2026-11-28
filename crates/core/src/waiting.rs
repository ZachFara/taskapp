//! Waiting for: the open issues that carry a `waiting_on` note or a `follow_up` day. The fields are plain issue
//! fields written through `issues::update`, so undo and export need nothing extra.

use crate::{Db, Issue, IssueId, Result, issues};

/// Open (not archived, not Done or Canceled) issues with a waiting note or follow-up day. Issues with a follow-up
/// day come first, earliest day first; then the rest by number.
///
/// # Errors
///
/// Fails on a database error.
pub async fn list(db: &Db) -> Result<Vec<Issue>> {
    let ids: Vec<i64> = sqlx::query_scalar(
        "SELECT i.id FROM issues i JOIN statuses s ON s.id = i.status_id
         WHERE i.archived_at IS NULL AND (i.waiting_on IS NOT NULL OR i.follow_up IS NOT NULL)
           AND s.category NOT IN ('completed', 'canceled')
         ORDER BY i.follow_up IS NULL, i.follow_up, i.number",
    )
    .fetch_all(&db.pool)
    .await?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        out.push(issues::get(db, IssueId(id)).await?);
    }
    Ok(out)
}
