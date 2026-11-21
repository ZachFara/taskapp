use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use crate::{Db, Result};

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn snapshot(_db: &Db, _dir: &Path, _reason: &str) -> Result<PathBuf> {
    todo!("T9")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn nightly(_db: &Db, _dir: &Path, _now: DateTime<Utc>) -> Result<Option<PathBuf>> {
    todo!("T9")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub fn prune(_dir: &Path) -> Result<Vec<PathBuf>> {
    todo!("T9")
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn restore(_from: &Path, _to: &Path) -> Result<()> {
    todo!("T9")
}
