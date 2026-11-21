use std::path::Path;

use taskapp_core::Result;

/// Returns whether a commit was made.
///
/// # Errors
///
/// Fails when git fails.
pub async fn commit_and_push(_dir: &Path, _message: &str, _remote: Option<&str>) -> Result<bool> {
    todo!("T25")
}
