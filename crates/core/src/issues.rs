use crate::{ChangeSource, Db, Issue, IssueId, IssuePatch, NewIssue, Result};

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn create(_db: &Db, _new: NewIssue, _src: ChangeSource) -> Result<Issue> {
    todo!("T2")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn update(
    _db: &Db,
    _id: IssueId,
    _patch: IssuePatch,
    _src: ChangeSource,
) -> Result<Issue> {
    todo!("T2")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn get(_db: &Db, _id: IssueId) -> Result<Issue> {
    todo!("T2")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn get_by_key(_db: &Db, _key: &str) -> Result<Issue> {
    todo!("T2")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn archive(_db: &Db, _id: IssueId, _src: ChangeSource) -> Result<()> {
    todo!("T2")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn restore(_db: &Db, _id: IssueId, _src: ChangeSource) -> Result<()> {
    todo!("T2")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn children(_db: &Db, _id: IssueId) -> Result<Vec<Issue>> {
    todo!("T2")
}
