//! Pure: turns issues into the calendar events that should exist, and diffs against what does.

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use sha2::{Digest, Sha256};
use taskapp_core::{Due, Event, Issue, Project, Status, StatusCategory, WorkBlock};

/// Digest notification time, local to the user's zone.
pub const DIGEST_NOTIFY_LOCAL: NaiveTime = NaiveTime::from_hms_opt(8, 0, 0).expect("valid time");

#[derive(Debug, Clone, PartialEq)]
pub struct DesiredEvent {
    pub managed_key: String,
    pub summary: String,
    pub description: String,
    pub timing: EventTiming,
    pub reminder_minutes: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventTiming {
    AllDay(NaiveDate),
    /// First day and the day after the last (Google's exclusive end), for an all-day event over several days.
    AllDaySpan(NaiveDate, NaiveDate),
    Timed {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManagedEvent {
    pub google_event_id: String,
    pub managed_key: String,
    pub last_synced_hash: String,
}

#[derive(Debug, Clone, PartialEq)]
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

/// The encoding of the digest reminder is decided by a later probe of the real Google API.
#[must_use]
pub fn digest_reminder_minutes() -> Option<u32> {
    None
}

/// An unknown zone falls back to UTC so the timed event still exists.
#[must_use]
pub fn desired(
    issues: &[Issue],
    statuses: &[Status],
    projects: &[Project],
    blocks: &[WorkBlock],
    tz: &str,
    _issue_prefix: &str,
) -> Vec<DesiredEvent> {
    let zone: Tz = tz.parse().unwrap_or(chrono_tz::UTC);
    let open: Vec<&Issue> = issues.iter().filter(|i| is_open(i, statuses)).collect();
    let mut events: Vec<DesiredEvent> = open
        .iter()
        .filter_map(|i| timed_event(i, projects, zone))
        .collect();
    events.extend(digest_events(&open, projects, zone));
    events.extend(block_events(blocks, issues, statuses, projects));
    events
}

/// Cairn events (not archived: `events::list` leaves those out). Naive local times become UTC in `tz`.
/// Linked issues come from `issues`; a link to an issue not in it is left out.
/// An event has no reminder in Cairn, so none is written; no end means one hour; "shortcut: a time
/// skipped by a clock change takes the zone's later reading.
#[must_use]
pub fn event_events(events: &[Event], issues: &[Issue], tz: &str) -> Vec<DesiredEvent> {
    let zone: Tz = tz.parse().unwrap_or(chrono_tz::UTC);
    let utc = |t| {
        zone.from_local_datetime(&t)
            .earliest()
            .or_else(|| zone.from_local_datetime(&t).latest())
            .map_or_else(|| t.and_utc(), |z| z.with_timezone(&Utc))
    };
    events
        .iter()
        .map(|e| {
            let day = e.starts_at.date();
            let last = e.ends_at.map_or(day, |t| t.date().max(day));
            let timing = match (e.all_day, last > day) {
                (true, false) => EventTiming::AllDay(day),
                (true, true) => EventTiming::AllDaySpan(day, last + Duration::days(1)),
                (false, _) => {
                    let start = utc(e.starts_at);
                    let end = e.ends_at.map_or(start + Duration::hours(1), utc);
                    EventTiming::Timed { start, end }
                }
            };
            let linked = e.issue_ids.iter().filter_map(|id| {
                issues
                    .iter()
                    .find(|i| i.id == *id)
                    .map(|i| format!("{} {}: taskapp://issue/{}", i.key, i.title, i.key))
            });
            let description = [
                e.place.as_ref().map(|p| format!("Place: {p}")),
                Some(e.notes.clone()),
            ]
            .into_iter()
            .flatten()
            .filter(|t| !t.is_empty())
            .chain(linked)
            .collect::<Vec<_>>()
            .join("\n");
            DesiredEvent {
                managed_key: format!("event:{}", e.id.0),
                summary: e.title.clone(),
                description,
                timing,
                reminder_minutes: None,
            }
        })
        .collect()
}

fn is_open(issue: &Issue, statuses: &[Status]) -> bool {
    issue.archived_at.is_none()
        && !statuses.iter().any(|s| {
            s.id == issue.status_id
                && matches!(
                    s.category,
                    StatusCategory::Completed | StatusCategory::Canceled
                )
        })
}

fn timed_event(issue: &Issue, projects: &[Project], zone: Tz) -> Option<DesiredEvent> {
    let utc = match issue.due.as_ref()? {
        Due::DateTime { utc, .. } => *utc,
        Due::Date(_) => return None,
    };
    Some(DesiredEvent {
        managed_key: format!("issue:{}", issue.id.0),
        summary: format!("{} {}", issue.key, issue.title),
        description: format!(
            "{}\nDue {}",
            header(issue, projects),
            utc.with_timezone(&zone).format("%Y-%m-%d %H:%M %Z")
        ),
        timing: EventTiming::Timed {
            start: utc,
            end: utc + Duration::minutes(15),
        },
        reminder_minutes: Some(0),
    })
}

/// An all-day event cannot carry an 08:00 reminder (Google counts minutes back from midnight and
/// rejects negatives), so each digest is paired with a short timed event at 08:00 local that carries the popup.
fn digest_events(open: &[&Issue], projects: &[Project], zone: Tz) -> Vec<DesiredEvent> {
    let mut by_date: BTreeMap<NaiveDate, Vec<&Issue>> = BTreeMap::new();
    for issue in open {
        if let Some(Due::Date(date)) = &issue.due {
            by_date.entry(*date).or_default().push(issue);
        }
    }
    by_date
        .into_iter()
        .flat_map(|(date, group)| {
            let summary = match group.len() {
                1 => "Due today: 1 task".to_string(),
                n => format!("Due today: {n} tasks"),
            };
            let description = group
                .iter()
                .map(|i| header(i, projects))
                .collect::<Vec<_>>()
                .join("\n\n");
            let alert = zone
                .from_local_datetime(&date.and_time(DIGEST_NOTIFY_LOCAL))
                .earliest()
                .map(|t| t.with_timezone(&Utc))
                .map(|start| DesiredEvent {
                    managed_key: format!("digest-alert:{date}"),
                    summary: summary.clone(),
                    description: description.clone(),
                    timing: EventTiming::Timed {
                        start,
                        end: start + Duration::minutes(15),
                    },
                    reminder_minutes: Some(0),
                });
            let digest = DesiredEvent {
                managed_key: format!("digest:{date}"),
                summary,
                description,
                timing: EventTiming::AllDay(date),
                reminder_minutes: digest_reminder_minutes(),
            };
            std::iter::once(digest).chain(alert)
        })
        .collect()
}

fn block_events(
    blocks: &[WorkBlock],
    issues: &[Issue],
    statuses: &[Status],
    projects: &[Project],
) -> Vec<DesiredEvent> {
    blocks
        .iter()
        .filter_map(|b| {
            let issue = issues
                .iter()
                .find(|i| i.id == b.issue_id)
                .filter(|i| is_open(i, statuses))?;
            Some(DesiredEvent {
                managed_key: format!("block:{}", b.id.0),
                summary: issue.title.clone(),
                description: header(issue, projects),
                timing: EventTiming::Timed {
                    start: b.start_utc,
                    end: b.end_utc,
                },
                reminder_minutes: None,
            })
        })
        .collect()
}

/// Key, title, priority and project first, so a notification reads alone; then the link back.
fn header(issue: &Issue, projects: &[Project]) -> String {
    let project = issue
        .project_id
        .and_then(|id| projects.iter().find(|p| p.id == id))
        .map_or_else(String::new, |p| format!(" | Project: {}", p.name));
    format!(
        "{} {}\nPriority: {}{}\ntaskapp://issue/{}",
        issue.key,
        issue.title,
        priority_name(issue.priority),
        project,
        issue.key
    )
}

fn priority_name(priority: u8) -> &'static str {
    match priority {
        0 => "None",
        1 => "Urgent",
        2 => "High",
        3 => "Medium",
        4 => "Low",
        _ => "Unknown",
    }
}

#[must_use]
pub fn plan(desired: &[DesiredEvent], existing: &[ManagedEvent]) -> Vec<EventOp> {
    let mut by_key: BTreeMap<&str, &ManagedEvent> = existing
        .iter()
        .map(|e| (e.managed_key.as_str(), e))
        .collect();
    let mut ops = Vec::new();
    for event in desired {
        match by_key.remove(event.managed_key.as_str()) {
            None => ops.push(EventOp::Create(event.clone())),
            Some(found) if found.last_synced_hash != hash(event) => ops.push(EventOp::Update {
                google_event_id: found.google_event_id.clone(),
                event: event.clone(),
            }),
            Some(_) => {}
        }
    }
    ops.extend(by_key.into_values().map(|e| EventOp::Delete {
        google_event_id: e.google_event_id.clone(),
    }));
    ops
}

#[must_use]
pub fn hash(event: &DesiredEvent) -> String {
    let timing = match &event.timing {
        EventTiming::AllDay(date) => format!("day {date}"),
        EventTiming::AllDaySpan(a, b) => format!("days {a} {b}"),
        EventTiming::Timed { start, end } => {
            format!("timed {} {}", start.to_rfc3339(), end.to_rfc3339())
        }
    };
    let canonical = format!(
        "{}\n{}\n{}\n{:?}",
        event.summary, event.description, timing, event.reminder_minutes
    );
    format!("{:x}", Sha256::digest(canonical.as_bytes()))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use taskapp_core::{IssueId, ProjectId, StatusId, WorkBlockId};

    use super::*;

    const NY: &str = "Europe/Berlin";

    fn at(y: i32, m: u32, d: u32, h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, 0, 0)
            .single()
            .expect("valid utc time")
    }

    fn status(id: i64, category: StatusCategory) -> Status {
        Status {
            id: StatusId(id),
            name: format!("s{id}"),
            category,
            position: 0,
            color: String::new(),
            color_hex: String::new(),
            icon: None,
            definition: String::new(),
            flow_kind: taskapp_core::FlowKind::Idle,
            nudge_days: None,
        }
    }

    fn statuses() -> Vec<Status> {
        vec![
            status(1, StatusCategory::Started),
            status(2, StatusCategory::Completed),
            status(3, StatusCategory::Canceled),
        ]
    }

    fn project() -> Project {
        Project {
            id: ProjectId(7),
            slug: "home".into(),
            name: "Home".into(),
            description: None,
            doc_link: None,
            start_date: None,
            target_date: None,
            archived_at: None,
            created_at: chrono::DateTime::UNIX_EPOCH,
            updated_at: chrono::DateTime::UNIX_EPOCH,
            icon: None,
            color: None,
            status_id: taskapp_core::ProjectStatusId(1),
            priority: 0,
            label_ids: vec![],
            properties: serde_json::Map::new(),
        }
    }

    /// Open (status 1), priority 2, in project 7, unless the test changes it.
    fn issue(id: i64, due: Option<Due>) -> Issue {
        let now = at(2026, 11, 1, 0);
        Issue {
            id: IssueId(id),
            key: format!("ABC-{id}"),
            title: format!("Task {id}"),
            description: String::new(),
            status_id: StatusId(1),
            priority: 2,
            project_id: Some(ProjectId(7)),
            milestone_id: None,
            parent_id: None,
            cycle_id: None,
            label_ids: vec![],
            start_date: None,
            due,
            estimate_minutes: None,
            points: None,
            hidden_until: None,
            depth: None,
            repeat: None,
            properties: serde_json::Map::default(),
            created_at: now,
            updated_at: now,
            completed_at: None,
            archived_at: None,
        }
    }

    fn date(d: u32) -> Due {
        Due::Date(NaiveDate::from_ymd_opt(2026, 11, d).expect("valid date"))
    }

    fn run(issues: &[Issue], blocks: &[WorkBlock]) -> Vec<DesiredEvent> {
        desired(issues, &statuses(), &[project()], blocks, NY, "ABC")
    }

    /// Only the all-day digests, without their 08:00 alert events.
    fn run_digests(issues: &[Issue]) -> Vec<DesiredEvent> {
        let mut events = run(issues, &[]);
        events.retain(|e| e.managed_key.starts_with("digest:"));
        events
    }

    fn event(key: &str, minutes: Option<u32>) -> DesiredEvent {
        DesiredEvent {
            managed_key: key.into(),
            summary: "s".into(),
            description: "d".into(),
            timing: EventTiming::AllDay(NaiveDate::from_ymd_opt(2026, 11, 21).expect("valid date")),
            reminder_minutes: minutes,
        }
    }

    #[test]
    fn timed_due_is_15_min_with_popup() {
        let utc = at(2026, 11, 21, 15);
        let events = run(&[issue(1, Some(Due::DateTime { utc, tz: NY.into() }))], &[]);
        assert_eq!(events.len(), 1);
        let e = &events[0];
        assert_eq!(e.managed_key, "issue:1");
        assert_eq!(
            e.timing,
            EventTiming::Timed {
                start: utc,
                end: utc + Duration::minutes(15),
            }
        );
        assert_eq!(e.reminder_minutes, Some(0));
        assert!(
            e.description
                .starts_with("ABC-1 Task 1\nPriority: High | Project: Home\n")
        );
        assert!(e.description.contains("taskapp://issue/ABC-1"));
    }

    #[test]
    fn date_only_grouped_into_digest() {
        let events = run_digests(&[issue(1, Some(date(21))), issue(2, Some(date(21)))]);
        assert_eq!(events.len(), 1);
        let e = &events[0];
        assert_eq!(e.managed_key, "digest:2026-11-21");
        assert_eq!(e.summary, "Due today: 2 tasks");
        assert_eq!(
            e.timing,
            EventTiming::AllDay(NaiveDate::from_ymd_opt(2026, 11, 21).expect("valid date"))
        );
        assert!(
            e.description
                .contains("ABC-1 Task 1\nPriority: High | Project: Home\ntaskapp://issue/ABC-1")
        );
        assert!(
            e.description
                .contains("ABC-2 Task 2\nPriority: High | Project: Home\ntaskapp://issue/ABC-2")
        );
    }

    #[test]
    fn single_task_digest_singular() {
        let events = run_digests(&[issue(1, Some(date(21)))]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].summary, "Due today: 1 task");
        assert_eq!(events[0].reminder_minutes, digest_reminder_minutes());
    }

    #[test]
    fn completed_and_archived_excluded() {
        let mut archived = issue(2, Some(date(21)));
        archived.archived_at = Some(at(2026, 11, 2, 0));
        let mut completed = issue(3, Some(date(21)));
        completed.status_id = StatusId(2);
        let mut timed_completed = issue(
            1,
            Some(Due::DateTime {
                utc: at(2026, 11, 21, 15),
                tz: NY.into(),
            }),
        );
        timed_completed.status_id = StatusId(2);
        let events = run(&[timed_completed, archived, completed], &[]);
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn canceled_status_gets_no_event() {
        let mut canceled = issue(1, Some(date(21)));
        canceled.status_id = StatusId(3);
        assert_eq!(run(&[canceled], &[]).len(), 0);
    }

    #[test]
    fn no_due_no_event() {
        assert_eq!(run(&[issue(1, None)], &[]).len(), 0);
    }

    #[test]
    fn digest_uses_local_date() {
        let utc = at(2026, 11, 20, 23);
        let events = run(
            &[
                issue(1, Some(Due::DateTime { utc, tz: NY.into() })),
                issue(2, Some(date(21))),
                issue(3, Some(date(21))),
            ],
            &[],
        );
        let timed = events
            .iter()
            .find(|e| e.managed_key == "issue:1")
            .expect("timed event");
        assert!(timed.description.contains("2026-11-21 00:00 CET"));
        let digest = events
            .iter()
            .find(|e| e.managed_key == "digest:2026-11-21")
            .expect("digest on the 21st");
        assert_eq!(digest.summary, "Due today: 2 tasks");
    }

    #[test]
    fn date_moved_to_timed_removes_from_digest() {
        let before = run(&[issue(1, Some(date(21)))], &[]);
        assert!(before.iter().any(|e| e.managed_key == "digest:2026-11-21"));
        let after = run(
            &[issue(
                1,
                Some(Due::DateTime {
                    utc: at(2026, 11, 21, 15),
                    tz: NY.into(),
                }),
            )],
            &[],
        );
        assert!(after.iter().all(|e| !e.managed_key.starts_with("digest:")));
    }

    #[test]
    fn two_digests_on_different_dates_are_separate_events() {
        let events = run_digests(&[issue(1, Some(date(21))), issue(2, Some(date(22)))]);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].managed_key, "digest:2026-11-21");
        assert_eq!(events[0].summary, "Due today: 1 task");
        assert_eq!(events[1].managed_key, "digest:2026-11-22");
        assert_eq!(events[1].summary, "Due today: 1 task");
    }

    #[test]
    fn work_block_event_has_no_reminder() {
        let block = WorkBlock {
            id: WorkBlockId(5),
            issue_id: IssueId(1),
            start_utc: at(2026, 11, 21, 9),
            end_utc: at(2026, 11, 21, 11),
        };
        let events = run(&[issue(1, None)], &[block]);
        assert_eq!(events.len(), 1);
        let e = &events[0];
        assert_eq!(e.managed_key, "block:5");
        assert_eq!(e.summary, "Task 1");
        assert_eq!(e.reminder_minutes, None);
    }

    #[test]
    fn digest_has_a_timed_0800_alert_beside_it() {
        // Winter: 08:00 in Berlin is 07:00 UTC.
        let events = run(&[issue(1, Some(date(21))), issue(2, Some(date(21)))], &[]);
        let alert = events
            .iter()
            .find(|e| e.managed_key == "digest-alert:2026-11-21")
            .expect("alert event");
        assert_eq!(alert.summary, "Due today: 2 tasks");
        assert_eq!(
            alert.timing,
            EventTiming::Timed {
                start: at(2026, 11, 21, 7),
                end: at(2026, 11, 21, 7) + Duration::minutes(15),
            }
        );
        assert_eq!(alert.reminder_minutes, Some(0));
        // Summer: the same wall-clock time is 06:00 UTC.
        let summer = Due::Date(NaiveDate::from_ymd_opt(2026, 7, 4).expect("valid date"));
        let events = run(&[issue(1, Some(summer))], &[]);
        let alert = events
            .iter()
            .find(|e| e.managed_key == "digest-alert:2026-07-04")
            .expect("summer alert");
        assert_eq!(
            alert.timing,
            EventTiming::Timed {
                start: at(2026, 7, 4, 6),
                end: at(2026, 7, 4, 6) + Duration::minutes(15),
            }
        );
    }

    #[test]
    fn digest_reminder_is_none_until_probe() {
        assert_eq!(digest_reminder_minutes(), None);
    }

    fn cairn_event(all_day: bool, start: (u32, u32), end: Option<(u32, u32)>) -> Event {
        let t = |(d, h): (u32, u32)| {
            NaiveDate::from_ymd_opt(2026, 11, d)
                .and_then(|x| x.and_hms_opt(h, 0, 0))
                .expect("valid time")
        };
        Event {
            id: taskapp_core::EventId(4),
            title: "Dentist".into(),
            starts_at: t(start),
            ends_at: end.map(t),
            all_day,
            place: Some("Main St".into()),
            notes: "Bring card".into(),
            icon: None,
            color: None,
            archived: false,
            issue_ids: vec![],
        }
    }

    #[test]
    fn timed_event_keeps_start_end_in_utc_without_popup() {
        let e = &event_events(&[cairn_event(false, (21, 10), Some((21, 11)))], &[], NY)[0];
        assert_eq!(e.managed_key, "event:4");
        assert_eq!(
            e.timing,
            EventTiming::Timed {
                start: at(2026, 11, 21, 9),
                end: at(2026, 11, 21, 10)
            }
        );
        assert_eq!(e.reminder_minutes, None);
        assert_eq!(e.description, "Place: Main St\nBring card");
    }

    #[test]
    fn timed_event_without_end_lasts_an_hour() {
        let e = &event_events(&[cairn_event(false, (21, 10), None)], &[], NY)[0];
        assert_eq!(
            e.timing,
            EventTiming::Timed {
                start: at(2026, 11, 21, 9),
                end: at(2026, 11, 21, 10)
            }
        );
    }

    #[test]
    fn all_day_event_stays_all_day_single_or_span() {
        let d = |n| NaiveDate::from_ymd_opt(2026, 11, n).expect("valid date");
        let one = event_events(&[cairn_event(true, (21, 0), None)], &[], NY);
        assert_eq!(one[0].timing, EventTiming::AllDay(d(21)));
        let span = event_events(&[cairn_event(true, (21, 0), Some((23, 0)))], &[], NY);
        assert_eq!(span[0].timing, EventTiming::AllDaySpan(d(21), d(24)));
    }

    #[test]
    fn edited_event_updates_and_removed_event_deletes() {
        let before = event_events(&[cairn_event(false, (21, 10), None)], &[], NY);
        let existing = [ManagedEvent {
            google_event_id: "g1".into(),
            managed_key: "event:4".into(),
            last_synced_hash: hash(&before[0]),
        }];
        let moved = event_events(&[cairn_event(false, (22, 10), None)], &[], NY);
        assert!(matches!(plan(&moved, &existing)[0], EventOp::Update { .. }));
        assert_eq!(
            plan(&[], &existing),
            vec![EventOp::Delete {
                google_event_id: "g1".into()
            }]
        );
    }

    #[test]
    fn event_description_lists_linked_issues_with_links() {
        let mut e = cairn_event(false, (21, 10), None);
        e.issue_ids = vec![IssueId(1), IssueId(9)];
        // Issue 9 is not in the list (archived or outside the view): left out, not an error.
        let d = event_events(&[e], &[issue(1, None)], NY)[0]
            .description
            .clone();
        assert_eq!(
            d,
            "Place: Main St\nBring card\nABC-1 Task 1: taskapp://issue/ABC-1"
        );
    }

    #[test]
    fn linking_and_unlinking_an_issue_changes_the_event_hash() {
        let issues = [issue(1, None)];
        let plain = cairn_event(false, (21, 10), None);
        let mut linked = plain.clone();
        linked.issue_ids = vec![IssueId(1)];
        let before = event_events(&[plain], &issues, NY);
        let after = event_events(&[linked], &issues, NY);
        assert_ne!(hash(&before[0]), hash(&after[0]));
    }

    #[test]
    fn plan_creates_updates_deletes() {
        let a = event("issue:1", None);
        let b = event("issue:2", None);
        let c = event("issue:3", None);
        let existing = [
            ManagedEvent {
                google_event_id: "g2".into(),
                managed_key: "issue:2".into(),
                last_synced_hash: "old".into(),
            },
            ManagedEvent {
                google_event_id: "g3".into(),
                managed_key: "issue:3".into(),
                last_synced_hash: hash(&c),
            },
            ManagedEvent {
                google_event_id: "g9".into(),
                managed_key: "issue:9".into(),
                last_synced_hash: "x".into(),
            },
        ];
        let ops = plan(&[a.clone(), b.clone(), c], &existing);
        assert_eq!(
            ops,
            vec![
                EventOp::Create(a),
                EventOp::Update {
                    google_event_id: "g2".into(),
                    event: b,
                },
                EventOp::Delete {
                    google_event_id: "g9".into(),
                },
            ]
        );
    }

    #[test]
    fn unchanged_event_produces_no_op() {
        let c = event("issue:3", Some(0));
        let existing = [ManagedEvent {
            google_event_id: "g3".into(),
            managed_key: "issue:3".into(),
            last_synced_hash: hash(&c),
        }];
        assert_eq!(plan(&[c], &existing).len(), 0);
    }
}
