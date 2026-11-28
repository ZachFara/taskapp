use chrono::{NaiveDate, TimeZone, Utc};
use taskapp_core::{
    ChangeSource, CycleId, Db, Error, ForecastQuery, IssueId, IssuePatch, NewIssue, StatusId,
    cycles, forecast::forecast_on, issues, palette::Look, settings,
};

type R = Result<(), Box<dyn std::error::Error>>;
const UI: ChangeSource = ChangeSource::Ui;
// Statuses from the migration: 2 unstarted, 4 completed.
const TODO: i64 = 2;
const DONE: i64 = 4;

fn d(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 1, day).unwrap_or_default()
}

async fn make(db: &Db) -> Result<IssueId, Error> {
    Ok(issues::create(
        db,
        NewIssue {
            title: "x".into(),
            status_id: Some(StatusId(TODO)),
            created_at: Some(Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap()),
            ..NewIssue::default()
        },
        UI,
    )
    .await?
    .id)
}

/// `done` issues finished on days 1..=done (one a day) and `open` still waiting.
async fn world(done: u32, open: u32) -> Result<Db, Box<dyn std::error::Error>> {
    let db = Db::open_in_memory().await?;
    settings::set(&db, "timezone", &"UTC").await?;
    for day in 1..=done {
        let id = make(&db).await?;
        let before: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(id), 0) FROM changes")
            .fetch_one(&db.pool)
            .await?;
        let patch = IssuePatch {
            status_id: Some(StatusId(DONE)),
            ..IssuePatch::default()
        };
        issues::update(&db, id, patch, UI).await?;
        sqlx::query("UPDATE changes SET at = ? WHERE id > ?")
            .bind(Utc.with_ymd_and_hms(2026, 1, day, 12, 0, 0).unwrap())
            .bind(before)
            .execute(&db.pool)
            .await?;
    }
    for _ in 0..open {
        make(&db).await?;
    }
    Ok(db)
}

fn all() -> ForecastQuery {
    ForecastQuery {
        filter: None,
        cycle: None,
    }
}

#[tokio::test]
async fn steady_history_gives_ordered_dates_and_the_same_answer_twice() -> R {
    let db = world(12, 5).await?;
    let a = forecast_on(&db, all(), d(28)).await?;
    assert_eq!((a.open, a.completions, a.enough), (5, 12, true));
    let (p50, p85) = (a.p50.unwrap_or_default(), a.p85.unwrap_or_default());
    assert!(d(28) < p50 && p50 <= p85, "{a:?}");
    // 12 done in 56 days is about 0.2 a day: five issues take about three weeks, far from 1 or 100 days.
    assert!((10..=45).contains(&(p50 - d(28)).num_days()), "{a:?}");
    assert_eq!(a, forecast_on(&db, all(), d(28)).await?);
    Ok(())
}

#[tokio::test]
async fn too_little_history_or_nothing_open_gives_no_dates() -> R {
    let thin = forecast_on(&world(9, 5).await?, all(), d(28)).await?;
    assert_eq!((thin.enough, thin.p50, thin.p85), (false, None, None));
    let none = forecast_on(&world(12, 0).await?, all(), d(28)).await?;
    assert_eq!((none.open, none.enough, none.p50), (0, true, None));
    Ok(())
}

#[tokio::test]
async fn old_completions_fall_out_of_the_window() -> R {
    let f = forecast_on(
        &world(12, 5).await?,
        all(),
        d(1) + chrono::Duration::days(80),
    )
    .await?;
    assert_eq!((f.completions, f.enough), (0, false));
    Ok(())
}

#[tokio::test]
async fn a_cycle_counts_only_its_members_and_an_unknown_one_is_not_found() -> R {
    let db = world(12, 5).await?;
    let cy = cycles::create(&db, None, d(20), d(30), Look::default(), UI).await?;
    let open: Vec<i64> = sqlx::query_scalar("SELECT id FROM issues WHERE status_id = ? LIMIT 2")
        .bind(TODO)
        .fetch_all(&db.pool)
        .await?;
    for id in open {
        let patch = IssuePatch {
            cycle_id: Some(Some(cy.id)),
            ..IssuePatch::default()
        };
        issues::update(&db, IssueId(id), patch, UI).await?;
    }
    let q = |c| ForecastQuery {
        filter: None,
        cycle: Some(c),
    };
    let f = forecast_on(&db, q(cy.id), d(28)).await?;
    assert_eq!((f.open, f.completions), (2, 12));
    let missing = forecast_on(&db, q(CycleId(99)), d(28)).await;
    assert!(matches!(missing, Err(Error::NotFound(_))));
    Ok(())
}
