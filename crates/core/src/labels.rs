use crate::{ChangeSource, Db, IssueId, Label, LabelId, Result};

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn list(_db: &Db) -> Result<Vec<Label>> {
    todo!("T4")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn create(_db: &Db, _name: &str, _color: &str, _src: ChangeSource) -> Result<Label> {
    todo!("T4")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn update(_db: &Db, _label: Label, _src: ChangeSource) -> Result<Label> {
    todo!("T4")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn delete(_db: &Db, _id: LabelId, _src: ChangeSource) -> Result<()> {
    todo!("T4")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn add_label(
    _db: &Db,
    _issue: IssueId,
    _label: LabelId,
    _src: ChangeSource,
) -> Result<()> {
    todo!("T4")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn remove_label(
    _db: &Db,
    _issue: IssueId,
    _label: LabelId,
    _src: ChangeSource,
) -> Result<()> {
    todo!("T4")
}
