use chrono::NaiveDate;

use crate::{ChangeSource, Cycle, CycleId, Db, Result};

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn list(_db: &Db) -> Result<Vec<Cycle>> {
    todo!("T4")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn create(
    _db: &Db,
    _name: Option<String>,
    _start_date: NaiveDate,
    _end_date: NaiveDate,
    _src: ChangeSource,
) -> Result<Cycle> {
    todo!("T4")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn update(_db: &Db, _cycle: Cycle, _src: ChangeSource) -> Result<Cycle> {
    todo!("T4")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn delete(_db: &Db, _id: CycleId, _src: ChangeSource) -> Result<()> {
    todo!("T4")
}
