//! The end-of-day shutdown: how much of the Inbox is left, up to three deep issues chosen for tomorrow, and the days
//! it was finished. Kept in two settings (`shutdown_tomorrow`, `shutdown_days`), so there is no table to migrate.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{Db, Depth, Error, Issue, IssueId, Result, inbox, issues, settings};

/// The plan made on `set_on`, for the day after it.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Tomorrow {
    set_on: NaiveDate,
    ids: Vec<i64>,
}

/// Most issues the ritual puts on tomorrow.
const MAX_TOMORROW: usize = 3;

/// Where the ritual stands today, with the lists it shows.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
pub struct ShutdownState {
    pub inbox_count: i64,
    /// Open issues marked deep, to choose tomorrow's from.
    pub deep: Vec<Issue>,
    /// Chosen today for tomorrow.
    pub tomorrow: Vec<Issue>,
    pub done_today: bool,
    /// Days in a row finished, ending today (or yesterday, until today is finished).
    pub streak: i64,
}

async fn days(db: &Db) -> Result<Vec<NaiveDate>> {
    Ok(settings::get(db, "shutdown_days")
        .await?
        .unwrap_or_default())
}

async fn issue_list(db: &Db, ids: impl IntoIterator<Item = i64>) -> Result<Vec<Issue>> {
    let mut out = Vec::new();
    for id in ids {
        out.push(issues::get(db, IssueId(id)).await?);
    }
    Ok(out)
}

/// The ritual as it stands on `today` (the workspace date).
///
/// # Errors
///
/// Fails on a database error or a stored value that does not parse.
pub async fn state(db: &Db, today: NaiveDate) -> Result<ShutdownState> {
    let inbox_count = i64::try_from(inbox::list(db, today).await?.len()).unwrap_or(i64::MAX);
    let deep_ids: Vec<i64> = sqlx::query_scalar(
        "SELECT i.id FROM issues i JOIN statuses s ON s.id = i.status_id
         WHERE i.archived_at IS NULL AND i.depth = 'deep' AND s.category NOT IN ('completed', 'canceled')
         ORDER BY i.number",
    )
    .fetch_all(&db.pool)
    .await?;
    let plan: Option<Tomorrow> = settings::get(db, "shutdown_tomorrow").await?;
    let tomorrow = match plan {
        Some(p) if p.set_on == today => issue_list(db, p.ids).await?,
        _ => Vec::new(),
    };
    let done = days(db).await?;
    let done_today = done.contains(&today);
    let mut day = if done_today {
        today
    } else {
        today.pred_opt().unwrap_or(today)
    };
    let mut streak = 0;
    while done.contains(&day) {
        streak += 1;
        day = day.pred_opt().unwrap_or(day);
    }
    Ok(ShutdownState {
        inbox_count,
        deep: issue_list(db, deep_ids).await?,
        tomorrow,
        done_today,
        streak,
    })
}

/// Replaces tomorrow's plan with `ids`: up to three open issues marked deep.
///
/// # Errors
///
/// `Invalid` for more than three, an issue that is not deep or is closed, or an unknown issue; nothing is written then.
pub async fn set_tomorrow(db: &Db, today: NaiveDate, ids: &[IssueId]) -> Result<ShutdownState> {
    if ids.len() > MAX_TOMORROW {
        return Err(Error::Invalid(format!(
            "choose up to {MAX_TOMORROW} issues for tomorrow"
        )));
    }
    for id in ids {
        let issue = issues::get(db, *id).await?;
        if issue.depth != Some(Depth::Deep) || issue.archived_at.is_some() {
            return Err(Error::Invalid(format!(
                "{} is not an open deep issue",
                issue.key
            )));
        }
    }
    let plan = Tomorrow {
        set_on: today,
        ids: ids.iter().map(|i| i.0).collect(),
    };
    settings::set(db, "shutdown_tomorrow", &plan).await?;
    state(db, today).await
}

/// Records today as finished. Finishing again the same day changes nothing.
///
/// # Errors
///
/// Fails on a database error.
pub async fn complete(db: &Db, today: NaiveDate) -> Result<ShutdownState> {
    let mut done = days(db).await?;
    if !done.contains(&today) {
        done.push(today);
        done.sort();
        settings::set(db, "shutdown_days", &done).await?;
    }
    state(db, today).await
}
