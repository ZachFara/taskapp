use chrono::{Duration, NaiveDate, Utc};
use serde_json::json;
use taskapp_core::{
    Bucket, ChangeSource, Db, Depth, FieldRef, FixedField, IssuePatch, Measure, MetricsQuery,
    MetricsScope, NewIssue, Op, ViewDef, focus, history, issues, metrics, undo, views,
};

type R = Result<(), Box<dyn std::error::Error>>;

async fn issue(db: &Db, title: &str) -> Result<taskapp_core::Issue, taskapp_core::Error> {
    issues::create(
        db,
        NewIssue {
            title: title.into(),
            ..NewIssue::default()
        },
        ChangeSource::Ui,
    )
    .await
}

#[tokio::test]
async fn start_and_stop_write_one_session() -> R {
    let db = Db::open_in_memory().await?;
    let a = issue(&db, "Draft").await?;
    let s = focus::start(&db, a.id, None, ChangeSource::Ui).await?;
    assert_eq!((s.issue_id, s.deep, s.ended_at), (a.id, false, None));
    assert_eq!(focus::running(&db).await?, Some(s.clone()));
    let ended = history::step("Stop focus", async {
        focus::stop(&db, ChangeSource::Ui).await
    })
    .await?
    .ok_or("a session was running")?;
    assert!(ended.ended_at.is_some_and(|e| e >= ended.started_at));
    assert_eq!(focus::running(&db).await?, None);
    assert!(focus::stop(&db, ChangeSource::Ui).await?.is_none());
    undo::undo(&db).await?;
    assert_eq!(focus::running(&db).await?.map(|r| r.id), Some(s.id));
    Ok(())
}

#[tokio::test]
async fn a_second_start_ends_the_running_session() -> R {
    let db = Db::open_in_memory().await?;
    let a = issue(&db, "First").await?;
    let b = issue(&db, "Second").await?;
    let first = focus::start(&db, a.id, None, ChangeSource::Ui).await?;
    let second = focus::start(&db, b.id, Some(true), ChangeSource::Ui).await?;
    assert_eq!(focus::running(&db).await?, Some(second.clone()));
    let all = focus::log(&db, None, None, None).await?;
    assert_eq!(all.len(), 2);
    assert!(all.iter().any(|s| s.id == first.id && s.ended_at.is_some()));
    assert!(second.deep);
    Ok(())
}

#[tokio::test]
async fn deep_defaults_to_the_issue_depth() -> R {
    let db = Db::open_in_memory().await?;
    let a = issue(&db, "Thinking").await?;
    let patched = issues::update(
        &db,
        a.id,
        IssuePatch {
            depth: Some(Some(Depth::Deep)),
            ..IssuePatch::default()
        },
        ChangeSource::Ui,
    )
    .await?;
    assert_eq!(patched.depth, Some(Depth::Deep));
    let s = focus::start(&db, a.id, None, ChangeSource::Ui).await?;
    assert!(s.deep);
    let cleared = issues::update(
        &db,
        a.id,
        IssuePatch {
            depth: Some(None),
            ..IssuePatch::default()
        },
        ChangeSource::Ui,
    )
    .await?;
    assert_eq!(cleared.depth, None);
    Ok(())
}

#[tokio::test]
async fn an_unknown_issue_is_not_found() -> R {
    let db = Db::open_in_memory().await?;
    let err = focus::start(&db, taskapp_core::IssueId(999), None, ChangeSource::Ui).await;
    assert!(matches!(err, Err(taskapp_core::Error::NotFound(_))));
    assert_eq!(focus::running(&db).await?, None);
    Ok(())
}

#[tokio::test]
async fn deep_hours_sum_the_deep_sessions_in_the_range() -> R {
    let db = Db::open_in_memory().await?;
    let a = issue(&db, "Deep one").await?;
    focus::start(&db, a.id, Some(true), ChangeSource::Ui).await?;
    focus::stop(&db, ChangeSource::Ui).await?;
    // A 90-minute deep session two days ago, and a shallow one today that must not count.
    let two_days_ago = Utc::now() - Duration::days(2);
    sqlx::query("UPDATE focus_sessions SET started_at = ?, ended_at = ? WHERE deep = 1")
        .bind(two_days_ago)
        .bind(two_days_ago + Duration::minutes(90))
        .execute(&db.pool)
        .await?;
    focus::start(&db, a.id, Some(false), ChangeSource::Ui).await?;
    focus::stop(&db, ChangeSource::Ui).await?;
    let today = metrics::today(&db).await?;
    let q = |from| MetricsQuery {
        scope: MetricsScope::Range { from, to: today },
        bucket: Bucket::Day,
        filter: None,
        measure: Measure::Count,
    };
    let week = metrics::metrics_for(&db, q(today - Duration::days(7)))
        .await?
        .deep
        .ok_or("deep hours")?;
    assert!(
        (week.total_hours - 1.5).abs() < 0.01,
        "total {}",
        week.total_hours
    );
    assert_eq!(week.streak_days, 0);
    let day = metrics::metrics_for(&db, q(today))
        .await?
        .deep
        .ok_or("deep hours")?;
    assert!(day.total_hours.abs() < 0.01);
    Ok(())
}

#[tokio::test]
async fn log_filters_by_issue_and_local_day() -> R {
    let db = Db::open_in_memory().await?;
    let a = issue(&db, "Logged").await?;
    let b = issue(&db, "Other").await?;
    focus::start(&db, a.id, None, ChangeSource::Ui).await?;
    focus::start(&db, b.id, None, ChangeSource::Ui).await?;
    focus::stop(&db, ChangeSource::Ui).await?;
    assert_eq!(focus::log(&db, Some(a.id), None, None).await?.len(), 1);
    let future: NaiveDate = "2999-01-01".parse()?;
    assert_eq!(focus::log(&db, None, Some(future), None).await?.len(), 0);
    Ok(())
}

#[tokio::test]
async fn views_filter_by_depth() -> R {
    let db = Db::open_in_memory().await?;
    let deep = issue(&db, "Deep").await?;
    issue(&db, "Unset").await?;
    issues::update(
        &db,
        deep.id,
        IssuePatch {
            depth: Some(Some(Depth::Deep)),
            ..IssuePatch::default()
        },
        ChangeSource::Ui,
    )
    .await?;
    let only_deep = ViewDef {
        filter: Some(taskapp_core::Filter::Cond {
            field: FieldRef::Fixed(FixedField::Depth),
            op: Op::Eq,
            value: json!("deep"),
        }),
        ..ViewDef::default()
    };
    let found = views::query(&db, &only_deep).await?;
    assert_eq!(
        found.iter().map(|i| i.id).collect::<Vec<_>>(),
        vec![deep.id]
    );
    Ok(())
}
