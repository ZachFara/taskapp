//! Progress over time, rebuilt from change history: for each local day, how much of a measure was in scope,
//! started and completed. Nothing is stored; every call replays the status and points/estimate changes of the
//! issues in scope. Membership is who is in scope now, and an issue counts from the day it was created.

use std::collections::HashMap;

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use chrono_tz::Tz;
use sqlx::Row;

use crate::{
    Db, Error, Measure, ProgressDay, ProgressScope, ProgressSeries, Result, StatusCategory,
    VelocityPeriod, cycles, projects, statuses::parse_category, views, workspace,
};

const MAX_DAYS: i64 = 1100;

/// One issue's state over time; each `Vec` holds (local day, value from then on), oldest first.
struct Timeline {
    created: NaiveDate,
    category0: StatusCategory,
    category: Vec<(NaiveDate, StatusCategory)>,
    value0: f64,
    value: Vec<(NaiveDate, f64)>,
    /// Whether the issue is in the scope's cycle; always true for other scopes.
    member0: bool,
    member: Vec<(NaiveDate, bool)>,
    /// A cycle's days (first included, last excluded) on which the issue only carries work finished before the cycle.
    carried: Option<(NaiveDate, NaiveDate)>,
}

fn latest<T: Copy>(initial: T, events: &[(NaiveDate, T)], day: NaiveDate) -> T {
    events
        .iter()
        .rev()
        .find(|(d, _)| *d <= day)
        .map_or(initial, |e| e.1)
}

impl Timeline {
    /// (scope, started, completed) of this issue at the end of `day`.
    fn at(&self, day: NaiveDate) -> (f64, f64, f64) {
        let carried = self.carried.is_some_and(|(a, b)| a <= day && day < b);
        if day < self.created || carried || !latest(self.member0, &self.member, day) {
            return (0.0, 0.0, 0.0);
        }
        let v = latest(self.value0, &self.value, day);
        match latest(self.category0, &self.category, day) {
            // Canceled work leaves the scope instead of lowering the percentage forever.
            StatusCategory::Canceled => (0.0, 0.0, 0.0),
            StatusCategory::Completed => (v, v, v),
            StatusCategory::Started => (v, v, 0.0),
            StatusCategory::Backlog | StatusCategory::Unstarted => (v, 0.0, 0.0),
        }
    }
}

fn totals(tls: &[Timeline], day: NaiveDate) -> (f64, f64, f64) {
    tls.iter().fold((0.0, 0.0, 0.0), |a, t| {
        let b = t.at(day);
        (a.0 + b.0, a.1 + b.1, a.2 + b.2)
    })
}

async fn zone(db: &Db) -> Result<Tz> {
    let name = workspace::status(db).await?.timezone;
    name.parse()
        .map_err(|_| Error::Invalid(format!("unknown time zone {name}")))
}

fn local(at: DateTime<Utc>, tz: Tz) -> NaiveDate {
    at.with_timezone(&tz).date_naive()
}

/// The issues in scope now; archived ones are left out unless the view asks for them.
async fn scope_ids(db: &Db, scope: ProgressScope) -> Result<Vec<i64>> {
    let by_column = |sql: &'static str, id: i64| async move {
        Ok::<_, Error>(
            sqlx::query_scalar::<_, i64>(sql)
                .bind(id)
                .fetch_all(&db.pool)
                .await?,
        )
    };
    match scope {
        ProgressScope::Project(p) => {
            if !projects::list(db).await?.iter().any(|x| x.id == p) {
                return Err(Error::NotFound(format!("project {}", p.0)));
            }
            by_column(
                "SELECT id FROM issues WHERE project_id = ? AND archived_at IS NULL",
                p.0,
            )
            .await
        }
        ProgressScope::Cycle(c) => {
            if !cycles::list(db).await?.iter().any(|x| x.id == c) {
                return Err(Error::NotFound(format!("cycle {}", c.0)));
            }
            // Issues that left the cycle still count on the days they were in it.
            by_column(
                "SELECT id FROM issues WHERE archived_at IS NULL AND (cycle_id = ?1 OR id IN
                   (SELECT CAST(entity_id AS INTEGER) FROM changes
                    WHERE entity = 'issue' AND field = 'cycle_id' AND old_value = CAST(?1 AS TEXT)))",
                c.0,
            )
            .await
        }
        ProgressScope::View(v) => {
            let view = views::list(db)
                .await?
                .into_iter()
                .find(|x| x.id == v)
                .ok_or_else(|| Error::NotFound(format!("view {}", v.0)))?;
            Ok(views::query(db, &view.def)
                .await?
                .into_iter()
                .map(|i| i.id.0)
                .collect())
        }
    }
}

fn number(v: Option<&str>) -> f64 {
    v.and_then(|s| serde_json::from_str::<Option<f64>>(s).ok().flatten())
        .unwrap_or(0.0)
}

/// (when, old, new) of status and value changes, per (issue, field), oldest first.
type Events = HashMap<(i64, String), Vec<(DateTime<Utc>, Option<String>, Option<String>, i64)>>;

async fn events(db: &Db, value_field: &str) -> Result<Events> {
    let rows = sqlx::query(
        "SELECT entity_id, field, at, old_value, new_value, id FROM changes
         WHERE entity = 'issue' AND field IN ('status_id', 'cycle_id', 'key', ?) ORDER BY at, id",
    )
    .bind(value_field)
    .fetch_all(&db.pool)
    .await?;
    let mut out = Events::new();
    for r in &rows {
        let Ok(id) = r.try_get::<String, _>(0)?.parse::<i64>() else {
            continue;
        };
        out.entry((id, r.try_get(1)?)).or_default().push((
            r.try_get(2)?,
            r.try_get(3)?,
            r.try_get(4)?,
            r.try_get(5)?,
        ));
    }
    Ok(out)
}

/// What every issue's timeline is built from.
struct Known {
    cats: HashMap<i64, StatusCategory>,
    events: Events,
    measure: Measure,
    value_field: &'static str,
    /// The cycle being charted and its first day, for a cycle scope.
    cycle: Option<(i64, NaiveDate)>,
    tz: Tz,
}

/// One issue row (id, status, points, minutes, created, cycle) replayed against its change history.
fn timeline_of(k: &Known, r: &sqlx::sqlite::SqliteRow) -> Result<Timeline> {
    let (tz, cycle, events) = (k.tz, k.cycle, &k.events);
    let id: i64 = r.try_get(0)?;
    let cat = |id: i64| k.cats.get(&id).copied().unwrap_or(StatusCategory::Backlog);
    let current = match k.measure {
        Measure::Points => r.try_get::<Option<f64>, _>(2)?.unwrap_or(0.0),
        Measure::Time => r.try_get::<Option<f64>, _>(3)?.unwrap_or(0.0),
        Measure::Count => 1.0,
    };
    let of = |field: &str| {
        events
            .get(&(id, field.into()))
            .map_or(&[][..], Vec::as_slice)
    };
    let (status_events, value_events) = (of("status_id"), of(k.value_field));
    // Creation records no status or value, so the state before the first change is that change's old value.
    let status_id = |s: &Option<String>| s.as_deref().and_then(|s| s.parse::<i64>().ok());
    // Creation records its rows (the key last) before anything else can: they are the starting state, not a move.
    let made = of("key").first().map_or(0, |e| e.3);
    let all_cycle = of("cycle_id");
    let cycle_events = &all_cycle[all_cycle.iter().take_while(|e| e.3 < made).count()..];
    let in_cycle = |v: Option<i64>| cycle.is_none_or(|(c, _)| v == Some(c));
    let category0 = cat(status_events
        .first()
        .and_then(|e| status_id(&e.1))
        .unwrap_or(r.try_get(1)?));
    let category: Vec<_> = status_events
        .iter()
        .filter_map(|e| Some((local(e.0, tz), cat(status_id(&e.2)?))))
        .collect();
    let carried = cycle.and_then(|(_, start)| {
        let before = latest(category0, &category, start - Duration::days(1));
        (before == StatusCategory::Completed).then(|| {
            let reopened = category.iter().map(|e| e.0).find(|d| *d >= start);
            (start, reopened.unwrap_or(NaiveDate::MAX))
        })
    });
    Ok(Timeline {
        created: local(r.try_get(4)?, tz),
        category0,
        category,
        member0: in_cycle(match cycle_events.first() {
            Some(e) => status_id(&e.1),
            None => r.try_get(5)?,
        }),
        member: cycle_events
            .iter()
            .map(|e| (local(e.0, tz), in_cycle(status_id(&e.2))))
            .collect(),
        carried,
        value0: value_events
            .first()
            .map_or(current, |e| number(e.1.as_deref())),
        value: value_events
            .iter()
            .map(|e| (local(e.0, tz), number(e.2.as_deref())))
            .collect(),
    })
}

async fn timelines(
    db: &Db,
    scope: ProgressScope,
    measure: Measure,
    tz: Tz,
) -> Result<Vec<Timeline>> {
    let ids = scope_ids(db, scope).await?;
    let cycle = match scope {
        ProgressScope::Cycle(c) => cycles::list(db)
            .await?
            .into_iter()
            .find(|x| x.id == c)
            .map(|c| (c.id.0, c.start_date)),
        _ => None,
    };
    let cats = sqlx::query("SELECT id, category FROM statuses")
        .fetch_all(&db.pool)
        .await?
        .iter()
        .map(|r| Ok((r.try_get(0)?, parse_category(&r.try_get::<String, _>(1)?)?)))
        .collect::<Result<_>>()?;
    let value_field = match measure {
        Measure::Points => "points",
        Measure::Time => "estimate_minutes",
        Measure::Count => "",
    };
    let known = Known {
        cats,
        events: events(db, value_field).await?,
        measure,
        value_field,
        cycle,
        tz,
    };
    sqlx::query(
        "SELECT id, status_id, points, CAST(estimate_minutes AS REAL), created_at, cycle_id FROM issues",
    )
    .fetch_all(&db.pool)
    .await?
    .iter()
    .filter(|r| r.try_get::<i64, _>(0).is_ok_and(|id| ids.contains(&id)))
    .map(|r| timeline_of(&known, r))
    .collect()
}

/// The days the series covers by default: a project's dates, a cycle's dates, a view's last 30 days.
async fn default_range(
    db: &Db,
    scope: ProgressScope,
    tls: &[Timeline],
    today: NaiveDate,
) -> Result<(NaiveDate, NaiveDate)> {
    let activity = || {
        let first = tls.iter().map(|t| t.created).min();
        let last = tls
            .iter()
            .flat_map(|t| {
                [t.created]
                    .into_iter()
                    .chain(t.category.iter().map(|e| e.0))
                    .chain(t.value.iter().map(|e| e.0))
            })
            .max();
        first.zip(last).unwrap_or((today, today))
    };
    Ok(match scope {
        ProgressScope::Project(p) => {
            let p = projects::list(db).await?.into_iter().find(|x| x.id == p);
            let (first, last) = activity();
            let from = p.as_ref().and_then(|p| p.start_date).unwrap_or(first);
            (
                from,
                p.and_then(|p| p.target_date).unwrap_or(last).max(from),
            )
        }
        ProgressScope::Cycle(c) => cycles::list(db)
            .await?
            .into_iter()
            .find(|x| x.id == c)
            .map_or((today, today), |c| (c.start_date, c.end_date)),
        ProgressScope::View(_) => (today - Duration::days(29), today),
    })
}

/// [`progress`] with the clock given, so a test can fix "today".
///
/// # Errors
/// `NotFound` for an unknown project, cycle or view; `Invalid` when `to` is before `from` or the range is over
/// 1100 days.
pub async fn progress_on(
    db: &Db,
    scope: ProgressScope,
    measure: Measure,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    today: NaiveDate,
) -> Result<ProgressSeries> {
    let tz = zone(db).await?;
    let tls = timelines(db, scope, measure, tz).await?;
    let (dfrom, dto) = default_range(db, scope, &tls, today).await?;
    let (from, to) = (from.unwrap_or(dfrom), to.unwrap_or(dto));
    if to < from || (to - from).num_days() >= MAX_DAYS {
        return Err(Error::Invalid(format!(
            "range {from} to {to} must be 1 to {MAX_DAYS} days"
        )));
    }
    let days = from
        .iter_days()
        .take_while(|d| *d <= to)
        .map(|date| {
            let (scope, started, completed) = totals(&tls, date);
            ProgressDay {
                date,
                scope,
                started,
                completed,
            }
        })
        .collect();
    Ok(ProgressSeries {
        days,
        measure,
        from,
        to,
    })
}

/// Totals per local day for the issues in `scope`. Without `from` and `to` the range is the scope's own dates.
///
/// # Errors
/// As [`progress_on`].
pub async fn progress(
    db: &Db,
    scope: ProgressScope,
    measure: Measure,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
) -> Result<ProgressSeries> {
    let today = local(Utc::now(), zone(db).await?);
    progress_on(db, scope, measure, from, to, today).await
}

/// [`velocity`] with the clock given.
///
/// # Errors
/// As [`progress_on`].
pub async fn velocity_on(
    db: &Db,
    scope: ProgressScope,
    measure: Measure,
    periods: u32,
    today: NaiveDate,
) -> Result<Vec<VelocityPeriod>> {
    let tz = zone(db).await?;
    let tls = timelines(db, scope, measure, tz).await?;
    let n = periods as usize;
    let mut cs: Vec<_> = cycles::list(db)
        .await?
        .into_iter()
        .filter(|c| c.start_date <= today)
        .collect();
    cs.sort_by_key(|c| c.number);
    let spans: Vec<(NaiveDate, NaiveDate, String)> = if cycles::list(db).await?.is_empty() {
        let monday = today - Duration::days(i64::from(today.weekday().num_days_from_monday()));
        (0..i64::from(periods))
            .rev()
            .map(|k| {
                let start = monday - Duration::days(7 * k);
                (
                    start,
                    start + Duration::days(6),
                    format!("Week of {}", start.format("%b %-d")),
                )
            })
            .collect()
    } else {
        cs.split_off(cs.len().saturating_sub(n))
            .into_iter()
            .map(|c| {
                (
                    c.start_date,
                    c.end_date,
                    c.name.unwrap_or_else(|| format!("Cycle {}", c.number)),
                )
            })
            .collect()
    };
    Ok(spans
        .into_iter()
        .map(|(start, end, label)| {
            // Net of reopened work, so a period never reports less than nothing.
            let completed =
                (totals(&tls, end).2 - totals(&tls, start - Duration::days(1)).2).max(0.0);
            VelocityPeriod {
                start,
                end,
                label,
                completed,
            }
        })
        .collect())
}

/// Completed per cycle when cycles exist (the last `periods` that have started), else per week.
///
/// # Errors
/// As [`progress_on`].
pub async fn velocity(
    db: &Db,
    scope: ProgressScope,
    measure: Measure,
    periods: u32,
) -> Result<Vec<VelocityPeriod>> {
    let today = local(Utc::now(), zone(db).await?);
    velocity_on(db, scope, measure, periods, today).await
}
