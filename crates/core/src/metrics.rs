//! Delivery metrics for a date range or a cycle: throughput, lead and cycle time, work in progress, and
//! breakdowns by label and by who made the change. Nothing is stored; every call replays the change history
//! (see [`crate::insights`] for the same idea behind the burn-up charts).

use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Datelike, Duration, Months, NaiveDate, Utc};
use chrono_tz::Tz;
use sqlx::Row;

use crate::{
    Bucket, ChangeSource, Cycle, CycleTime, CycleTimePoint, Db, Error, Issue, LabelBreakdown,
    LeadTime, Measure, Metrics, MetricsBucket, MetricsQuery, MetricsScope, Result, SourceBreakdown,
    StatusCategory, VelocityCycle, ViewDef, cycles,
    insights::{local, zone},
    labels, statuses, views,
};

const MAX_DAYS: i64 = 1100;
const THIN: usize = 5;
const VELOCITY_CYCLES: usize = 6;

/// One issue's story: when it was made and by whom, where its status went, and when it finished.
struct Facts {
    issue: Issue,
    made: NaiveDate,
    made_by: String,
    cat0: StatusCategory,
    /// (local day, category entered), oldest first.
    moves: Vec<(NaiveDate, StatusCategory)>,
    started: Option<NaiveDate>,
    done: Option<NaiveDate>,
    done_by: String,
}

impl Facts {
    /// The category at the end of `day` and the day it was entered; `None` before the issue existed.
    fn at(&self, day: NaiveDate) -> Option<(StatusCategory, NaiveDate)> {
        if self.made > day {
            return None;
        }
        Some(
            self.moves
                .iter()
                .rev()
                .find(|m| m.0 <= day)
                .map_or((self.cat0, self.made), |m| (m.1, m.0)),
        )
    }

    fn points(&self) -> f64 {
        self.issue.points.unwrap_or(0.0)
    }

    fn finished_in(&self, a: NaiveDate, b: NaiveDate) -> bool {
        self.done.is_some_and(|d| a <= d && d <= b)
    }
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn gap(from: NaiveDate, to: NaiveDate) -> f64 {
    f64::from(i32::try_from((to - from).num_days().max(0)).unwrap_or(i32::MAX))
}

fn source_name(raw: &str) -> String {
    serde_json::from_str::<ChangeSource>(raw)
        .map_or_else(|_| "Unknown".into(), |s| format!("{s:?}"))
}

/// Nearest-rank percentile (`pct` out of 100) of a list.
fn percentile(days: &[f64], pct: usize) -> Option<f64> {
    let mut v = days.to_vec();
    v.sort_by(f64::total_cmp);
    v.get((v.len() * pct).div_ceil(100).max(1) - 1).copied()
}

fn bucket_start(day: NaiveDate, b: Bucket) -> NaiveDate {
    match b {
        Bucket::Day => day,
        Bucket::Week => day - Duration::days(i64::from(day.weekday().num_days_from_monday())),
        Bucket::Month => day.with_day(1).unwrap_or(day),
    }
}

fn next_start(start: NaiveDate, b: Bucket) -> NaiveDate {
    match b {
        Bucket::Day => start + Duration::days(1),
        Bucket::Week => start + Duration::days(7),
        Bucket::Month => start
            .checked_add_months(Months::new(1))
            .unwrap_or(start + Duration::days(31)),
    }
}

/// (when, source, old, new) of one status change.
type Move = (DateTime<Utc>, String, Option<String>, Option<String>);
/// The days a query covers and the cycle it is about, if any.
type Span = (NaiveDate, NaiveDate, Option<Cycle>);

/// Every status change and the source of every creation, per issue.
struct Log {
    moves: HashMap<i64, Vec<Move>>,
    made_by: HashMap<i64, String>,
}

async fn log(db: &Db) -> Result<Log> {
    let rows = sqlx::query(
        "SELECT entity_id, field, at, source, old_value, new_value FROM changes
         WHERE entity = 'issue' AND field IN ('key', 'status_id') ORDER BY at, id",
    )
    .fetch_all(&db.pool)
    .await?;
    let mut out = Log {
        moves: HashMap::new(),
        made_by: HashMap::new(),
    };
    for r in &rows {
        let Ok(id) = r.try_get::<String, _>(0)?.parse::<i64>() else {
            continue;
        };
        let source = source_name(&r.try_get::<String, _>(3)?);
        if r.try_get::<String, _>(1)? == "key" {
            out.made_by.entry(id).or_insert(source);
        } else {
            out.moves.entry(id).or_default().push((
                r.try_get(2)?,
                source,
                r.try_get(4)?,
                r.try_get(5)?,
            ));
        }
    }
    Ok(out)
}

fn story(issue: Issue, log: &Log, cats: &HashMap<i64, StatusCategory>, tz: Tz) -> Facts {
    let cat_of = |s: &Option<String>| {
        s.as_deref()
            .and_then(|s| s.parse::<i64>().ok())
            .and_then(|id| cats.get(&id).copied())
    };
    let events = log.moves.get(&issue.id.0).map_or(&[][..], Vec::as_slice);
    let now = cats.get(&issue.status_id.0).copied();
    // Creation records no status, so the state before the first move is that move's old value.
    let before = events
        .first()
        .and_then(|e| cat_of(&e.2))
        .or(now)
        .unwrap_or(StatusCategory::Backlog);
    let moves: Vec<_> = events
        .iter()
        .filter_map(|e| Some((local(e.0, tz), cat_of(&e.3)?, &e.1)))
        .collect();
    // Linear imports have no history: their completed_at stands in for the move.
    let finishing = moves
        .iter()
        .rev()
        .find(|m| m.1 == StatusCategory::Completed);
    let (done, done_by) = match (now, finishing) {
        (Some(StatusCategory::Completed), Some(m)) => (Some(m.0), m.2.clone()),
        (Some(StatusCategory::Completed), None) => {
            (issue.completed_at.map(|t| local(t, tz)), "Migration".into())
        }
        _ => (None, String::new()),
    };
    let started = moves
        .iter()
        .find(|m| m.1 == StatusCategory::Started)
        .map(|m| m.0)
        .filter(|s| done.is_some_and(|d| *s <= d));
    Facts {
        made: local(issue.created_at, tz),
        made_by: log
            .made_by
            .get(&issue.id.0)
            .cloned()
            .unwrap_or_else(|| "Migration".into()),
        cat0: before,
        moves: moves.into_iter().map(|m| (m.0, m.1)).collect(),
        started,
        done,
        done_by,
        issue,
    }
}

fn buckets(mine: &[&Facts], from: NaiveDate, to: NaiveDate, kind: Bucket) -> Vec<MetricsBucket> {
    let mut out = vec![];
    let mut start = bucket_start(from, kind);
    while start <= to {
        let next = next_start(start, kind);
        let (a, b) = (start.max(from), (next - Duration::days(1)).min(to));
        let (mut open, mut finished, mut wip) = (0, 0, 0);
        for state in mine.iter().filter_map(|f| f.at(b)) {
            match state {
                // Work finished before the range is not part of its scope.
                (StatusCategory::Completed, since) if since >= from => finished += 1,
                (StatusCategory::Backlog | StatusCategory::Unstarted, _) => open += 1,
                (StatusCategory::Started, _) => {
                    open += 1;
                    wip += 1;
                }
                _ => {}
            }
        }
        let done: Vec<_> = mine.iter().filter(|f| f.finished_in(a, b)).collect();
        out.push(MetricsBucket {
            start,
            created: count(mine.iter().filter(|f| a <= f.made && f.made <= b).count()),
            completed: count(done.len()),
            completed_points: total(done.iter().map(|f| f.points())),
            scope: open + finished,
            done_total: finished,
            wip,
        });
        start = next;
    }
    out
}

/// Sum of points. A float `sum` of nothing is -0.0, which prints as "-0"; `+ 0.0` makes it 0.
fn total(points: impl Iterator<Item = f64>) -> f64 {
    points.sum::<f64>() + 0.0
}

async fn by_label(db: &Db, done: &[&Facts], measure: Measure) -> Result<Vec<LabelBreakdown>> {
    let names: HashMap<i64, String> = labels::list(db)
        .await?
        .into_iter()
        .map(|l| (l.id.0, l.name))
        .collect();
    let mut tally: BTreeMap<Option<i64>, (u32, f64)> = BTreeMap::new();
    for f in done {
        let ids: Vec<_> = f.issue.label_ids.iter().map(|l| Some(l.0)).collect();
        for id in if ids.is_empty() { vec![None] } else { ids } {
            let e = tally.entry(id).or_default();
            e.0 += 1;
            e.1 += f.points();
        }
    }
    let mut out: Vec<_> = tally
        .into_iter()
        .map(|(label_id, (completed, points))| LabelBreakdown {
            name: label_id
                .and_then(|i| names.get(&i).cloned())
                .unwrap_or_else(|| "No label".into()),
            label_id,
            completed,
            points,
        })
        .collect();
    out.sort_by(|a, b| {
        let by = if measure == Measure::Points {
            b.points.total_cmp(&a.points)
        } else {
            b.completed.cmp(&a.completed)
        };
        by.then_with(|| a.name.cmp(&b.name))
    });
    Ok(out)
}

fn by_source(mine: &[&Facts], from: NaiveDate, to: NaiveDate) -> Vec<SourceBreakdown> {
    let mut tally: BTreeMap<&str, (u32, u32)> = BTreeMap::new();
    for f in mine {
        if from <= f.made && f.made <= to {
            tally.entry(&f.made_by).or_default().0 += 1;
        }
        if f.finished_in(from, to) {
            tally.entry(&f.done_by).or_default().1 += 1;
        }
    }
    let mut out: Vec<_> = tally
        .into_iter()
        .map(|(source, (created, completed))| SourceBreakdown {
            source: source.into(),
            created,
            completed,
        })
        .collect();
    out.sort_by(|a, b| {
        (b.created + b.completed)
            .cmp(&(a.created + a.completed))
            .then_with(|| a.source.cmp(&b.source))
    });
    out
}

/// What finished inside each of the last finished cycles, whichever issues were in them.
fn velocity(all: &[Facts], all_cycles: &[Cycle], today: NaiveDate) -> Vec<VelocityCycle> {
    let mut past: Vec<_> = all_cycles.iter().filter(|c| c.end_date < today).collect();
    past.sort_by_key(|c| c.number);
    past.split_off(past.len().saturating_sub(VELOCITY_CYCLES))
        .into_iter()
        .map(|c| {
            let done: Vec<_> = all
                .iter()
                .filter(|f| f.finished_in(c.start_date, c.end_date))
                .collect();
            VelocityCycle {
                cycle_id: c.id,
                name: c
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("Cycle {}", c.number)),
                points: total(done.iter().map(|f| f.points())),
                count: count(done.len()),
            }
        })
        .collect()
}

fn times(done: &[&Facts]) -> (LeadTime, CycleTime) {
    let lead: Vec<f64> = done
        .iter()
        .map(|f| gap(f.made, f.done.unwrap_or(f.made)))
        .collect();
    let mut timed: Vec<_> = done
        .iter()
        .filter_map(|f| Some((f, f.started?, f.done?)))
        .collect();
    timed.sort_by_key(|t| t.2);
    let days: Vec<f64> = timed.iter().map(|t| gap(t.1, t.2)).collect();
    (
        LeadTime {
            p50_days: percentile(&lead, 50),
            p85_days: percentile(&lead, 85),
            n: count(lead.len()),
        },
        CycleTime {
            p50_days: percentile(&days, 50),
            p85_days: percentile(&days, 85),
            n: count(days.len()),
            points: timed
                .iter()
                .map(|(f, s, d)| CycleTimePoint {
                    issue_id: f.issue.id,
                    key: f.issue.key.clone(),
                    title: f.issue.title.clone(),
                    completed: *d,
                    days: gap(*s, *d),
                })
                .collect(),
        },
    )
}

fn span(q: &MetricsQuery, all_cycles: &[Cycle]) -> Result<Span> {
    let (from, to, cycle) = match q.scope {
        MetricsScope::Range { from, to } => (from, to, None),
        MetricsScope::Cycle(id) => {
            let c = all_cycles
                .iter()
                .find(|c| c.id == id)
                .ok_or_else(|| Error::NotFound(format!("cycle {}", id.0)))?;
            (c.start_date, c.end_date, Some(c.clone()))
        }
    };
    if to < from {
        return Err(Error::Invalid(format!(
            "range ends ({to}) before it starts ({from})"
        )));
    }
    if (to - from).num_days() >= MAX_DAYS {
        return Err(Error::Invalid(format!(
            "range {from} to {to} must be 1 to {MAX_DAYS} days"
        )));
    }
    Ok((from, to, cycle))
}

/// [`metrics_for`] with the clock given, so a test can fix "today".
///
/// # Errors
/// `Invalid` for a backwards range, one over 1100 days, or the time measure; `NotFound` for an unknown cycle.
pub async fn metrics_on(db: &Db, q: MetricsQuery, today: NaiveDate) -> Result<Metrics> {
    if q.measure == Measure::Time {
        return Err(Error::Invalid(
            "metrics count issues or points, not time".into(),
        ));
    }
    let tz = zone(db).await?;
    let all_cycles = cycles::list(db).await?;
    let (from, to, cycle) = span(&q, &all_cycles)?;
    let cats: HashMap<i64, StatusCategory> = statuses::list(db)
        .await?
        .into_iter()
        .map(|s| (s.id.0, s.category))
        .collect();
    let log = log(db).await?;
    let def = ViewDef {
        filter: q.filter,
        ..ViewDef::default()
    };
    let all: Vec<Facts> = views::query(db, &def)
        .await?
        .into_iter()
        .map(|i| story(i, &log, &cats, tz))
        .collect();
    // Velocity looks across cycles, so it keeps every filtered issue; the rest is the cycle's members.
    let mine: Vec<&Facts> = all
        .iter()
        .filter(|f| {
            cycle
                .as_ref()
                .is_none_or(|c| f.issue.cycle_id == Some(c.id))
        })
        .collect();
    let done: Vec<&Facts> = mine
        .iter()
        .copied()
        .filter(|f| f.finished_in(from, to))
        .collect();
    let (lead_time, cycle_time) = times(&done);
    Ok(Metrics {
        from,
        to,
        bucket: q.bucket,
        buckets: buckets(&mine, from, to, q.bucket),
        lead_time,
        cycle_time,
        wip_now: count(
            mine.iter()
                .filter(|f| matches!(f.at(today), Some((StatusCategory::Started, _))))
                .count(),
        ),
        by_label: by_label(db, &done, q.measure).await?,
        by_source: by_source(&mine, from, to),
        velocity: velocity(&all, &all_cycles, today),
        thin: done.len() < THIN,
    })
}

/// Today in the workspace time zone.
///
/// # Errors
/// `Invalid` when the stored time zone is unknown.
pub async fn today(db: &Db) -> Result<NaiveDate> {
    Ok(local(Utc::now(), zone(db).await?))
}

/// The metrics for `q`, as of now.
///
/// # Errors
/// As [`metrics_on`].
pub async fn metrics_for(db: &Db, q: MetricsQuery) -> Result<Metrics> {
    metrics_on(db, q, today(db).await?).await
}

#[cfg(test)]
mod total_tests {
    #[test]
    fn nothing_sums_to_positive_zero() {
        assert!(super::total(std::iter::empty()).is_sign_positive());
        assert!((super::total([1.5, 2.0].into_iter()) - 3.5).abs() < f64::EPSILON);
    }
}
