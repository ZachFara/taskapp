use std::path::Path;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, de::DeserializeOwned};
use taskapp_core::{Due, Error, Result};
use tokio::process::Command;

pub struct Reminder {
    pub external_id: String,
    pub title: String,
    pub notes: Option<String>,
    pub due: Option<Due>,
}

#[derive(Deserialize)]
struct RawReminder {
    external_id: String,
    title: String,
    notes: Option<String>,
    due: Option<RawDue>,
}

#[derive(Deserialize)]
struct RawDue {
    date: Option<NaiveDate>,
    datetime: Option<DateTime<Utc>>,
    tz: Option<String>,
}

#[derive(Deserialize)]
struct Deleted {
    deleted: Vec<String>,
    missing: Vec<String>,
}

/// Runs one helper subcommand. The helper prints one JSON value; `{"error": ...}` means failure.
async fn run<T: DeserializeOwned>(helper: &Path, args: &[&str]) -> Result<T> {
    let out = Command::new(helper).args(args).output().await?;
    let value: serde_json::Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| Error::Invalid(format!("reminders helper printed invalid JSON: {e}")))?;
    if let Some(msg) = value.get("error").and_then(|m| m.as_str()) {
        return Err(Error::Invalid(format!("reminders helper: {msg}")));
    }
    if !out.status.success() {
        return Err(Error::Invalid(format!(
            "reminders helper exited with {}",
            out.status
        )));
    }
    serde_json::from_value(value)
        .map_err(|e| Error::Invalid(format!("reminders helper output: {e}")))
}

/// `(id, name)` of each Reminders list.
///
/// # Errors
///
/// Fails when the helper cannot run or reports an error.
pub async fn list_reminder_lists(helper: &Path) -> Result<Vec<(String, String)>> {
    #[derive(Deserialize)]
    struct L {
        id: String,
        title: String,
    }
    #[derive(Deserialize)]
    struct Access {
        granted: bool,
    }
    // Without a granted request macOS answers with no lists at all, so ask first (it prompts once).
    let access: Access = run(helper, &["authorize"]).await?;
    if !access.granted {
        return Err(Error::Invalid(
            "Reminders access is off: allow Cairn (taskapp) in System Settings > Privacy & Security > Reminders, then press Authorize again".into(),
        ));
    }
    let lists: Vec<L> = run(helper, &["lists"]).await?;
    Ok(lists.into_iter().map(|l| (l.id, l.title)).collect())
}

pub(crate) async fn fetch(helper: &Path, list: &str) -> Result<Vec<Reminder>> {
    let raw: Vec<RawReminder> = run(helper, &["fetch", "--list", list]).await?;
    raw.into_iter()
        .map(|r| {
            let due = match r.due {
                None => None,
                Some(RawDue { date: Some(d), .. }) => Some(Due::Date(d)),
                Some(RawDue {
                    datetime: Some(utc),
                    tz: Some(tz),
                    ..
                }) => Some(Due::DateTime { utc, tz }),
                Some(_) => {
                    return Err(Error::Invalid(format!(
                        "reminder {}: malformed due",
                        r.external_id
                    )));
                }
            };
            Ok(Reminder {
                external_id: r.external_id,
                title: r.title,
                notes: r.notes,
                due,
            })
        })
        .collect()
}

/// Deletes the reminders in one call. Returns every id that is gone, deleted now or already missing.
pub(crate) async fn delete(helper: &Path, list: &str, ids: &[String]) -> Result<Vec<String>> {
    let mut args = vec!["delete", "--list", list];
    args.extend(ids.iter().map(String::as_str));
    let d: Deleted = run(helper, &args).await?;
    Ok(d.deleted.into_iter().chain(d.missing).collect())
}
