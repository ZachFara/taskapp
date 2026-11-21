use crate::{Db, Result, Status, StatusCategory, StatusId};

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn list(_db: &Db) -> Result<Vec<Status>> {
    todo!("T2")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn create(
    _db: &Db,
    _name: &str,
    _category: StatusCategory,
    _color: &str,
) -> Result<Status> {
    todo!("T2")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn update(
    _db: &Db,
    _id: StatusId,
    _name: Option<String>,
    _color: Option<String>,
    _position: Option<i32>,
) -> Result<Status> {
    todo!("T2")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn delete(_db: &Db, _id: StatusId, _move_issues_to: StatusId) -> Result<()> {
    todo!("T2")
}
