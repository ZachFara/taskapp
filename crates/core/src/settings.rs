use std::path::{Path, PathBuf};

use chrono::NaiveDate;
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
    "issue_fields",
    "insights_charts",
    "dashboard",
    "sidebar_sparklines",
    "reminders_list_id",
    "gcal_enabled",
    "gcal_calendar_id",
    "export_enabled",
    "export_dir",
    "export_remote",
    "backup_dir",
    "default_view_template",
    "default_status",
    "last_review",
];

/// Prefixes of keys the UI writes per item: `insights.<kind>.<id>` and `sidebar_*`.
const USER_PREFIXES: &[&str] = &["insights.", "sidebar_", "feature_"];

/// Optional features that can be switched off (name, setting key); all default to on. Off hides the feature, data stays.
/// The names are what `taskapp features set` takes and what `ui/src/lib/features.svelte.ts` lists.
pub const FEATURES: &[(&str, &str)] = &[
    ("dashboard", "dashboard"),
    ("decisions", "feature_decisions"),
    ("cycles", "feature_cycles"),
    ("events", "feature_events"),
    ("charts", "insights_charts"),
    ("stale", "feature_stale"),
    ("reminders", "feature_reminders"),
    ("gcal", "feature_gcal"),
    ("process_inbox", "feature_process_inbox"),
    ("weekly_review", "feature_weekly_review"),
    ("next_actions", "feature_next_actions"),
    ("focus", "feature_focus"),
    ("recurring", "feature_recurring"),
    ("time_blocking", "feature_time_blocking"),
    ("shutdown", "feature_shutdown"),
    ("goals", "feature_goals"),
];

/// Switches that start off; a never-written one is off.
const OFF_BY_DEFAULT: &[&str] = &["feature_shutdown"];

/// Every feature with whether it is on (a never-written switch is on).
///
/// # Errors
/// Fails when a stored value is not true or false.
pub async fn features(db: &Db) -> Result<Vec<(&'static str, bool)>> {
    let mut out = Vec::new();
    for (name, key) in FEATURES {
        out.push((*name, feature_on(db, key).await?));
    }
    Ok(out)
}

/// Whether one feature switch (its setting key) is on; a never-written switch is on unless it starts off.
///
/// # Errors
/// Fails when the stored value is not true or false.
pub async fn feature_on(db: &Db, key: &str) -> Result<bool> {
    Ok(get::<bool>(db, key)
        .await?
        .unwrap_or(!OFF_BY_DEFAULT.contains(&key)))
}

/// # Errors
/// `Invalid` for an unknown feature name.
pub async fn set_feature(db: &Db, name: &str, on: bool) -> Result<()> {
    let (_, key) = FEATURES.iter().find(|(n, _)| *n == name).ok_or_else(|| {
        let names: Vec<_> = FEATURES.iter().map(|(n, _)| *n).collect();
        Error::Invalid(format!(
            "unknown feature `{name}`; choose from {}",
            names.join(", ")
        ))
    })?;
    set_from_user(db, key, &serde_json::Value::Bool(on)).await
}

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
        k if k == "dashboard" || k == "insights_charts" || k.starts_with("feature_") => {
            if !value.is_boolean() {
                return Err(bad("true or false"));
            }
        }
        "theme" => check_one_of(key, value, &["system", "light", "dark"])?,
        "week_start" => check_one_of(key, value, &["sunday", "monday"])?,
        "issue_fields" => check_issue_fields(value)?,
        "default_status" => {
            let id = value.as_i64().ok_or_else(|| bad("a status id"))?;
            if !crate::statuses::list(db)
                .await?
                .iter()
                .any(|st| st.id.0 == id)
            {
                return Err(bad("an existing status"));
            }
        }
        "last_review" => {
            let day = value
                .as_str()
                .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
            if day.is_none() {
                return Err(bad("a date (YYYY-MM-DD)"));
            }
        }
        "default_view_template" if !(value.is_null() || value.is_i64()) => {
            return Err(bad("a view template id or null"));
        }
        _ => {}
    }
    set(db, key, value).await
}

/// The built-in issue fields a workspace may switch off; status and title are always on.
pub const ISSUE_FIELDS: &[&str] = &[
    "priority",
    "project",
    "milestone",
    "cycle",
    "labels",
    "start",
    "due",
    "estimate",
    "points",
    "parent",
];

/// `issue_fields` is an ordered list of `{field, on}`: known names only, each at most once.
fn check_issue_fields(value: &serde_json::Value) -> Result<()> {
    let bad = |why: String| Error::Invalid(format!("setting issue_fields: {why}"));
    let list = value
        .as_array()
        .ok_or_else(|| bad("must be a list of {field, on}".into()))?;
    let mut seen = Vec::new();
    for item in list {
        let name = item.get("field").and_then(|f| f.as_str());
        let on = item.get("on").is_some_and(serde_json::Value::is_boolean);
        match name {
            Some(n) if on && ISSUE_FIELDS.contains(&n) && !seen.contains(&n) => seen.push(n),
            Some(n) if on => return Err(bad(format!("unknown or repeated field `{n}`"))),
            _ => {
                return Err(bad(
                    "each entry needs a field name and a true or false `on`".into(),
                ));
            }
        }
    }
    Ok(())
}

/// Fields quick add must not read tokens for: those switched off in `issue_fields`, and points when `estimation`
/// has them off.
///
/// # Errors
/// Fails on a database error.
pub async fn fields_off(db: &Db) -> Result<Vec<String>> {
    let list = get::<Vec<serde_json::Value>>(db, "issue_fields")
        .await?
        .unwrap_or_default();
    let mut off: Vec<String> = list
        .iter()
        .filter(|i| i.get("on") == Some(&serde_json::Value::Bool(false)))
        .filter_map(|i| i.get("field")?.as_str().map(String::from))
        .collect();
    let est = get::<String>(db, "estimation").await?;
    if matches!(est.as_deref(), Some("none" | "time")) {
        off.push("points".into());
    }
    Ok(off)
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

/// The folder a person typed in a path setting: `~` and `~/...` mean the home folder; anything else must be a full
/// path, because a relative one would resolve against the app's working folder (the disk root when opened from Finder).
///
/// # Errors
///
/// `Invalid` when the text is empty, relative, or names another user's home (`~name`).
pub fn user_path(text: &str, home: &Path) -> Result<PathBuf> {
    let text = text.trim();
    if text == "~" {
        return Ok(home.to_path_buf());
    }
    if let Some(rest) = text.strip_prefix("~/") {
        return Ok(home.join(rest));
    }
    let path = Path::new(text);
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    Err(Error::Invalid(format!(
        "\"{text}\" is not a full path: start it with / or ~/"
    )))
}

/// [`user_path`] with the current user's home folder.
///
/// # Errors
///
/// As [`user_path`], and `Invalid` when the home folder is unknown.
pub fn home_path(text: &str) -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| Error::Invalid("the home folder is unknown".into()))?;
    user_path(text, &home)
}
