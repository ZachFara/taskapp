use std::path::Path;

use taskapp_core::Result;

/// `(id, name)` of each Reminders list.
///
/// # Errors
///
/// Fails when the helper cannot run or reports an error.
pub async fn list_reminder_lists(_helper: &Path) -> Result<Vec<(String, String)>> {
    todo!("T23")
}
