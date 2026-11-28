//! Delivery metrics for a date range or a cycle: throughput, lead and cycle time, work in progress, and
//! breakdowns by label and by who made the change. Nothing is stored; every call replays the change history
//! (see [`crate::insights`] for the same idea behind the burn-up charts).

use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Datelike, Duration, Months, NaiveDate, Utc};
use chrono_tz::Tz;
use sqlx::Row;

#[path = "metrics_ext.rs"]
mod ext;

use crate::{
    Bucket, ChangeSource, Cycle, CycleTime, CycleTimePoint, Db, Error, Issue, LabelBreakdown,
    LeadTime, Measure, Metrics, MetricsBucket, MetricsQuery, MetricsScope, Relation, Result,
    SourceBreakdown, StatusCategory, VelocityCycle, ViewDef, cycles, focus,
    insights::{local, zone},
    labels, statuses, views,
};

const MAX_DAYS: i64 = 1100;
const THIN: usize = 5;
const VELOCITY_CYCLES: usize = 6;

/// One issue's story: when it was made and by whom, where its status went, and when it finished.
pub(crate) struct Facts {
    pub(crate) issue: Issue,
    made: NaiveDate,
    made_by: String,
    cat0: StatusCategory,
    /// (local day, category entered), oldest first.
    moves: Vec<(NaiveDate, StatusCategory)>,
    started: Option<NaiveDate>,
    pub(crate) done: Option<NaiveDate>,
    done_by: String,
    /// The status before the first move, and every status entered as (local day, status id).
    status0: i64,
    path: Vec<(NaiveDate, i64)>,
    /// The cycle before the first move, and every cycle change as (local day, cycle id entered).
    cycle0: Option<i64>,
    cycles: Vec<(NaiveDate, Option<i64>)>,
}

impl Facts {
    /// The cycle at the end of `day`.
    fn cycle_at(&self, day: NaiveDate) -> Option<i64> {
        self.cycles
            .iter()
            .rev()
            .find(|m| m.0 <= day)
            .map_or(self.cycle0, |m| m.1)
    }

    /// The category at the end of `day` and the day it was entered; `None` before the issue existed.
    pub(crate) fn at(&self, day: NaiveDate) -> Option<(StatusCategory, NaiveDate)> {
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

pub(crate) fn bucket_start(day: NaiveDate, b: Bucket) -> NaiveDate {
    match b {
        Bucket::Day => day,
        Bucket::Week => day - Duration::days(i64::from(day.weekday().num_days_from_monday())),
        Bucket::Month => day.with_day(1).unwrap_or(day),
    }
}

pub(crate) fn next_start(start: NaiveDate, b: Bucket) -> NaiveDate {
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
/// (when, old, new, made with the issue) of one cycle change; creation rows say what the issue was made into.
type CycleMove = (DateTime<Utc>, Option<i64>, Option<i64>, bool);
/// (when, relation, added) of one add or remove of a `blocks` relation.
type BlockEdit = (DateTime<Utc>, Relation, bool);
/// (local day, status id entered), oldest first.
type StatusPath = Vec<(NaiveDate, i64)>;
/// The days a query covers and the cycle it is about, if any.
type Span = (NaiveDate, NaiveDate, Option<Cycle>);

/// Every status change and the source of every creation, per issue.
struct Log {
    moves: HashMap<i64, Vec<Move>>,
    /// (when, old, new) of each cycle change, per issue.
    cycles: HashMap<i64, Vec<CycleMove>>,
    /// (when, relation, added) of each add or remove of a `blocks` relation.
    blocks: Vec<BlockEdit>,
    made_by: HashMap<i64, String>,
}

async fn log(db: &Db) -> Result<Log> {
    let rows = sqlx::query(
        "SELECT entity_id, field, at, source, old_value, new_value FROM changes
         WHERE entity = 'issue' AND field IN ('key', 'status_id', 'cycle_id', 'relation') ORDER BY at, id",
    )
    .fetch_all(&db.pool)
    .await?;
    let mut out = Log {
        moves: HashMap::new(),
        cycles: HashMap::new(),
        blocks: vec![],
        made_by: HashMap::new(),
    };
    let id_of =
        |v: Option<String>| v.and_then(|v| serde_json::from_str::<Option<i64>>(&v).ok().flatten());
    for r in &rows {
        let Ok(id) = r.try_get::<String, _>(0)?.parse::<i64>() else {
            continue;
        };
        let source = source_name(&r.try_get::<String, _>(3)?);
        let field = r.try_get::<String, _>(1)?;
        if field == "key" {
            out.made_by.entry(id).or_insert(source);
        } else if field == "cycle_id" {
            let raw: Option<String> = r.try_get(4)?;
            let (old, new) = (id_of(raw.clone()), id_of(r.try_get(5)?));
            out.cycles
                .entry(id)
                .or_default()
                .push((r.try_get(2)?, old, new, raw.is_none()));
        } else if field == "relation" {
            let added = r.try_get::<Option<String>, _>(5)?;
            let json = added.clone().or(r.try_get(4)?);
            if let Some(rel) = json.and_then(|j| serde_json::from_str::<Relation>(&j).ok())
                && rel.kind.0 == "blocks"
            {
                out.blocks.push((r.try_get(2)?, rel, added.is_some()));
            }
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

/// The status before the first move, and each status entered as (local day, id).
fn status_path(issue: &Issue, events: &[Move], tz: Tz) -> (i64, StatusPath) {
    let sid = |s: &Option<String>| s.as_deref().and_then(|s| s.parse::<i64>().ok());
    (
        events
            .first()
            .and_then(|e| sid(&e.2))
            .unwrap_or(issue.status_id.0),
        events
            .iter()
            .filter_map(|e| Some((local(e.0, tz), sid(&e.3)?)))
            .collect(),
    )
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
    let made = local(issue.created_at, tz);
    let (status0, path) = status_path(&issue, events, tz);
    let cyc = log.cycles.get(&issue.id.0).map_or(&[][..], Vec::as_slice);
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
        made,
        made_by: log
            .made_by
            .get(&issue.id.0)
            .cloned()
            .unwrap_or_else(|| "Migration".into()),
        cat0: before,
        status0,
        path,
        cycle0: cyc.first().map_or(issue.cycle_id.map(|c| c.0), |c| c.1),
        // A creation row stands at the issue's own creation day, whenever the row was written.
        cycles: cyc
            .iter()
            .map(|c| (if c.3 { made } else { local(c.0, tz) }, c.2))
            .collect(),
        moves: moves.into_iter().map(|m| (m.0, m.1)).collect(),
        started,
        done,
        done_by,
        issue,
    }
}

async fn facts(
    db: &Db,
    def: &ViewDef,
    log: &Log,
    cats: &HashMap<i64, StatusCategory>,
    tz: Tz,
) -> Result<Vec<Facts>> {
    Ok(views::query(db, def)
        .await?
        .into_iter()
        .map(|i| story(i, log, cats, tz))
        .collect())
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

/// The story of every issue the filter lets through (for the forecast).
pub(crate) async fn facts_for(db: &Db, filter: Option<crate::Filter>) -> Result<Vec<Facts>> {
    let tz = zone(db).await?;
    let cats: HashMap<i64, StatusCategory> = statuses::list(db)
        .await?
        .into_iter()
        .map(|s| (s.id.0, s.category))
        .collect();
    let def = ViewDef {
        filter,
        ..ViewDef::default()
    };
    facts(db, &def, &log(db).await?, &cats, tz).await
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

fn started_at(f: &Facts, day: NaiveDate) -> bool {
    matches!(f.at(day), Some((StatusCategory::Started, _)))
}

/// Velocity looks across cycles, so `all` keeps every filtered issue; the rest is the cycle's members and what they finished.
fn members<'a>(
    all: &'a [Facts],
    cycle: Option<&Cycle>,
    from: NaiveDate,
    to: NaiveDate,
) -> (Vec<&'a Facts>, Vec<&'a Facts>) {
    let mine: Vec<_> = all
        .iter()
        .filter(|f| cycle.is_none_or(|c| f.issue.cycle_id == Some(c.id)))
        .collect();
    let done = mine
        .iter()
        .copied()
        .filter(|f| f.finished_in(from, to))
        .collect();
    (mine, done)
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
    let sts = statuses::list(db).await?;
    let cats: HashMap<i64, StatusCategory> = sts.iter().map(|s| (s.id.0, s.category)).collect();
    let log = log(db).await?;
    let def = ViewDef {
        filter: q.filter,
        ..ViewDef::default()
    };
    let all = facts(db, &def, &log, &cats, tz).await?;
    let (mine, done) = members(&all, cycle.as_ref(), from, to);
    let (lead_time, cycle_time) = times(&done);
    let every = ext::everyone(db, &def, &log, &cats, tz).await?;
    let links = ext::links(db, &log, every.as_deref().unwrap_or(&all), tz).await?;
    Ok(Metrics {
        from,
        to,
        bucket: q.bucket,
        buckets: buckets(&mine, from, to, q.bucket),
        lead_time,
        cycle_time,
        wip_now: count(mine.iter().filter(|f| started_at(f, today)).count()),
        by_label: by_label(db, &done, q.measure).await?,
        by_source: by_source(&mine, from, to),
        velocity: velocity(&all, &all_cycles, today),
        thin: done.len() < THIN,
        cycle: cycle.as_ref().map(|c| ext::commit(&all, c, today)),
        blocked: Some(ext::blocked(
            &mine,
            &done,
            every.as_deref().unwrap_or(&all),
            &links,
            today,
        )),
        flow: Some(ext::flow(&done, &sts)),
        deep: Some(focus::deep_hours(db, from, to, q.bucket, Utc::now(), tz).await?),
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
