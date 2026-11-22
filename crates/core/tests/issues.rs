use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use chrono_tz::Europe::Berlin;
use taskapp_core::{
    ChangeSource, Db, Due, Error, IssueId, IssuePatch, NewIssue, StatusCategory, StatusId, history,
    issues, search, statuses,
};

type R = Result<(), Box<dyn std::error::Error>>;

fn new(title: &str) -> NewIssue {
    NewIssue {
        title: title.into(),
        ..NewIssue::default()
    }
}

async fn changes(db: &Db, id: IssueId) -> Result<Vec<taskapp_core::Change>, Error> {
    history::list_for(db, "issue", &id.0.to_string()).await
}

#[tokio::test]
async fn create_assigns_prefixed_sequential_keys() -> R {
    let db = Db::open_in_memory().await?;
    let a = issues::create(&db, new("a"), ChangeSource::Ui).await?;
    let b = issues::create(&db, new("b"), ChangeSource::Ui).await?;
    assert_eq!((a.key.as_str(), b.key.as_str()), ("ABC-1", "ABC-2"));
    assert_eq!(issues::get_by_key(&db, "ABC-2").await?.id, b.id);
    Ok(())
}

#[tokio::test]
async fn create_respects_migration_timestamps() -> R {
    let db = Db::open_in_memory().await?;
    let created = Utc.with_ymd_and_hms(2024, 5, 1, 10, 0, 0).unwrap();
    let done = Utc.with_ymd_and_hms(2024, 6, 1, 10, 0, 0).unwrap();
    let i = issues::create(
        &db,
        NewIssue {
            created_at: Some(created),
            completed_at: Some(done),
            ..new("old")
        },
        ChangeSource::Migration,
    )
    .await?;
    let got = issues::get(&db, i.id).await?;
    assert_eq!(got.created_at, created);
    assert_eq!(got.completed_at, Some(done));
    Ok(())
}

#[tokio::test]
async fn update_writes_one_change_per_field() -> R {
    let db = Db::open_in_memory().await?;
    let i = issues::create(&db, new("a"), ChangeSource::Ui).await?;
    let before = changes(&db, i.id).await?.len();
    let patch = IssuePatch {
        title: Some("b".into()),
        priority: Some(2),
        ..IssuePatch::default()
    };
    issues::update(&db, i.id, patch, ChangeSource::Agent).await?;
    let rows = changes(&db, i.id).await?;
    let new_rows = &rows[before..];
    assert_eq!(new_rows.len(), 2);
    assert!(new_rows.iter().all(|c| c.source == ChangeSource::Agent));
    assert_eq!(new_rows[0].field, "title");
    assert_eq!(new_rows[1].field, "priority");
    Ok(())
}

#[tokio::test]
async fn update_of_missing_id_is_not_found_and_writes_no_history() -> R {
    let db = Db::open_in_memory().await?;
    let patch = IssuePatch {
        title: Some("x".into()),
        ..IssuePatch::default()
    };
    let err = issues::update(&db, IssueId(99), patch, ChangeSource::Ui).await;
    assert!(matches!(err, Err(Error::NotFound(_))));
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM changes")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(n, 0);
    Ok(())
}

#[tokio::test]
async fn patch_that_changes_nothing_writes_no_change_rows() -> R {
    let db = Db::open_in_memory().await?;
    let i = issues::create(&db, new("a"), ChangeSource::Ui).await?;
    let before = changes(&db, i.id).await?.len();
    let patch = IssuePatch {
        title: Some("a".into()),
        priority: Some(0),
        ..IssuePatch::default()
    };
    let same = issues::update(&db, i.id, patch, ChangeSource::Ui).await?;
    assert_eq!(same, i);
    assert_eq!(changes(&db, i.id).await?.len(), before);
    Ok(())
}

#[tokio::test]
async fn completing_sets_completed_at_and_reopening_clears_it() -> R {
    let db = Db::open_in_memory().await?;
    let i = issues::create(&db, new("a"), ChangeSource::Ui).await?;
    assert_eq!(i.completed_at, None);
    let to = |id| IssuePatch {
        status_id: Some(StatusId(id)),
        ..IssuePatch::default()
    };
    // Seeded ids: 3 In Progress, 4 Done, 5 Canceled.
    let done = issues::update(&db, i.id, to(4), ChangeSource::Ui).await?;
    assert!(done.completed_at.is_some());
    assert!(issues::get(&db, i.id).await?.completed_at.is_some());
    let open = issues::update(&db, i.id, to(3), ChangeSource::Ui).await?;
    assert_eq!(open.completed_at, None);
    let canceled = issues::update(&db, i.id, to(5), ChangeSource::Ui).await?;
    assert!(canceled.completed_at.is_some());
    Ok(())
}

#[tokio::test]
async fn due_datetime_survives_dst() -> R {
    let db = Db::open_in_memory().await?;
    // Entered on 2026-03-01 (CET) for 2026-04-02 09:00, after the 03-29 spring-forward.
    let utc = Berlin
        .with_ymd_and_hms(2026, 4, 2, 9, 0, 0)
        .unwrap()
        .with_timezone(&Utc);
    let due = Due::DateTime {
        utc,
        tz: "Europe/Berlin".into(),
    };
    let i = issues::create(
        &db,
        NewIssue {
            due: Some(due),
            ..new("a")
        },
        ChangeSource::Ui,
    )
    .await?;
    let Some(Due::DateTime { utc, tz }) = issues::get(&db, i.id).await?.due else {
        panic!("due lost");
    };
    assert_eq!(tz, "Europe/Berlin");
    assert_eq!(utc, DateTime::parse_from_rfc3339("2026-04-02T07:00:00Z")?);
    assert_eq!(
        utc.with_timezone(&Berlin).format("%H:%M").to_string(),
        "09:00"
    );
    Ok(())
}

#[tokio::test]
async fn date_only_due_never_shifts() -> R {
    let db = Db::open_in_memory().await?;
    let d = Due::Date(NaiveDate::from_ymd_opt(2026, 11, 30).expect("valid date"));
    let i = issues::create(
        &db,
        NewIssue {
            due: Some(d.clone()),
            ..new("a")
        },
        ChangeSource::Ui,
    )
    .await?;
    assert_eq!(issues::get(&db, i.id).await?.due, Some(d));
    let cleared = IssuePatch {
        due: Some(None),
        ..IssuePatch::default()
    };
    let got = issues::update(&db, i.id, cleared, ChangeSource::Ui).await?;
    assert_eq!(got.due, None);
    Ok(())
}

#[tokio::test]
async fn archive_hides_from_children_and_restore_returns_it() -> R {
    let db = Db::open_in_memory().await?;
    let p = issues::create(&db, new("parent"), ChangeSource::Ui).await?;
    let c = issues::create(
        &db,
        NewIssue {
            parent_id: Some(p.id),
            ..new("child")
        },
        ChangeSource::Ui,
    )
    .await?;
    assert_eq!(issues::children(&db, p.id).await?.len(), 1);
    issues::archive(&db, c.id, ChangeSource::Ui).await?;
    assert_eq!(issues::children(&db, p.id).await?.len(), 0);
    assert!(issues::get(&db, c.id).await?.archived_at.is_some());
    issues::restore(&db, c.id, ChangeSource::Ui).await?;
    assert_eq!(issues::children(&db, p.id).await?.len(), 1);
    Ok(())
}

#[tokio::test]
async fn get_by_key_unknown_is_not_found() -> R {
    let db = Db::open_in_memory().await?;
    for key in ["ABC-1", "nonsense", "XX-1", "ABC-x"] {
        let got = issues::get_by_key(&db, key).await;
        assert!(matches!(got, Err(Error::NotFound(_))), "{key}");
    }
    Ok(())
}

#[tokio::test]
async fn delete_status_moves_issues() -> R {
    let db = Db::open_in_memory().await?;
    let s = statuses::create(
        &db,
        "Review",
        StatusCategory::Started,
        "#112233",
        None,
        ChangeSource::Ui,
    )
    .await?;
    let i = issues::create(
        &db,
        NewIssue {
            status_id: Some(s.id),
            ..new("a")
        },
        ChangeSource::Ui,
    )
    .await?;
    statuses::delete(&db, s.id, StatusId(2)).await?;
    assert_eq!(issues::get(&db, i.id).await?.status_id, StatusId(2));
    assert!(statuses::list(&db).await?.iter().all(|x| x.id != s.id));
    Ok(())
}

#[tokio::test]
async fn delete_status_into_itself_is_invalid() -> R {
    let db = Db::open_in_memory().await?;
    let err = statuses::delete(&db, StatusId(2), StatusId(2)).await;
    assert!(matches!(err, Err(Error::Invalid(_))));
    assert_eq!(statuses::list(&db).await?.len(), 5);
    Ok(())
}

#[tokio::test]
async fn concurrent_writers_do_not_error() -> R {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("t.db");
    let a = Db::open(&path).await?;
    let b = Db::open(&path).await?;
    let run = |db: Db| async move {
        for n in 0..50 {
            issues::create(&db, new(&format!("t{n}")), ChangeSource::Ui).await?;
        }
        Ok::<(), Error>(())
    };
    let (ra, rb) = tokio::join!(run(a.clone()), run(b));
    ra?;
    rb?;
    let (count, distinct): (i64, i64) =
        sqlx::query_as("SELECT count(*), count(DISTINCT number) FROM issues")
            .fetch_one(&a.pool)
            .await?;
    assert_eq!((count, distinct), (100, 100));
    Ok(())
}

#[tokio::test]
async fn create_and_update_keep_search_current() -> R {
    let db = Db::open_in_memory().await?;
    let i = issues::create(&db, new("giraffe"), ChangeSource::Ui).await?;
    assert_eq!(search::search(&db, "giraffe", 10).await?.len(), 1);
    let patch = IssuePatch {
        title: Some("okapi".into()),
        description: Some("savanna".into()),
        ..IssuePatch::default()
    };
    issues::update(&db, i.id, patch, ChangeSource::Ui).await?;
    assert_eq!(search::search(&db, "giraffe", 10).await?.len(), 0);
    assert_eq!(search::search(&db, "okapi", 10).await?.len(), 1);
    assert_eq!(search::search(&db, "savanna", 10).await?.len(), 1);
    Ok(())
}

#[tokio::test]
async fn archived_issue_is_not_found_after_reindex() -> R {
    let db = Db::open_in_memory().await?;
    let i = issues::create(&db, new("giraffe"), ChangeSource::Ui).await?;
    issues::archive(&db, i.id, ChangeSource::Ui).await?;
    search::reindex(&db, i.id).await?;
    assert_eq!(search::search(&db, "giraffe", 10).await?.len(), 0);
    issues::restore(&db, i.id, ChangeSource::Ui).await?;
    assert_eq!(search::search(&db, "giraffe", 10).await?.len(), 1);
    Ok(())
}

#[tokio::test]
async fn create_in_and_update_in_roll_back_with_the_caller() -> R {
    let db = Db::open_in_memory().await?;
    let keep = issues::create(&db, new("keep"), ChangeSource::Ui).await?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    issues::create_in(&mut tx, new("gone"), ChangeSource::Ui).await?;
    let patch = IssuePatch {
        title: Some("changed".into()),
        ..IssuePatch::default()
    };
    issues::update_in(&mut tx, keep.id, patch, ChangeSource::Ui).await?;
    tx.rollback().await?;
    assert_eq!(issues::get(&db, keep.id).await?.title, "keep");
    assert!(issues::get_by_key(&db, "ABC-2").await.is_err());
    Ok(())
}

#[tokio::test]
async fn update_merges_properties_and_validates_defined_keys() -> R {
    use serde_json::json;
    use taskapp_core::{PropertyDef, PropertyKind, properties};
    let db = Db::open_in_memory().await?;
    let num = PropertyDef {
        key: "size".into(),
        name: "Size".into(),
        kind: PropertyKind::Number,
        options: vec![],
        project_id: None,
        position: 0,
    };
    properties::create_def(&db, num, ChangeSource::Ui).await?;
    let i = issues::create(&db, new("a"), ChangeSource::Ui).await?;
    let props = |v: serde_json::Value| IssuePatch {
        properties: serde_json::from_value(v).ok(),
        ..IssuePatch::default()
    };
    issues::update(
        &db,
        i.id,
        props(json!({"size": 1, "linear_id": "L1"})),
        ChangeSource::Ui,
    )
    .await?;
    let got = issues::update(&db, i.id, props(json!({"size": 2})), ChangeSource::Ui).await?;
    assert_eq!(got.properties["linear_id"], json!("L1"));
    assert_eq!(got.properties["size"], json!(2));
    let got = issues::update(
        &db,
        i.id,
        props(json!({"linear_id": null})),
        ChangeSource::Ui,
    )
    .await?;
    assert!(!got.properties.contains_key("linear_id"));
    let bad = issues::update(&db, i.id, props(json!({"size": "big"})), ChangeSource::Ui).await;
    assert!(matches!(bad, Err(Error::Invalid(_))), "{bad:?}");
    Ok(())
}

#[tokio::test]
async fn parent_cycles_are_rejected() -> R {
    let db = Db::open_in_memory().await?;
    let a = issues::create(&db, new("a"), ChangeSource::Ui).await?;
    let b = issues::create(&db, new("b"), ChangeSource::Ui).await?;
    let c = issues::create(&db, new("c"), ChangeSource::Ui).await?;
    let parent = |p: IssueId| IssuePatch {
        parent_id: Some(Some(p)),
        ..IssuePatch::default()
    };
    issues::update(&db, a.id, parent(b.id), ChangeSource::Ui).await?;
    issues::update(&db, b.id, parent(c.id), ChangeSource::Ui).await?;
    let bad = issues::update(&db, c.id, parent(a.id), ChangeSource::Ui).await;
    assert!(matches!(bad, Err(Error::Invalid(_))), "{bad:?}");
    Ok(())
}

#[tokio::test]
async fn key_resolves_by_number_under_any_prefix() -> R {
    let db = Db::open_in_memory().await?;
    let a = issues::create(&db, new("a"), ChangeSource::Ui).await?;
    assert_eq!(issues::get_by_key(&db, "XY-1").await?.id, a.id);
    taskapp_core::settings::set(&db, "issue_prefix", &"XY").await?;
    assert_eq!(issues::get_by_key(&db, "ABC-1").await?.id, a.id);
    assert!(matches!(
        issues::get_by_key(&db, "ab-1").await,
        Err(Error::NotFound(_))
    ));
    Ok(())
}
