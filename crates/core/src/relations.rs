use crate::{ChangeSource, Db, IssueId, Relation, Result};

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn add(_db: &Db, _rel: Relation, _src: ChangeSource) -> Result<()> {
    todo!("T5")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn remove(_db: &Db, _rel: Relation, _src: ChangeSource) -> Result<()> {
    todo!("T5")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn for_issue(_db: &Db, _id: IssueId) -> Result<Vec<Relation>> {
    todo!("T5")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn all_blocking(_db: &Db) -> Result<Vec<Relation>> {
    todo!("T5")
}
