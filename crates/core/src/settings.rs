use serde::{Serialize, de::DeserializeOwned};

use crate::{Db, Error, Result};

/// Setting keys: `issue_prefix`, `reminders_list_id`, `gcal_enabled`, `gcal_calendar_id`, `export_enabled`,
/// `export_dir`, `export_remote`, `backup_dir`, `sync_status`. Values are JSON.
///
/// # Errors
/// Fails when the query fails or the stored value is not a `T`.
pub async fn get<T: DeserializeOwned>(db: &Db, key: &str) -> Result<Option<T>> {
    let row = sqlx::query_scalar!("SELECT value FROM settings WHERE key = ?", key)
        .fetch_optional(&db.pool)
        .await?;
    row.map(|v| serde_json::from_str(&v).map_err(|e| Error::Invalid(format!("setting {key}: {e}"))))
        .transpose()
}

/// # Errors
/// Fails when the value cannot be encoded or the write fails.
pub async fn set<T: Serialize>(db: &Db, key: &str, value: &T) -> Result<()> {
    let json = serde_json::to_string(value).map_err(|e| Error::Invalid(e.to_string()))?;
    sqlx::query!(
        "INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        key,
        json
    )
    .execute(&db.pool)
    .await?;
    Ok(())
}
