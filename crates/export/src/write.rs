use std::path::Path;

use taskapp_core::{Db, Result};

pub struct ExportStats {
    pub issues: u32,
    pub files_written: u32,
    pub files_removed: u32,
}

/// # Errors
///
/// Fails when the database cannot be read or a file cannot be written.
pub async fn write_snapshot(_db: &Db, _dir: &Path) -> Result<ExportStats> {
    todo!("T25")
}
