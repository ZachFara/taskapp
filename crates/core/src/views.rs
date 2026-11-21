use serde_json::Value;

use crate::{Db, Issue, PropertyDef, Result, SavedView, ViewDef, ViewId};

/// Compiles a view definition to SQL plus bind values.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub fn compile(_def: &ViewDef, _defs: &[PropertyDef]) -> Result<(String, Vec<Value>)> {
    todo!("T6")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn query(_db: &Db, _def: &ViewDef) -> Result<Vec<Issue>> {
    todo!("T6")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn list(_db: &Db) -> Result<Vec<SavedView>> {
    todo!("T6")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn save(_db: &Db, _id: Option<ViewId>, _name: &str, _def: ViewDef) -> Result<SavedView> {
    todo!("T6")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn delete(_db: &Db, _id: ViewId) -> Result<()> {
    todo!("T6")
}
