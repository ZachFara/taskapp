use serde::{Serialize, de::DeserializeOwned};

use crate::{ChangeSource, Db, Error, Result, rows};

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
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let before = rows::by_key(&mut tx, "settings", key).await?;
    sqlx::query!(
        "INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        key,
        json
    )
    .execute(&mut *tx)
    .await?;
    // sync_status is rewritten by every sync, not chosen by the user: nothing to undo, and it would flood history.
    if key == "sync_status" {
    } else if before.is_empty() {
        rows::created(&mut tx, ChangeSource::System, "settings", key).await?;
    } else {
        rows::changed(&mut tx, ChangeSource::System, before).await?;
    }
    tx.commit().await?;
    Ok(())
}
