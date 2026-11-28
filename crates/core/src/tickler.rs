//! The tickler: issues deferred to a later day (`hidden_until`). They leave the Inbox and come back on that day.
//! Deferring is an ordinary issue write, so undo and export need nothing extra.

use chrono::NaiveDate;

use crate::{ChangeSource, Db, Error, Issue, IssueId, IssuePatch, Result, issues};

/// Not archived issues that stay hidden after `today`, earliest return day first, then by number.
///
/// # Errors
///
/// Fails on a database error.
pub async fn list(db: &Db, today: NaiveDate) -> Result<Vec<Issue>> {
    let ids: Vec<i64> = sqlx::query_scalar(
        "SELECT id FROM issues WHERE archived_at IS NULL AND hidden_until > ?1
         ORDER BY hidden_until, number",
    )
    .bind(today)
    .fetch_all(&db.pool)
    .await?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        out.push(issues::get(db, IssueId(id)).await?);
    }
    Ok(out)
}

/// Hides the issue until `until` (or brings it back now when `None`). Status, due day and calendar are untouched.
///
/// # Errors
///
/// `Invalid` when `until` is not after `today`; `NotFound` for an unknown issue.
pub async fn defer(
    db: &Db,
    id: IssueId,
    until: Option<NaiveDate>,
    today: NaiveDate,
    src: ChangeSource,
) -> Result<Issue> {
    if until.is_some_and(|d| d <= today) {
        return Err(Error::Invalid(
            "the hide-until date must be after today".into(),
        ));
    }
    let patch = IssuePatch {
        hidden_until: Some(until),
        ..IssuePatch::default()
    };
    issues::update(db, id, patch, src).await
}
