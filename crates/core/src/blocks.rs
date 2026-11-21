use chrono::{DateTime, Utc};

use crate::{Db, IssueId, Result, WorkBlock, WorkBlockId};

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn create_block(
    _db: &Db,
    _issue: IssueId,
    _start: DateTime<Utc>,
    _end: DateTime<Utc>,
) -> Result<WorkBlock> {
    todo!("T4")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn move_block(
    _db: &Db,
    _id: WorkBlockId,
    _start: DateTime<Utc>,
    _end: DateTime<Utc>,
) -> Result<WorkBlock> {
    todo!("T4")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn delete_block(_db: &Db, _id: WorkBlockId) -> Result<()> {
    todo!("T4")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn blocks_between(
    _db: &Db,
    _from: DateTime<Utc>,
    _to: DateTime<Utc>,
) -> Result<Vec<WorkBlock>> {
    todo!("T4")
}
