use chrono::{DateTime, Utc};

use crate::{ChangeSource, Comment, CommentId, Db, IssueId, Result};

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn list(_db: &Db, _issue: IssueId) -> Result<Vec<Comment>> {
    todo!("T4")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn add_comment(
    _db: &Db,
    _issue: IssueId,
    _body: &str,
    _created_at: Option<DateTime<Utc>>,
    _src: ChangeSource,
) -> Result<Comment> {
    todo!("T4")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn update(_db: &Db, _id: CommentId, _body: &str, _src: ChangeSource) -> Result<Comment> {
    todo!("T4")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn delete(_db: &Db, _id: CommentId, _src: ChangeSource) -> Result<()> {
    todo!("T4")
}
