use crate::{Db, Issue, IssueId, Result};

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn search(_db: &Db, _text: &str, _limit: u32) -> Result<Vec<Issue>> {
    todo!("T6")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn reindex(_db: &Db, _id: IssueId) -> Result<()> {
    todo!("T6")
}
