//! First-run setup: the workspace name, the issue key prefix and the time zone the owner picks once.

use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::{Db, Error, Result, settings};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct WorkspaceStatus {
    pub setup_done: bool,
    pub name: Option<String>,
    pub key: String,
    pub timezone: String,
}

/// IANA name of the system time zone, from the `/etc/localtime` link; UTC when the link is missing.
#[must_use]
pub fn system_tz() -> String {
    std::fs::read_link("/etc/localtime")
        .ok()
        .and_then(|p| {
            p.to_str()
                .and_then(|s| s.split_once("zoneinfo/").map(|(_, tz)| tz.to_owned()))
        })
        .unwrap_or_else(|| "UTC".to_owned())
}

/// # Errors
/// `Invalid` unless the key is 2 to 5 capital ASCII letters.
pub fn validate_key(key: &str) -> Result<()> {
    if (2..=5).contains(&key.len()) && key.bytes().all(|b| b.is_ascii_uppercase()) {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "issue key `{key}` must be 2 to 5 letters A-Z"
        )))
    }
}

/// # Errors
/// Fails when a setting is unreadable.
pub async fn status(db: &Db) -> Result<WorkspaceStatus> {
    let key = settings::get::<String>(db, "issue_prefix")
        .await?
        .ok_or_else(|| Error::Invalid("issue_prefix setting is missing".into()))?;
    Ok(WorkspaceStatus {
        setup_done: settings::get(db, "setup_done").await?.unwrap_or(false),
        name: settings::get(db, "workspace_name").await?,
        key,
        timezone: match settings::get::<String>(db, "timezone").await? {
            Some(tz) => tz,
            None => system_tz(),
        },
    })
}

/// Saves the first-run choices. Without a zone, the stored zone stays (or the system zone is used).
/// Changing the key changes only the display prefix; issue numbers stay.
///
/// # Errors
/// `Invalid` for an empty name, a bad key or an unknown zone; nothing is written then.
pub async fn setup(
    db: &Db,
    name: &str,
    key: &str,
    timezone: Option<&str>,
) -> Result<WorkspaceStatus> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::Invalid("workspace name must not be empty".into()));
    }
    validate_key(key)?;
    if let Some(tz) = timezone {
        tz.parse::<Tz>()
            .map_err(|_| Error::Invalid(format!("unknown time zone {tz}")))?;
    }
    let zone = match timezone {
        Some(tz) => tz.to_owned(),
        None => status(db).await?.timezone,
    };
    let keys: Vec<String> = sqlx::query_scalar("SELECT key FROM property_defs ORDER BY position")
        .fetch_all(&db.pool)
        .await?;
    crate::view_templates::seed(db, &keys).await?;
    settings::set(db, "workspace_name", &name).await?;
    settings::set(db, "issue_prefix", &key).await?;
    settings::set(db, "timezone", &zone).await?;
    settings::set(db, "setup_done", &true).await?;
    status(db).await
}
