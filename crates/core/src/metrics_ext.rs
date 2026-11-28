//! Cycle commitment, blocked work and flow efficiency: the metrics that read cycle, relation and status-kind history.

use std::collections::HashMap;

use chrono::NaiveDate;
use chrono_tz::Tz;
use sqlx::Row;

use super::{Facts, Log, count, gap, total};
use crate::{
    Blocked, BlockedItem, BlockedTime, Cycle, CycleCommit, Db, FlowEfficiency, FlowKind, Result,
    Status, StatusCategory, ViewDef, WaitingStatus, insights::local,
};

fn pct(part: f64, of: f64) -> Option<f64> {
    (of > 0.0).then(|| part * 100.0 / of)
}

fn open(f: &Facts, day: NaiveDate) -> bool {
    matches!(
        f.at(day),
        Some((
            StatusCategory::Backlog | StatusCategory::Unstarted | StatusCategory::Started,
            _
        ))
    )
}

/// What the cycle held at the end of its start day against what it holds (and has finished) at its end or today.
pub(super) fn commit(all: &[Facts], c: &Cycle, today: NaiveDate) -> CycleCommit {
    let end = c.end_date.min(today).max(c.start_date);
    let member = |f: &Facts, day| f.made <= day && f.cycle_at(day) == Some(c.id.0);
    let sum = |v: &[&Facts]| total(v.iter().map(|f| f.points()));
    let pick = |keep: &dyn Fn(&Facts) -> bool| all.iter().filter(|f| keep(f)).collect::<Vec<_>>();
    let committed = pick(&|f| member(f, c.start_date));
    let done = pick(&|f| {
        member(f, c.start_date) && matches!(f.at(end), Some((StatusCategory::Completed, _)))
    });
    let added = pick(&|f| member(f, end) && !member(f, c.start_date));
    let removed = pick(&|f| member(f, c.start_date) && !member(f, end));
    let (cp, ch) = (sum(&committed), f64::from(count(committed.len())));
    let moved = f64::from(count(added.len() + removed.len()));
    CycleCommit {
        committed: count(committed.len()),
        committed_points: cp,
        done: count(done.len()),
        done_points: sum(&done),
        say_do_pct: pct(f64::from(count(done.len())), ch),
        say_do_points_pct: pct(sum(&done), cp),
        added: count(added.len()),
        added_points: sum(&added),
        removed: count(removed.len()),
        removed_points: sum(&removed),
        scope_change_pct: pct(moved, ch),
        scope_change_points_pct: pct(sum(&added) + sum(&removed), cp),
    }
}

/// Every issue, unfiltered, when `def` filters (a blocker may fall outside the filter); `None` when `all` already is.
pub(super) async fn everyone(
    db: &Db,
    def: &ViewDef,
    log: &Log,
    cats: &HashMap<i64, StatusCategory>,
    tz: Tz,
) -> Result<Option<Vec<Facts>>> {
    if def.filter.is_none() {
        return Ok(None);
    }
    super::facts(db, &ViewDef::default(), log, cats, tz)
        .await
        .map(Some)
}

/// The `blocks` links into each issue, by the blocked issue's id.
pub(super) type Links = HashMap<i64, Vec<Link>>;

/// One `blocks` link: the blocker, the first day it stood and the day it was removed (still standing if `None`).
pub(super) struct Link {
    blocker: i64,
    from: NaiveDate,
    until: Option<NaiveDate>,
}

/// The `blocks` links into each issue, from the relation history plus the relations table for links that have none.
pub(super) async fn links(db: &Db, log: &Log, every: &[Facts], tz: Tz) -> Result<Links> {
    let mut out = Links::new();
    for (at, rel, added) in &log.blocks {
        let (blocker, target, day) = (rel.from.0, rel.to.0, local(*at, tz));
        let mine = out.entry(target).or_default();
        if *added {
            mine.push(Link {
                blocker,
                from: day,
                until: None,
            });
        } else if let Some(l) = mine
            .iter_mut()
            .rev()
            .find(|l| l.blocker == blocker && l.until.is_none())
        {
            l.until = Some(day);
        }
    }
    let made: HashMap<i64, NaiveDate> = every.iter().map(|f| (f.issue.id.0, f.made)).collect();
    let rows = sqlx::query("SELECT from_id, to_id FROM relations WHERE kind = 'blocks'")
        .fetch_all(&db.pool)
        .await?;
    let now: Vec<(i64, i64)> = rows
        .iter()
        .map(|r| Ok((r.try_get(0)?, r.try_get(1)?)))
        .collect::<Result<_>>()?;
    // The table is the truth: a link the history leaves open but the table lacks (an undo, say) never lasted.
    for (target, mine) in &mut out {
        for l in mine.iter_mut().filter(|l| l.until.is_none()) {
            if !now.contains(&(l.blocker, *target)) {
                l.until = Some(l.from);
            }
        }
    }
    for (blocker, target) in now {
        let mine = out.entry(target).or_default();
        if !mine
            .iter()
            .any(|l| l.blocker == blocker && l.until.is_none())
        {
            // No history (imported): the link is as old as the younger of the two issues.
            let day = made.get(&blocker).max(made.get(&target)).copied();
            mine.push(Link {
                blocker,
                from: day.unwrap_or_default(),
                until: None,
            });
        }
    }
    Ok(out)
}

pub(super) fn blocked(
    mine: &[&Facts],
    done: &[&Facts],
    every: &[Facts],
    links: &Links,
    today: NaiveDate,
) -> Blocked {
    let by_id: HashMap<i64, &Facts> = every.iter().map(|f| (f.issue.id.0, f)).collect();
    let mut now = vec![];
    for f in mine.iter().filter(|f| open(f, today)) {
        let live: Vec<_> = links
            .get(&f.issue.id.0)
            .into_iter()
            .flatten()
            .filter(|l| l.until.is_none())
            .filter_map(|l| Some((by_id.get(&l.blocker).filter(|b| open(b, today))?, l.from)))
            .collect();
        let Some(since) = live.iter().map(|l| l.1).max() else {
            continue;
        };
        let mut keys: Vec<_> = live.iter().map(|l| l.0.issue.key.clone()).collect();
        keys.sort();
        now.push(BlockedItem {
            issue_id: f.issue.id,
            key: f.issue.key.clone(),
            title: f.issue.title.clone(),
            blocked_by: keys,
            days: gap(since, today),
        });
    }
    now.sort_by(|a, b| b.days.total_cmp(&a.days).then_with(|| a.key.cmp(&b.key)));
    let mut time = vec![];
    for f in done {
        let days = links
            .get(&f.issue.id.0)
            .map_or(0, |l| blocked_days(f, l, &by_id));
        if days > 0 {
            time.push(BlockedTime {
                issue_id: f.issue.id,
                key: f.issue.key.clone(),
                title: f.issue.title.clone(),
                days: f64::from(count(days)),
            });
        }
    }
    time.sort_by(|a, b| b.days.total_cmp(&a.days).then_with(|| a.key.cmp(&b.key)));
    let avg_days = (!done.is_empty())
        .then(|| total(time.iter().map(|t| t.days)) / f64::from(count(done.len())));
    Blocked {
        now,
        time,
        avg_days,
    }
}

/// Days from creation to finish at whose end an unfinished blocker was linked.
fn blocked_days(f: &Facts, links: &[Link], by_id: &HashMap<i64, &Facts>) -> usize {
    let end = f.done.unwrap_or(f.made);
    f.made
        .iter_days()
        .take_while(|d| *d < end)
        .filter(|d| {
            links.iter().any(|l| {
                l.from <= *d
                    && l.until.is_none_or(|u| *d < u)
                    && by_id.get(&l.blocker).is_some_and(|b| open(b, *d))
            })
        })
        .count()
}

/// Days spent in each status between starting and finishing.
fn spent(f: &Facts, start: NaiveDate, end: NaiveDate) -> HashMap<i64, f64> {
    let mut out = HashMap::new();
    let line: Vec<_> = std::iter::once((f.made, f.status0))
        .chain(f.path.iter().copied())
        .collect();
    for (i, (day, id)) in line.iter().enumerate() {
        let next = line.get(i + 1).map_or(end, |n| n.0);
        let (lo, hi) = ((*day).max(start), next.min(end));
        if hi > lo {
            *out.entry(*id).or_default() += gap(lo, hi);
        }
    }
    out
}

/// Per finished issue with a cycle time: the share of the days between starting and finishing spent in Active statuses.
pub(super) fn flow(done: &[&Facts], statuses: &[Status]) -> FlowEfficiency {
    let kinds: HashMap<i64, &Status> = statuses.iter().map(|s| (s.id.0, s)).collect();
    let mut effs: Vec<f64> = vec![];
    let mut waiting: HashMap<i64, (f64, u32)> = HashMap::new();
    for f in done {
        let (Some(start), Some(end)) = (f.started, f.done) else {
            continue;
        };
        if end <= start {
            continue;
        }
        let spent = spent(f, start, end);
        let kind = |id: &i64| kinds.get(id).map(|s| s.flow_kind);
        let active: f64 = spent
            .iter()
            .filter(|(id, _)| kind(id) == Some(FlowKind::Active))
            .map(|(_, d)| d)
            .sum();
        effs.push(active * 100.0 / gap(start, end));
        for (id, d) in spent
            .iter()
            .filter(|(id, _)| kind(id) == Some(FlowKind::Waiting))
        {
            let e = waiting.entry(*id).or_default();
            e.0 += d;
            e.1 += 1;
        }
    }
    let mut distribution = vec![0; 5];
    for e in &effs {
        let bin = [20.0, 40.0, 60.0, 80.0]
            .iter()
            .filter(|t| *e >= **t)
            .count();
        distribution[bin] += 1;
    }
    let mut waiting: Vec<_> = waiting
        .into_iter()
        .filter_map(|(id, (days, issues))| {
            Some(WaitingStatus {
                status_id: kinds.get(&id)?.id,
                name: kinds.get(&id)?.name.clone(),
                days,
                issues,
            })
        })
        .collect();
    waiting.sort_by(|a, b| b.days.total_cmp(&a.days).then_with(|| a.name.cmp(&b.name)));
    FlowEfficiency {
        avg_pct: (!effs.is_empty())
            .then(|| effs.iter().sum::<f64>() / f64::from(count(effs.len()))),
        n: count(effs.len()),
        distribution,
        waiting,
    }
}
