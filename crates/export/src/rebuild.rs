use std::path::Path;

use taskapp_core::{Db, Result};

/// Rebuilds a database at `db_path` from an export directory.
///
/// # Errors
///
/// Fails when a file cannot be parsed or the database cannot be written.
pub async fn rebuild(_dir: &Path, _db_path: &Path) -> Result<Db> {
    todo!("T25")
}
