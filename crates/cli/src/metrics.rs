//! `taskapp metrics`: delivery numbers for a range or a cycle, as a readable table or JSON.

use std::fmt::Write;

use chrono::{Datelike, Duration, NaiveDate};
use serde_json::{Value, json};
use taskapp_core::{
    Bucket, Db, Error, FieldRef, Filter, FixedField, Measure, Metrics, MetricsQuery, MetricsScope,
    Op, Result, metrics,
};

use crate::{references, render_table};

#[derive(clap::Args)]
pub struct MetricsArgs {
    /// today, week, month, quarter or year (each up to today), or FROM..TO as YYYY-MM-DD..YYYY-MM-DD.
    #[arg(long, default_value = "week", conflicts_with = "cycle")]
    range: String,
    /// A cycle's name or id: its own dates and its members.
    #[arg(long)]
    cycle: Option<String>,
    /// Only issues of this project (name, slug or id).
    #[arg(long)]
    project: Option<String>,
    /// Only issues with this label.
    #[arg(long)]
    label: Option<String>,
    /// Rank the label breakdown by points instead of issue count.
    #[arg(long)]
    points: bool,
}

fn date(text: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d")
        .map_err(|_| Error::Invalid(format!("{text} is not a date (YYYY-MM-DD)")))
}

fn range(spec: &str, today: NaiveDate) -> Result<(NaiveDate, NaiveDate)> {
    let first_month = today.month0() / 3 * 3 + 1;
    let from = match spec {
        "today" => today,
        "week" => today - Duration::days(i64::from(today.weekday().num_days_from_monday())),
        "month" => today.with_day(1).unwrap_or(today),
        "quarter" => NaiveDate::from_ymd_opt(today.year(), first_month, 1).unwrap_or(today),
        "year" => NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap_or(today),
        other => {
            let (a, b) = other.split_once("..").ok_or_else(|| {
                Error::Invalid(format!(
                    "range {other}: use today, week, month, quarter, year or FROM..TO"
                ))
            })?;
            return Ok((date(a)?, date(b)?));
        }
    };
    Ok((from, today))
}

/// Day buckets up to a month, week buckets up to about six months, month buckets beyond.
fn bucket(from: NaiveDate, to: NaiveDate) -> Bucket {
    match (to - from).num_days() {
        ..=30 => Bucket::Day,
        31..=182 => Bucket::Week,
        _ => Bucket::Month,
    }
}

async fn filter(db: &Db, args: &MetricsArgs) -> Result<Option<Filter>> {
    let mut parts = vec![];
    let mut has = |field, ids: Vec<i64>| {
        parts.push(Filter::Cond {
            field: FieldRef::Fixed(field),
            op: Op::In,
            value: json!(ids),
        });
    };
    if let Some(p) = references::project(db, args.project.as_deref()).await? {
        has(FixedField::Project, vec![p.0]);
    }
    if let Some(name) = &args.label {
        let ids = references::labels(db, std::slice::from_ref(name)).await?;
        has(FixedField::Label, ids.iter().map(|l| l.0).collect());
    }
    Ok((!parts.is_empty()).then_some(Filter::And(parts)))
}

async fn run(db: &Db, args: &MetricsArgs) -> Result<Metrics> {
    let today = metrics::today(db).await?;
    let (scope, from, to) = if let Some(id) = references::cycle(db, args.cycle.as_deref()).await? {
        let all = taskapp_core::cycles::list(db).await?;
        let c = all
            .iter()
            .find(|x| x.id == id)
            .ok_or_else(|| Error::NotFound(format!("cycle {}", id.0)))?;
        (MetricsScope::Cycle(id), c.start_date, c.end_date)
    } else {
        let (from, to) = range(&args.range, today)?;
        (MetricsScope::Range { from, to }, from, to)
    };
    let q = MetricsQuery {
        scope,
        bucket: bucket(from, to),
        filter: filter(db, args).await?,
        measure: if args.points {
            Measure::Points
        } else {
            Measure::Count
        },
    };
    metrics::metrics_on(db, q, today).await
}

fn days(p50: Option<f64>, p85: Option<f64>, n: u32) -> String {
    let d = |v: Option<f64>| v.map_or_else(|| "-".into(), |v| format!("{v:.1}d"));
    format!("p50 {}, p85 {} (n={n})", d(p50), d(p85))
}

fn table(rows: &[Value]) -> String {
    if rows.is_empty() {
        "(none)\n".into()
    } else {
        render_table(rows)
    }
}

/// The cycle, blocked and flow sections, when the metrics carry them.
fn extras(m: &Metrics) -> String {
    let mut extra = String::new();
    if let Some(c) = &m.cycle {
        let p = |v: Option<f64>| v.map_or_else(|| "-".into(), |v| format!("{v:.0}%"));
        let _ = write!(
            extra,
            "\nCycle commitment: {} committed ({} pts), {} done: say/do {} ({} by points)\nScope change: +{} / -{} issues ({} of commitment, {} by points)\n",
            c.committed,
            c.committed_points,
            c.done,
            p(c.say_do_pct),
            p(c.say_do_points_pct),
            c.added,
            c.removed,
            p(c.scope_change_pct),
            p(c.scope_change_points_pct),
        );
    }
    if let Some(b) = &m.blocked {
        let now: Vec<Value> = b
            .now
            .iter()
            .map(|i| json!({"issue": i.key, "title": i.title, "blocked_by": i.blocked_by.join(", "), "days": i.days}))
            .collect();
        let _ = write!(
            extra,
            "\nBlocked now\n{}Blocked time per finished issue (approx., whole days): avg {}\n",
            table(&now),
            b.avg_days
                .map_or_else(|| "-".into(), |v| format!("{v:.1}d")),
        );
    }
    if let Some(f) = &m.flow {
        let waiting: Vec<Value> = f
            .waiting
            .iter()
            .map(|w| json!({"waiting status": w.name, "days": w.days, "issues": w.issues}))
            .collect();
        let _ = write!(
            extra,
            "\nFlow efficiency: avg {} (n={}), bins of 20%: {:?}\n{}",
            f.avg_pct.map_or_else(|| "-".into(), |v| format!("{v:.0}%")),
            f.n,
            f.distribution,
            table(&waiting),
        );
    }
    extra
}

/// The readable form: a summary, then one small table per breakdown.
pub fn text(m: &Metrics) -> String {
    let buckets: Vec<Value> = m
        .buckets
        .iter()
        .map(|b| {
            json!({"start": b.start.to_string(), "created": b.created, "completed": b.completed,
            "points": b.completed_points, "wip": b.wip, "scope": b.scope, "done": b.done_total})
        })
        .collect();
    let labels: Vec<Value> = m
        .by_label
        .iter()
        .map(|l| json!({"label": l.name, "completed": l.completed, "points": l.points}))
        .collect();
    let sources: Vec<Value> = m
        .by_source
        .iter()
        .map(|s| json!({"source": s.source, "created": s.created, "completed": s.completed}))
        .collect();
    let velocity: Vec<Value> = m
        .velocity
        .iter()
        .map(|v| json!({"cycle": v.name, "completed": v.count, "points": v.points}))
        .collect();
    let thin = if m.thin {
        "Few finished issues (under 5): read these as anecdotes.\n"
    } else {
        ""
    };
    let extra = extras(m);
    format!(
        "Metrics {} to {} (per {:?})\n{thin}Lead time:  {}\nCycle time: {}\nIn progress now: {}\n\n{}\nBy label\n{}\nBy source\n{}\nVelocity (last finished cycles)\n{}{extra}",
        m.from,
        m.to,
        m.bucket,
        days(m.lead_time.p50_days, m.lead_time.p85_days, m.lead_time.n),
        days(m.cycle_time.p50_days, m.cycle_time.p85_days, m.cycle_time.n),
        m.wip_now,
        table(&buckets),
        table(&labels),
        table(&sources),
        table(&velocity),
    )
}

/// Prints the metrics as JSON or as the readable text.
pub async fn print(db: &Db, args: &MetricsArgs, json: bool) -> Result<()> {
    let m = run(db, args).await?;
    if json {
        let v = serde_json::to_value(&m).map_err(|e| Error::Invalid(e.to_string()))?;
        println!("{v}");
    } else {
        print!("{}", text(&m));
    }
    Ok(())
}
