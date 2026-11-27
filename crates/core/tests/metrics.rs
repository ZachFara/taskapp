use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use serde_json::json;
use taskapp_core::{
    Bucket, ChangeSource, Db, Error, FieldRef, Filter, FixedField, IssueId, IssuePatch, LabelId,
    Measure, Metrics, MetricsQuery, MetricsScope, NewIssue, Op, StatusId, cycles, issues, labels,
    metrics, palette::Look, settings,
};

type R = Result<(), Box<dyn std::error::Error>>;
const UI: ChangeSource = ChangeSource::Ui;
const CLI: ChangeSource = ChangeSource::Cli;
// Statuses from the migration: 2 unstarted, 3 started, 4 completed, 5 canceled.
const TODO: i64 = 2;
const DOING: i64 = 3;
const DONE: i64 = 4;
const CANCELED: i64 = 5;

fn d(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 1, day).unwrap_or_default()
}

fn noon(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, day, 12, 0, 0).unwrap()
}

async fn make(
    db: &Db,
    day: u32,
    points: f64,
    label: Option<LabelId>,
    src: ChangeSource,
) -> Result<IssueId, Error> {
    Ok(issues::create(
        db,
        NewIssue {
            title: format!("Issue {day}"),
            status_id: Some(StatusId(TODO)),
            points: Some(points),
            label_ids: label.map(|l| vec![l]),
            created_at: Some(noon(day)),
            ..NewIssue::default()
        },
        src,
    )
    .await?
    .id)
}

/// Moves the issue to `status` and dates the change row.
async fn mv(db: &Db, id: IssueId, status: i64, day: u32, src: ChangeSource) -> R {
    let before: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(id), 0) FROM changes")
        .fetch_one(&db.pool)
        .await?;
    let patch = IssuePatch {
        status_id: Some(StatusId(status)),
        ..IssuePatch::default()
    };
    issues::update(db, id, patch, src).await?;
    sqlx::query("UPDATE changes SET at = ? WHERE id > ?")
        .bind(noon(day))
        .bind(before)
        .execute(&db.pool)
        .await?;
    Ok(())
}

fn query(from: u32, to: u32, bucket: Bucket, filter: Option<Filter>) -> MetricsQuery {
    MetricsQuery {
        scope: MetricsScope::Range {
            from: d(from),
            to: d(to),
        },
        bucket,
        filter,
        measure: Measure::Count,
    }
}

/// bug: A (ui, started 2, done 4) and B (ui, started 3, done 3 by cli); C (cli, todo to done on 5 by cli);
/// D (ui, started 6, still open); E (ui, canceled on 3). Cycle 1 (days 1 to 3) holds B.
async fn people(db: &Db) -> Result<[IssueId; 5], Error> {
    let bug = labels::create(db, "bug", "red", None, UI).await?.id;
    let cy = cycles::create(db, None, d(1), d(3), Look::default(), UI).await?;
    let made = [
        make(db, 1, 3.0, Some(bug), UI).await?,
        make(db, 1, 2.0, Some(bug), UI).await?,
        make(db, 2, 1.0, None, CLI).await?,
        make(db, 2, 5.0, None, UI).await?,
        make(db, 2, 1.0, None, UI).await?,
    ];
    let patch = IssuePatch {
        cycle_id: Some(Some(cy.id)),
        ..IssuePatch::default()
    };
    issues::update(db, made[1], patch, UI).await?;
    Ok(made)
}

async fn world() -> Result<(Db, Metrics), Box<dyn std::error::Error>> {
    let db = Db::open_in_memory().await?;
    settings::set(&db, "timezone", &"UTC").await?;
    let [ann, bea, cat, dan, eve] = people(&db).await?;
    mv(&db, ann, DOING, 2, UI).await?;
    mv(&db, bea, DOING, 3, UI).await?;
    mv(&db, eve, CANCELED, 3, UI).await?;
    mv(&db, bea, DONE, 3, CLI).await?;
    mv(&db, ann, DONE, 4, UI).await?;
    mv(&db, cat, DONE, 5, CLI).await?;
    mv(&db, dan, DOING, 6, UI).await?;
    let m = metrics::metrics_on(&db, query(1, 7, Bucket::Day, None), d(7)).await?;
    Ok((db, m))
}

#[tokio::test]
async fn buckets_count_created_completed_scope_and_wip_per_day() -> R {
    let (_, m) = world().await?;
    let rows: Vec<_> = m
        .buckets
        .iter()
        .map(|b| {
            (
                b.created,
                b.completed,
                b.completed_points,
                b.scope,
                b.done_total,
                b.wip,
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            (2, 0, 0.0, 2, 0, 0),
            (3, 0, 0.0, 5, 0, 1),
            (0, 1, 2.0, 4, 1, 1), // E canceled leaves scope; B started and finished the same day
            (0, 1, 3.0, 4, 2, 0),
            (0, 1, 1.0, 4, 3, 0),
            (0, 0, 0.0, 4, 3, 1),
            (0, 0, 0.0, 4, 3, 1),
        ]
    );
    assert_eq!(
        (m.from, m.to, m.bucket, m.wip_now, m.thin),
        (d(1), d(7), Bucket::Day, 1, true)
    );
    Ok(())
}

#[tokio::test]
async fn lead_and_cycle_time_skip_issues_that_never_started() -> R {
    let (_, m) = world().await?;
    // Lead: A 3 days, B 2, C 3. Cycle: A 2, B 0; C went straight to done, so it has no cycle time.
    assert_eq!(
        (m.lead_time.n, m.lead_time.p50_days, m.lead_time.p85_days),
        (3, Some(3.0), Some(3.0))
    );
    assert_eq!(
        (m.cycle_time.n, m.cycle_time.p50_days, m.cycle_time.p85_days),
        (2, Some(0.0), Some(2.0))
    );
    let pts: Vec<_> = m
        .cycle_time
        .points
        .iter()
        .map(|p| (p.completed, p.days))
        .collect();
    assert_eq!(pts, [(d(3), 0.0), (d(4), 2.0)]);
    Ok(())
}

#[tokio::test]
async fn breakdowns_by_label_and_by_source() -> R {
    let (_, m) = world().await?;
    let labels: Vec<_> = m
        .by_label
        .iter()
        .map(|l| (l.name.as_str(), l.completed, l.points))
        .collect();
    assert_eq!(labels, [("bug", 2, 5.0), ("No label", 1, 1.0)]);
    let src: Vec<_> = m
        .by_source
        .iter()
        .map(|s| (s.source.as_str(), s.created, s.completed))
        .collect();
    assert_eq!(src, [("Ui", 4, 1), ("Cli", 1, 2)]);
    Ok(())
}

#[tokio::test]
async fn weekly_buckets_start_on_monday_and_a_cycle_scopes_to_its_members() -> R {
    let (db, _) = world().await?;
    // 2026-01-01 is a Thursday: the first week starts Monday 2025-12-29.
    let w = metrics::metrics_on(&db, query(1, 7, Bucket::Week, None), d(7)).await?;
    assert_eq!(
        w.buckets
            .iter()
            .map(|b| (b.start.to_string(), b.created, b.completed))
            .collect::<Vec<_>>(),
        [("2025-12-29".into(), 5, 2), ("2026-01-05".into(), 0, 1)]
    );
    let cy = cycles::list(&db).await?[0].id;
    let q = MetricsQuery {
        scope: MetricsScope::Cycle(cy),
        ..query(1, 1, Bucket::Day, None)
    };
    let m = metrics::metrics_on(&db, q, d(10)).await?;
    assert_eq!((m.from, m.to), (d(1), d(3)));
    assert_eq!(m.buckets.iter().map(|b| b.completed).sum::<u32>(), 1);
    // Velocity counts what finished inside each finished cycle, members or not.
    assert_eq!(
        (
            m.velocity.len(),
            m.velocity[0].count,
            m.velocity[0].points,
            m.velocity[0].name.as_str()
        ),
        (1, 1, 2.0, "Cycle 1")
    );
    Ok(())
}

#[tokio::test]
async fn filter_narrows_every_number_and_bad_queries_are_refused() -> R {
    let (db, _) = world().await?;
    let bug = Filter::Cond {
        field: FieldRef::Fixed(FixedField::Label),
        op: Op::In,
        value: json!([1]),
    };
    let m = metrics::metrics_on(&db, query(1, 7, Bucket::Day, Some(bug)), d(7)).await?;
    assert_eq!((m.lead_time.n, m.wip_now, m.by_source.len()), (2, 0, 2));
    let back = metrics::metrics_on(&db, query(5, 1, Bucket::Day, None), d(7)).await;
    assert!(matches!(back, Err(Error::Invalid(_))));
    let time = MetricsQuery {
        measure: Measure::Time,
        ..query(1, 7, Bucket::Day, None)
    };
    assert!(matches!(
        metrics::metrics_on(&db, time, d(7)).await,
        Err(Error::Invalid(_))
    ));
    let none = MetricsQuery {
        scope: MetricsScope::Cycle(taskapp_core::CycleId(99)),
        ..query(1, 1, Bucket::Day, None)
    };
    assert!(matches!(
        metrics::metrics_on(&db, none, d(7)).await,
        Err(Error::NotFound(_))
    ));
    Ok(())
}

#[tokio::test]
async fn imported_issue_without_history_has_lead_time_but_no_cycle_time() -> R {
    let db = Db::open_in_memory().await?;
    settings::set(&db, "timezone", &"UTC").await?;
    issues::create(
        &db,
        NewIssue {
            title: "Linear".into(),
            status_id: Some(StatusId(DONE)),
            created_at: Some(noon(1)),
            completed_at: Some(noon(4)),
            ..NewIssue::default()
        },
        ChangeSource::Migration,
    )
    .await?;
    sqlx::query("DELETE FROM changes").execute(&db.pool).await?;
    let m = metrics::metrics_on(&db, query(1, 7, Bucket::Week, None), d(7)).await?;
    assert_eq!(
        (m.lead_time.n, m.lead_time.p50_days, m.cycle_time.n),
        (1, Some(3.0), 0)
    );
    assert_eq!(m.by_source[0].source, "Migration");
    Ok(())
}
