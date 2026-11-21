//! Pure: turns issues into the calendar events that should exist, and diffs against what does.

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use chrono_tz::Tz;
use sha2::{Digest, Sha256};
use taskapp_core::{Due, Issue, Project, Status, StatusCategory, WorkBlock};

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
    events.extend(digest_events(&open, projects));
    events.extend(block_events(blocks, issues, statuses, projects));
    events
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

fn digest_events(open: &[&Issue], projects: &[Project]) -> Vec<DesiredEvent> {
    let mut by_date: BTreeMap<NaiveDate, Vec<&Issue>> = BTreeMap::new();
    for issue in open {
        if let Some(Due::Date(date)) = &issue.due {
            by_date.entry(*date).or_default().push(issue);
        }
    }
    by_date
        .into_iter()
        .map(|(date, group)| {
            let summary = match group.len() {
                1 => "Due today: 1 task".to_string(),
                n => format!("Due today: {n} tasks"),
            };
            DesiredEvent {
                managed_key: format!("digest:{date}"),
                summary,
                description: group
                    .iter()
                    .map(|i| header(i, projects))
                    .collect::<Vec<_>>()
                    .join("\n\n"),
                timing: EventTiming::AllDay(date),
                reminder_minutes: digest_reminder_minutes(),
            }
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

    const NY: &str = "America/New_York";

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
        }
    }

    /// Open (status 1), priority 2, in project 7, unless the test changes it.
    fn issue(id: i64, due: Option<Due>) -> Issue {
        let now = at(2026, 11, 1, 0);
        Issue {
            id: IssueId(id),
            key: format!("ZF-{id}"),
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
        desired(issues, &statuses(), &[project()], blocks, NY, "ZF")
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
                .starts_with("ZF-1 Task 1\nPriority: High | Project: Home\n")
        );
        assert!(e.description.contains("taskapp://issue/ZF-1"));
    }

    #[test]
    fn date_only_grouped_into_digest() {
        let events = run(&[issue(1, Some(date(21))), issue(2, Some(date(21)))], &[]);
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
                .contains("ZF-1 Task 1\nPriority: High | Project: Home\ntaskapp://issue/ZF-1")
        );
        assert!(
            e.description
                .contains("ZF-2 Task 2\nPriority: High | Project: Home\ntaskapp://issue/ZF-2")
        );
    }

    #[test]
    fn single_task_digest_singular() {
        let events = run(&[issue(1, Some(date(21)))], &[]);
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
        let utc = at(2026, 11, 21, 1);
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
        assert!(timed.description.contains("2026-11-20 20:00 EST"));
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
        let events = run(&[issue(1, Some(date(21))), issue(2, Some(date(22)))], &[]);
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
    fn digest_reminder_is_none_until_probe() {
        assert_eq!(digest_reminder_minutes(), None);
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
