//! `taskapp inbox`: the next Inbox items, oldest first, and `inbox process` to clear one (the app's Process Inbox).

use chrono::{NaiveDate, Utc};
use clap::{Args, Subcommand};
use serde_json::{Value, json};
use taskapp_core::{ChangeSource, Db, Error, InboxChoice, Result, inbox, issues};

#[derive(Args)]
pub struct InboxArgs {
    #[command(subcommand)]
    cmd: Option<InboxCmd>,
    /// How many items to list.
    #[arg(long, default_value_t = 5)]
    limit: usize,
}

#[derive(Subcommand)]
pub enum InboxCmd {
    /// Clears one Inbox issue with exactly one choice.
    Process {
        key: String,
        /// Done (the two-minute rule).
        #[arg(long = "do", group = "choice")]
        done: bool,
        /// Delegated, with a "Waiting on: NOTE" comment.
        #[arg(long, group = "choice", value_name = "NOTE")]
        delegate: Option<String>,
        /// Hide it until this later day, YYYY-MM-DD.
        #[arg(long, group = "choice", value_name = "DATE")]
        defer: Option<String>,
        /// Status Next, in this project (id, slug or name).
        #[arg(long, group = "choice", value_name = "PROJECT")]
        next: Option<String>,
        /// With --next: also make it the project's next action.
        #[arg(long = "as-next-action", requires = "next")]
        as_next_action: bool,
        #[arg(long, group = "choice")]
        someday: bool,
        /// Due on this day, YYYY-MM-DD; status Reminder (or the first unstarted status), so it reaches the calendar.
        #[arg(long, group = "choice", value_name = "DATE")]
        schedule: Option<String>,
        /// With --schedule: the time of day, HH:MM in the workspace zone.
        #[arg(long, requires = "schedule", value_name = "HH:MM")]
        at: Option<String>,
        /// Archive it.
        #[arg(long, group = "choice")]
        trash: bool,
    },
}

pub(crate) async fn today(db: &Db) -> Result<NaiveDate> {
    let tz: String = taskapp_core::settings::get(db, "timezone")
        .await?
        .unwrap_or_else(|| "UTC".into());
    let zone: chrono_tz::Tz = tz
        .parse()
        .map_err(|_| Error::Invalid(format!("unknown time zone {tz}")))?;
    Ok(Utc::now().with_timezone(&zone).date_naive())
}

pub async fn run(db: &Db, args: InboxArgs) -> Result<Value> {
    let today = today(db).await?;
    let Some(InboxCmd::Process {
        key,
        done,
        delegate,
        defer,
        next,
        as_next_action,
        someday,
        schedule,
        at,
        trash,
    }) = args.cmd
    else {
        let all = inbox::list(db, today).await?;
        let shown: Vec<_> = all.iter().take(args.limit).collect();
        return Ok(json!({ "total": all.len(), "items": shown }));
    };
    // clap's `choice` group already allows at most one; none at all is refused below.
    let choice = if done {
        InboxChoice::Done
    } else if let Some(note) = delegate {
        InboxChoice::Delegate { note }
    } else if let Some(d) = defer {
        InboxChoice::Defer {
            until: crate::commands::date(&d)?,
        }
    } else if let Some(p) = next {
        InboxChoice::Next {
            project: crate::references::project(db, Some(&p)).await?,
            as_next_action,
        }
    } else if someday {
        InboxChoice::Someday
    } else if let Some(d) = schedule {
        InboxChoice::Schedule {
            date: crate::commands::date(&d)?,
            time: at,
        }
    } else if trash {
        InboxChoice::Trash
    } else {
        return Err(Error::Invalid(
            "pass exactly one of --do, --delegate, --defer, --next, --someday, --schedule, --trash"
                .into(),
        ));
    };
    let id = issues::get_by_key(db, &key).await?.id;
    let issue = inbox::process(db, id, choice, today, ChangeSource::Cli).await?;
    serde_json::to_value(issue).map_err(|e| Error::Invalid(e.to_string()))
}
