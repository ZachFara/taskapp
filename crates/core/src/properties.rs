use serde_json::Value;

use crate::{ChangeSource, Db, Issue, IssueId, PropertyDef, PropertyKind, Result, SelectOption};

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn list_defs(_db: &Db) -> Result<Vec<PropertyDef>> {
    todo!("T3")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn create_def(_db: &Db, _def: PropertyDef, _src: ChangeSource) -> Result<PropertyDef> {
    todo!("T3")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn rename_def(
    _db: &Db,
    _key: &str,
    _new_name: &str,
    _src: ChangeSource,
) -> Result<PropertyDef> {
    todo!("T3")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn change_kind(
    _db: &Db,
    _key: &str,
    _kind: PropertyKind,
    _options: Vec<SelectOption>,
    _src: ChangeSource,
) -> Result<PropertyDef> {
    todo!("T3")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn delete_def(_db: &Db, _key: &str, _src: ChangeSource) -> Result<()> {
    todo!("T3")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn set_value(
    _db: &Db,
    _issue: IssueId,
    _key: &str,
    _value: Option<Value>,
    _src: ChangeSource,
) -> Result<Issue> {
    todo!("T3")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub fn validate(_def: &PropertyDef, _value: &Value) -> Result<()> {
    todo!("T3")
}
