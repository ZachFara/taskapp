//! Pure scheduling logic; no database access.

use std::collections::{HashMap, VecDeque};

use chrono::{Days, NaiveDate};

use crate::{Bar, Due, Issue, IssueId, Relation, Reschedule};

const DAY_MINUTES: u32 = 480;

fn due_date(due: &Due) -> NaiveDate {
    match due {
        Due::Date(d) => *d,
        // shortcut: an unknown zone name falls back to the UTC date, when a bad tz can be stored, reject it at write time.
        Due::DateTime { utc, tz } => tz
            .parse::<chrono_tz::Tz>()
            .map_or_else(|_| utc.date_naive(), |z| utc.with_timezone(&z).date_naive()),
    }
}

fn days_back(d: NaiveDate, n: u64) -> NaiveDate {
    d.checked_sub_days(Days::new(n)).unwrap_or(NaiveDate::MIN)
}

fn days_fwd(d: NaiveDate, n: u64) -> NaiveDate {
    d.checked_add_days(Days::new(n)).unwrap_or(NaiveDate::MAX)
}

/// The days an issue is drawn over; a missing end date is derived from the estimate (1 day = 8 hours, at least 1).
#[must_use]
pub fn bar_for(issue: &Issue) -> Option<Bar> {
    let due = issue.due.as_ref().map(due_date);
    let span = |m: u32| u64::from(m.div_ceil(DAY_MINUTES).max(1)) - 1;
    let (start, end, from_estimate) = match (issue.start_date, due, issue.estimate_minutes) {
        (Some(s), Some(e), _) => (s, e, false),
        (None, Some(e), Some(m)) => (days_back(e, span(m)), e, true),
        (Some(s), None, Some(m)) => (s, days_fwd(s, span(m)), true),
        _ => return None,
    };
    Some(Bar {
        issue_id: issue.id,
        start,
        end,
        from_estimate,
    })
}

fn blocked_by(blocks: &[Relation]) -> HashMap<IssueId, Vec<IssueId>> {
    let mut m: HashMap<IssueId, Vec<IssueId>> = HashMap::new();
    for r in blocks.iter().filter(|r| r.kind.0 == "blocks") {
        m.entry(r.from).or_default().push(r.to);
    }
    m
}

/// Finish-to-start: anything blocked by a moved issue that starts on or before the blocker's due date moves to start the
/// next day, keeping its length, recursively. Nothing moves earlier. The moved issue itself is not in the result.
#[must_use]
pub fn shift_dependents(
    issues: &[Issue],
    blocks: &[Relation],
    moved: &Reschedule,
) -> Vec<Reschedule> {
    let next = blocked_by(blocks);
    let bars: HashMap<IssueId, Bar> = issues
        .iter()
        .filter_map(bar_for)
        .map(|b| (b.issue_id, b))
        .collect();
    let mut dates: HashMap<IssueId, (NaiveDate, NaiveDate)> = HashMap::new();
    dates.insert(moved.issue_id, (moved.start_date, moved.due_date));
    let mut queue = VecDeque::from([moved.issue_id]);
    // A dependency chain is acyclic, so no issue needs more than one pass per issue; the cap only stops a stored cycle.
    let mut budget = issues.len().saturating_mul(issues.len()).saturating_add(1);
    while let Some(id) = queue.pop_front() {
        budget = budget.saturating_sub(1);
        if budget == 0 {
            break;
        }
        let blocker_due = dates[&id].1;
        for dep in next.get(&id).into_iter().flatten() {
            let Some(bar) = bars.get(dep) else { continue };
            let (start, end) = dates.get(dep).copied().unwrap_or((bar.start, bar.end));
            if start > blocker_due {
                continue;
            }
            let delta = u64::try_from((blocker_due - start).num_days() + 1).unwrap_or(1);
            dates.insert(*dep, (days_fwd(start, delta), days_fwd(end, delta)));
            queue.push_back(*dep);
        }
    }
    let mut out: Vec<Reschedule> = dates
        .into_iter()
        .filter(|(id, _)| *id != moved.issue_id)
        .map(|(issue_id, (start_date, due_date))| Reschedule {
            issue_id,
            start_date,
            due_date,
        })
        .collect();
    out.sort_by_key(|r| r.issue_id.0);
    out
}

type Chain = (i64, Vec<IssueId>);

fn bar_days(b: &Bar) -> i64 {
    (b.end - b.start).num_days() + 1
}

/// Longest chain from `id` (total days, then lowest ids first) through blocking edges.
fn longest_from(
    id: IssueId,
    bars: &HashMap<IssueId, Bar>,
    next: &HashMap<IssueId, Vec<IssueId>>,
    memo: &mut HashMap<IssueId, Chain>,
    on_path: &mut Vec<IssueId>,
) -> Chain {
    if let Some(hit) = memo.get(&id) {
        return hit.clone();
    }
    on_path.push(id);
    let mut best: Chain = (0, Vec::new());
    for n in next.get(&id).into_iter().flatten() {
        // Skipping nodes on the current path keeps a stored cycle from recursing forever.
        if !bars.contains_key(n) || on_path.contains(n) {
            continue;
        }
        let cand = longest_from(*n, bars, next, memo, on_path);
        if cand.0 > best.0
            || (cand.0 == best.0 && cand.1.first().map(|f| f.0) < best.1.first().map(|f| f.0))
        {
            best = cand;
        }
    }
    on_path.pop();
    let mut chain = vec![id];
    chain.extend(best.1);
    let res = (best.0 + bar_days(&bars[&id]), chain);
    memo.insert(id, res.clone());
    res
}

/// The longest chain by total days through blocking edges among issues with bars; ties go to the lowest issue id.
#[must_use]
pub fn critical_path(issues: &[Issue], blocks: &[Relation]) -> Vec<IssueId> {
    let bars: HashMap<IssueId, Bar> = issues
        .iter()
        .filter_map(bar_for)
        .map(|b| (b.issue_id, b))
        .collect();
    let next = blocked_by(blocks);
    let mut memo = HashMap::new();
    let mut ids: Vec<IssueId> = bars.keys().copied().collect();
    ids.sort_by_key(|i| i.0);
    let mut best: Chain = (0, Vec::new());
    for id in ids {
        let cand = longest_from(id, &bars, &next, &mut memo, &mut Vec::new());
        if cand.0 > best.0 {
            best = cand;
        }
    }
    best.1
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::Map;

    use super::*;
    use crate::{RelationKind, StatusId};

    fn d(s: &str) -> NaiveDate {
        s.parse().expect("date")
    }

    fn issue(id: i64, start: Option<&str>, due: Option<&str>, est: Option<u32>) -> Issue {
        Issue {
            id: IssueId(id),
            key: format!("ZF-{id}"),
            title: String::new(),
            description: String::new(),
            status_id: StatusId(1),
            priority: 0,
            project_id: None,
            milestone_id: None,
            parent_id: None,
            cycle_id: None,
            label_ids: vec![],
            start_date: start.map(d),
            due: due.map(|x| Due::Date(d(x))),
            estimate_minutes: est,
            points: None,
            hidden_until: None,
            depth: None,
            properties: Map::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
            archived_at: None,
        }
    }

    fn blk(from: i64, to: i64) -> Relation {
        Relation {
            from: IssueId(from),
            to: IssueId(to),
            kind: RelationKind::new("blocks"),
        }
    }

    fn moved(id: i64, start: &str, due: &str) -> Reschedule {
        Reschedule {
            issue_id: IssueId(id),
            start_date: d(start),
            due_date: d(due),
        }
    }

    #[test]
    fn bar_from_estimate() {
        let b = bar_for(&issue(1, None, Some("2026-12-10"), Some(960))).expect("bar");
        assert_eq!(
            (b.start, b.end, b.from_estimate),
            (d("2026-12-09"), d("2026-12-10"), true)
        );
        let b = bar_for(&issue(1, Some("2026-12-01"), None, Some(1))).expect("bar");
        assert_eq!((b.start, b.end), (d("2026-12-01"), d("2026-12-01")));
        let b = bar_for(&issue(1, Some("2026-12-01"), None, Some(481))).expect("bar");
        assert_eq!(b.end, d("2026-12-02"));
    }

    #[test]
    fn bar_with_both_dates_ignores_estimate() {
        let b = bar_for(&issue(1, Some("2026-12-01"), Some("2026-12-05"), Some(60))).expect("bar");
        assert_eq!(
            (b.start, b.end, b.from_estimate),
            (d("2026-12-01"), d("2026-12-05"), false)
        );
    }

    #[test]
    fn bar_uses_local_date_of_timed_due() {
        let mut i = issue(1, None, None, Some(480));
        // 23:30 UTC is already the next day in Auckland.
        i.due = Some(Due::DateTime {
            utc: "2026-12-10T23:30:00Z".parse().expect("utc"),
            tz: "Pacific/Auckland".into(),
        });
        assert_eq!(bar_for(&i).expect("bar").end, d("2026-12-11"));
    }

    #[test]
    fn no_dates_no_bar() {
        assert!(bar_for(&issue(1, None, None, None)).is_none());
        assert!(bar_for(&issue(1, None, None, Some(480))).is_none());
        assert!(bar_for(&issue(1, Some("2026-12-01"), None, None)).is_none());
        assert!(bar_for(&issue(1, None, Some("2026-12-01"), None)).is_none());
    }

    fn chain() -> Vec<Issue> {
        vec![
            issue(1, Some("2026-12-01"), Some("2026-12-02"), None),
            issue(2, Some("2026-12-03"), Some("2026-12-04"), None),
            issue(3, Some("2026-12-05"), Some("2026-12-06"), None),
        ]
    }

    #[test]
    fn shift_pushes_chain() {
        let out = shift_dependents(
            &chain(),
            &[blk(1, 2), blk(2, 3)],
            &moved(1, "2026-12-04", "2026-12-05"),
        );
        assert_eq!(
            out,
            vec![
                moved(2, "2026-12-06", "2026-12-07"),
                moved(3, "2026-12-08", "2026-12-09")
            ]
        );
    }

    #[test]
    fn shift_never_pulls_earlier() {
        let out = shift_dependents(
            &chain(),
            &[blk(1, 2), blk(2, 3)],
            &moved(1, "2026-11-20", "2026-11-21"),
        );
        assert_eq!(out, vec![]);
    }

    #[test]
    fn shift_ignores_unaffected_gap() {
        let issues = vec![
            issue(1, Some("2026-12-01"), Some("2026-12-02"), None),
            issue(2, Some("2026-12-12"), Some("2026-12-13"), None),
        ];
        let out = shift_dependents(&issues, &[blk(1, 2)], &moved(1, "2026-12-03", "2026-12-04"));
        assert_eq!(out, vec![]);
    }

    #[test]
    fn shift_diamond_moves_join_once_to_later_date() {
        let issues = vec![
            issue(1, Some("2026-12-01"), Some("2026-12-02"), None),
            issue(2, Some("2026-12-03"), Some("2026-12-04"), None),
            issue(3, Some("2026-12-03"), Some("2026-12-10"), None),
            issue(4, Some("2026-12-05"), Some("2026-12-06"), None),
        ];
        let b = [blk(1, 2), blk(1, 3), blk(2, 4), blk(3, 4)];
        let out = shift_dependents(&issues, &b, &moved(1, "2026-12-01", "2026-12-04"));
        // C ends 12-12 after the shift, later than B, so D starts the day after that.
        assert_eq!(out.iter().filter(|r| r.issue_id == IssueId(4)).count(), 1);
        assert_eq!(
            out,
            vec![
                moved(2, "2026-12-05", "2026-12-06"),
                moved(3, "2026-12-05", "2026-12-12"),
                moved(4, "2026-12-13", "2026-12-14"),
            ]
        );
    }

    #[test]
    fn shift_survives_a_stored_cycle() {
        let out = shift_dependents(
            &chain(),
            &[blk(1, 2), blk(2, 1)],
            &moved(1, "2026-12-04", "2026-12-05"),
        );
        assert_eq!(out.first().map(|r| r.issue_id), Some(IssueId(2)));
    }

    #[test]
    fn critical_path_longest_chain() {
        let issues = vec![
            issue(1, Some("2026-12-01"), Some("2026-12-02"), None),
            issue(2, Some("2026-12-03"), Some("2026-12-04"), None),
            issue(3, Some("2026-12-03"), Some("2026-12-09"), None),
            issue(4, Some("2026-12-10"), Some("2026-12-10"), None),
        ];
        let b = [blk(1, 2), blk(1, 3), blk(3, 4)];
        assert_eq!(
            critical_path(&issues, &b),
            vec![IssueId(1), IssueId(3), IssueId(4)]
        );
    }

    #[test]
    fn critical_path_without_edges_is_longest_bar() {
        let issues = vec![
            issue(5, Some("2026-12-01"), Some("2026-12-02"), None),
            issue(3, Some("2026-12-01"), Some("2026-12-05"), None),
            issue(2, Some("2026-12-01"), Some("2026-12-05"), None),
            issue(1, None, None, None),
        ];
        assert_eq!(critical_path(&issues, &[]), vec![IssueId(2)]);
        assert_eq!(critical_path(&[], &[]), vec![]);
    }

    #[test]
    fn critical_path_skips_edges_to_issues_without_bars() {
        let issues = vec![
            issue(1, Some("2026-12-01"), Some("2026-12-02"), None),
            issue(2, None, None, None),
        ];
        assert_eq!(critical_path(&issues, &[blk(1, 2)]), vec![IssueId(1)]);
    }
}
