use crate::{Db, Pin, PinId, PinKind, ResolvedPin, Result};

/// Idempotent: pinning the same `(kind, target)` twice returns the existing pin.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn pin(_db: &Db, _kind: PinKind, _target: &str, _title: Option<String>) -> Result<Pin> {
    todo!("T11")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn unpin(_db: &Db, _id: PinId) -> Result<()> {
    todo!("T11")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn reorder(_db: &Db, _ids_in_order: &[PinId]) -> Result<()> {
    todo!("T11")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn list_resolved(_db: &Db) -> Result<Vec<ResolvedPin>> {
    todo!("T11")
}
