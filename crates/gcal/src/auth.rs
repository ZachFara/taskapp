use taskapp_core::{Db, Result};

/// Browser OAuth with PKCE on a loopback redirect.
///
/// # Errors
///
/// Fails when the user cancels, the exchange fails, or the token cannot be stored.
pub async fn connect(_db: &Db) -> Result<()> {
    todo!("T24")
}
