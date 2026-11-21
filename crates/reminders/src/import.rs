use std::path::Path;

use chrono::NaiveDate;
use taskapp_core::{Db, Result};

pub struct ImportReport {
    pub imported: u32,
    pub skipped_duplicates: u32,
    pub failed: Vec<String>,
}

/// # Errors
///
/// Fails when the helper cannot run or the database cannot be written.
pub async fn import(
    _db: &Db,
    _helper: &Path,
    _today: NaiveDate,
    _tz: &str,
) -> Result<ImportReport> {
    todo!("T23")
}
