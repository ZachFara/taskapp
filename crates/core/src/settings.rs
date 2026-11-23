use serde::{Serialize, de::DeserializeOwned};

use crate::{ChangeSource, Db, Error, Result, rows, workspace};

/// Setting keys: `issue_prefix`, `reminders_list_id`, `gcal_enabled`, `gcal_calendar_id`, `export_enabled`,
/// `export_dir`, `export_remote`, `backup_dir`, `sync_status`, `estimation` (none, points, time, both), `point_scale`
/// (fibonacci, linear, powers, tshirt). Values are JSON.
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
    let allowed: Option<&[&str]> = match key {
        "estimation" => Some(&["none", "points", "time", "both"]),
        "point_scale" => Some(&["fibonacci", "linear", "powers", "tshirt"]),
        _ => None,
    };
    if let Some(allowed) = allowed {
        let ok = serde_json::from_str::<String>(&json).is_ok_and(|v| allowed.contains(&v.as_str()));
        if !ok {
            return Err(Error::Invalid(format!(
                "setting {key} must be one of {}",
                allowed.join(", ")
            )));
        }
    }
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

/// Keys the UI may write through `settings_set`. Other keys are written by the core and sync crates directly.
const USER_KEYS: &[&str] = &[
    "workspace_name",
    "issue_prefix",
    "timezone",
    "setup_done",
    "week_start",
    "theme",
    "estimation",
    "point_scale",
    "insights_charts",
    "sidebar_sparklines",
    "reminders_list_id",
    "gcal_enabled",
    "gcal_calendar_id",
    "export_enabled",
    "export_dir",
    "export_remote",
    "backup_dir",
    "default_view_template",
];

/// Prefixes of keys the UI writes per item: `insights.<kind>.<id>` and `sidebar_*`.
const USER_PREFIXES: &[&str] = &["insights.", "sidebar_"];

/// Writes a value the UI chose, checked the way its dedicated command checks it.
///
/// # Errors
/// `Invalid` for an unknown key or a value its command would refuse; nothing is written then.
pub async fn set_from_user(db: &Db, key: &str, value: &serde_json::Value) -> Result<()> {
    let known = USER_KEYS.contains(&key) || USER_PREFIXES.iter().any(|p| key.starts_with(p));
    if !known {
        return Err(Error::Invalid(format!("unknown setting `{key}`")));
    }
    let bad = |what: &str| Error::Invalid(format!("setting {key} must be {what}"));
    match key {
        "issue_prefix" => {
            let s = value.as_str().ok_or_else(|| bad("a key"))?;
            workspace::validate_key(s)?;
        }
        "timezone" => {
            let s = value.as_str().ok_or_else(|| bad("a time zone"))?;
            s.parse::<chrono_tz::Tz>()
                .map_err(|_| Error::Invalid(format!("unknown time zone {s}")))?;
        }
        "workspace_name" => {
            if value.as_str().is_none_or(|s| s.trim().is_empty()) {
                return Err(bad("a non-empty name"));
            }
        }
        "theme" => check_one_of(key, value, &["system", "light", "dark"])?,
        "week_start" => check_one_of(key, value, &["sunday", "monday"])?,
        "default_view_template" if !(value.is_null() || value.is_i64()) => {
            return Err(bad("a view template id or null"));
        }
        _ => {}
    }
    set(db, key, value).await
}

fn check_one_of(key: &str, value: &serde_json::Value, allowed: &[&str]) -> Result<()> {
    let ok = value.as_str().is_some_and(|v| allowed.contains(&v));
    if ok {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "setting {key} must be one of {}",
            allowed.join(", ")
        )))
    }
}
