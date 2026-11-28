//! `taskapp plan`: the day's time blocks (events linked to issues) with planned vs done, and `plan add` to make one.

use chrono::{Duration, NaiveDate, NaiveTime};
use clap::{Args, Subcommand};
use serde_json::{Value, json};
use taskapp_core::{ChangeSource, Db, Error, EventInput, Result, events, issues};

#[derive(Args)]
pub struct PlanArgs {
    #[command(subcommand)]
    cmd: Option<PlanCmd>,
    /// YYYY-MM-DD; today when left out.
    #[arg(long, global = true)]
    date: Option<String>,
}

#[derive(Subcommand)]
pub enum PlanCmd {
    /// Time-block an issue: creates an event linked to it (it syncs to Google Calendar like any event).
    Add {
        key: String,
        /// Start, HH:MM.
        #[arg(long)]
        at: String,
        /// Length: 90m, 2h, 1h30m or plain minutes.
        #[arg(long = "for")]
        length: String,
    },
}

/// 90m, 2h, 1h30m or plain minutes.
fn minutes(text: &str) -> Result<i64> {
    let bad = || Error::Invalid(format!("not a length (90m, 2h, 1h30m): {text}"));
    let (h, m) = match text.split_once('h') {
        Some((h, m)) => (h, m.trim_end_matches('m')),
        None => ("0", text.trim_end_matches('m')),
    };
    let part = |s: &str| {
        if s.is_empty() {
            Ok(0)
        } else {
            s.parse::<i64>().map_err(|_| bad())
        }
    };
    let total = part(h)? * 60 + part(m)?;
    if total <= 0 {
        return Err(bad());
    }
    Ok(total)
}

pub async fn run(db: &Db, args: PlanArgs) -> Result<Value> {
    let day: NaiveDate = match &args.date {
        Some(d) => crate::commands::date(d)?,
        None => crate::inbox::today(db).await?,
    };
    if let Some(PlanCmd::Add { key, at, length }) = args.cmd {
        let issue = issues::get_by_key(db, &key).await?;
        let start = day.and_time(
            NaiveTime::parse_from_str(&at, "%H:%M")
                .map_err(|_| Error::Invalid(format!("not a time (HH:MM): {at}")))?,
        );
        let input = EventInput {
            title: format!("{} {}", issue.key, issue.title),
            starts_at: start,
            ends_at: Some(start + Duration::minutes(minutes(&length)?)),
            all_day: false,
            place: None,
            notes: String::new(),
            icon: None,
            color: None,
            archived: false,
        };
        let event = events::create(db, input, ChangeSource::Cli).await?;
        events::link(db, event.id, issue.id, ChangeSource::Cli).await?;
        return serde_json::to_value(events::get(db, event.id).await?)
            .map_err(|e| Error::Invalid(e.to_string()));
    }
    let (mut blocks, mut planned, mut done) = (vec![], 0, 0);
    for e in events::list(db, Some(day), Some(day)).await? {
        if e.all_day || e.issue_ids.is_empty() {
            continue;
        }
        let mut linked = vec![];
        for id in &e.issue_ids {
            linked.push(issues::get(db, *id).await?);
        }
        let mins = e.ends_at.map_or(0, |end| (end - e.starts_at).num_minutes());
        let finished = linked.iter().all(|i| i.completed_at.is_some());
        planned += mins;
        if finished {
            done += mins;
        }
        blocks.push(json!({
            "event_id": e.id, "start": e.starts_at, "end": e.ends_at, "minutes": mins, "done": finished,
            "issues": linked.iter().map(|i| json!({ "key": i.key, "title": i.title, "done": i.completed_at.is_some() })).collect::<Vec<_>>(),
        }));
    }
    Ok(json!({ "date": day, "blocks": blocks, "planned_minutes": planned, "done_minutes": done }))
}

#[cfg(test)]
mod tests {
    use super::minutes;

    #[test]
    fn lengths() {
        assert_eq!(minutes("90m").expect("ok"), 90);
        assert_eq!(minutes("2h").expect("ok"), 120);
        assert_eq!(minutes("1h30m").expect("ok"), 90);
        assert_eq!(minutes("45").expect("ok"), 45);
        assert!(minutes("0m").is_err() && minutes("abc").is_err() && minutes("").is_err());
    }
}
