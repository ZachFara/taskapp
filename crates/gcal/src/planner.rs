//! Pure: turns issues into the calendar events that should exist, and diffs against what does.

use chrono::{DateTime, NaiveDate, Utc};
use taskapp_core::{Issue, Status, WorkBlock};

pub struct DesiredEvent {
    pub managed_key: String,
    pub summary: String,
    pub description: String,
    pub timing: EventTiming,
    pub reminder_minutes: Option<u32>,
}

pub enum EventTiming {
    AllDay(NaiveDate),
    Timed {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
}

pub struct ManagedEvent {
    pub google_event_id: String,
    pub managed_key: String,
    pub last_synced_hash: String,
}

pub enum EventOp {
    Create(DesiredEvent),
    Update {
        google_event_id: String,
        event: DesiredEvent,
    },
    Delete {
        google_event_id: String,
    },
}

#[must_use]
pub fn desired(
    _issues: &[Issue],
    _statuses: &[Status],
    _blocks: &[WorkBlock],
    _tz: &str,
    _issue_prefix: &str,
) -> Vec<DesiredEvent> {
    todo!("T8")
}

#[must_use]
pub fn plan(_desired: &[DesiredEvent], _existing: &[ManagedEvent]) -> Vec<EventOp> {
    todo!("T8")
}

#[must_use]
pub fn hash(_event: &DesiredEvent) -> String {
    todo!("T8")
}
