use taskapp_core::{Db, Result};

/// `(id, name)` of each calendar the account can write to.
///
/// # Errors
///
/// Fails when not connected or the request fails.
pub async fn list_calendars(_db: &Db) -> Result<Vec<(String, String)>> {
    todo!("T24")
}
