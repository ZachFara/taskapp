use taskapp_core::{Db, Result};

pub struct ReconcileReport {
    pub created: u32,
    pub updated: u32,
    pub deleted: u32,
}

/// # Errors
///
/// Fails when the calendar request fails; an expired or revoked token sets the visible error state first.
pub async fn reconcile(_db: &Db) -> Result<ReconcileReport> {
    todo!("T24")
}
