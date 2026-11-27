use chrono::{DateTime, NaiveDate, Utc};
use taskapp_core::{
    ChangeSource, Db, Error, IssueId, ProjectId, blocks, comments, cycles, history, labels,
    projects, properties, search,
};

type R = Result<(), Box<dyn std::error::Error>>;

fn date(s: &str) -> NaiveDate {
    s.parse().expect("date")
}

fn time(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .expect("rfc3339")
        .with_timezone(&Utc)
}

/// Inserts an issue row directly: issue creation belongs to T2, which may not be merged yet.
async fn issue(
    db: &Db,
    project: Option<ProjectId>,
    status: i64,
    archived: bool,
) -> Result<IssueId, sqlx::Error> {
    let now = Utc::now();
    let number: i64 = sqlx::query_scalar("SELECT coalesce(max(number), 0) + 1 FROM issues")
        .fetch_one(&db.pool)
        .await?;
    let id = sqlx::query(
        "INSERT INTO issues (number, title, status_id, project_id, created_at, updated_at, archived_at)
         VALUES (?, 'task', ?, ?, ?, ?, ?)",
    )
    .bind(number)
    .bind(status)
    .bind(project.map(|p| p.0))
    .bind(now)
    .bind(now)
    .bind(archived.then_some(now))
    .execute(&db.pool)
    .await?
    .last_insert_rowid();
    Ok(IssueId(id))
}

#[tokio::test]
async fn project_without_description_or_link() -> R {
    let db = Db::open_in_memory().await?;
    let p = projects::create(&db, "alpha", "Alpha", None, None, ChangeSource::Ui).await?;
    assert_eq!(p.description, None);
    assert_eq!(p.doc_link, None);
    let all = projects::list(&db).await?;
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].slug, "alpha");
    Ok(())
}

#[tokio::test]
async fn doc_link_validation() -> R {
    let db = Db::open_in_memory().await?;
    let p = projects::create(&db, "alpha", "Alpha", None, None, ChangeSource::Ui).await?;
    for ok in [
        "obsidian://open?file=plan",
        "https://example.com/spec",
        "notes/plan.md",
    ] {
        let mut q = p.clone();
        q.doc_link = Some(ok.into());
        projects::update(&db, q, ChangeSource::Ui).await?;
    }
    for bad in [
        "javascript:alert(1)",
        "file:///etc/passwd",
        "http://example.com/spec",
        "notes/plan.txt",
        "/abs/plan.md",
        "../plan.md",
        "notes/../../plan.md",
    ] {
        let mut q = p.clone();
        q.doc_link = Some(bad.into());
        let err = projects::update(&db, q, ChangeSource::Ui).await;
        assert!(matches!(err, Err(Error::Invalid(_))), "accepted {bad}");
    }
    Ok(())
}

#[tokio::test]
async fn project_slug_validation() -> R {
    let db = Db::open_in_memory().await?;
    let long = "a".repeat(40);
    for ok in ["a", "abc-123", "0x", long.as_str()] {
        projects::create(&db, ok, "Name", None, None, ChangeSource::Ui).await?;
    }
    let too_long = "a".repeat(41);
    for bad in [
        "",
        "-abc",
        "Abc",
        "a_b",
        "a b",
        "abc/def",
        too_long.as_str(),
    ] {
        let err = projects::create(&db, bad, "Name", None, None, ChangeSource::Ui).await;
        assert!(matches!(err, Err(Error::Invalid(_))), "accepted {bad:?}");
    }
    Ok(())
}

#[tokio::test]
async fn project_progress_counts_completed() -> R {
    let db = Db::open_in_memory().await?;
    let p = projects::create(&db, "alpha", "Alpha", None, None, ChangeSource::Ui).await?;
    let other = projects::create(&db, "beta", "Beta", None, None, ChangeSource::Ui).await?;
    issue(&db, Some(p.id), 1, false).await?; // Backlog
    issue(&db, Some(p.id), 1, false).await?; // Backlog
    issue(&db, Some(p.id), 4, false).await?; // Done
    issue(&db, Some(p.id), 5, false).await?; // Canceled: counted, not completed
    issue(&db, Some(p.id), 4, true).await?; // archived: not counted
    issue(&db, Some(other.id), 4, false).await?; // other project
    assert_eq!(projects::progress(&db, p.id).await?, (1, 4));
    Ok(())
}

#[tokio::test]
async fn project_delete_unassigns_issues() -> R {
    let db = Db::open_in_memory().await?;
    let p = projects::create(&db, "alpha", "Alpha", None, None, ChangeSource::Ui).await?;
    let m = projects::create_milestone(&db, p.id, "v1", None, ChangeSource::Ui).await?;
    let i = issue(&db, Some(p.id), 1, false).await?;
    sqlx::query("UPDATE issues SET milestone_id = ? WHERE id = ?")
        .bind(m.id.0)
        .bind(i.0)
        .execute(&db.pool)
        .await?;
    projects::delete(&db, p.id, ChangeSource::Ui).await?;
    assert_eq!(projects::list(&db).await?.len(), 0);
    let (project, milestone): (Option<i64>, Option<i64>) =
        sqlx::query_as("SELECT project_id, milestone_id FROM issues WHERE id = ?")
            .bind(i.0)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!((project, milestone), (None, None));
    Ok(())
}

#[tokio::test]
async fn overlapping_cycles_rejected() -> R {
    let db = Db::open_in_memory().await?;
    let first = cycles::create(
        &db,
        None,
        date("2026-01-01"),
        date("2026-01-14"),
        taskapp_core::palette::Look::default(),
        ChangeSource::Ui,
    )
    .await?;
    assert_eq!(first.number, 1);
    // Touching on the same day counts as overlap.
    let touch = cycles::create(
        &db,
        None,
        date("2026-01-14"),
        date("2026-01-20"),
        taskapp_core::palette::Look::default(),
        ChangeSource::Ui,
    )
    .await;
    assert!(matches!(touch, Err(Error::Conflict(_))), "same-day touch");
    let inside = cycles::create(
        &db,
        None,
        date("2026-01-05"),
        date("2026-01-06"),
        taskapp_core::palette::Look::default(),
        ChangeSource::Ui,
    )
    .await;
    assert!(matches!(inside, Err(Error::Conflict(_))), "contained");
    let end_before_start = cycles::create(
        &db,
        None,
        date("2026-01-20"),
        date("2026-01-19"),
        taskapp_core::palette::Look::default(),
        ChangeSource::Ui,
    )
    .await;
    assert!(matches!(end_before_start, Err(Error::Invalid(_))));
    let next = cycles::create(
        &db,
        Some("next".into()),
        date("2026-01-15"),
        date("2026-01-28"),
        taskapp_core::palette::Look::default(),
        ChangeSource::Ui,
    )
    .await?;
    assert_eq!(next.number, 2);
    // Moving the second cycle onto the first is rejected, and moving it onto itself is not.
    let mut moved = next.clone();
    moved.start_date = date("2026-01-10");
    let err = cycles::update(&db, moved.clone(), ChangeSource::Ui).await;
    assert!(matches!(err, Err(Error::Conflict(_))), "update overlap");
    moved.start_date = date("2026-01-16");
    cycles::update(&db, moved, ChangeSource::Ui).await?;
    assert_eq!(cycles::list(&db).await?.len(), 2);
    Ok(())
}

#[tokio::test]
async fn label_add_remove_records_history() -> R {
    let db = Db::open_in_memory().await?;
    let label = labels::create(&db, "bug", "#ff0000", None, ChangeSource::Ui).await?;
    let i = issue(&db, None, 1, false).await?;
    let entity_id = i.0.to_string();
    labels::add_label(&db, i, label.id, ChangeSource::Agent).await?;
    labels::add_label(&db, i, label.id, ChangeSource::Agent).await?; // idempotent
    let count = |sql: &'static str| {
        let pool = db.pool.clone();
        async move {
            sqlx::query_scalar::<_, i64>(sql)
                .bind(i.0)
                .fetch_one(&pool)
                .await
        }
    };
    assert_eq!(
        count("SELECT count(*) FROM issue_labels WHERE issue_id = ?").await?,
        1
    );
    let added = history::list_for(&db, "issue", &entity_id).await?;
    assert_eq!(added.len(), 1);
    assert_eq!(added[0].source, ChangeSource::Agent);
    assert_eq!(added[0].new_value, Some(serde_json::json!(label.id.0)));

    labels::remove_label(&db, i, label.id, ChangeSource::Ui).await?;
    assert_eq!(
        count("SELECT count(*) FROM issue_labels WHERE issue_id = ?").await?,
        0
    );
    let all = history::list_for(&db, "issue", &entity_id).await?;
    assert_eq!(all.len(), 2);
    assert_eq!(all[1].old_value, Some(serde_json::json!(label.id.0)));
    assert_eq!(all[1].new_value, None);
    Ok(())
}

#[tokio::test]
async fn comment_accepts_migration_timestamp() -> R {
    let db = Db::open_in_memory().await?;
    let i = issue(&db, None, 1, false).await?;
    let ts = time("2020-01-02T03:04:05Z");
    let old = comments::add_comment(&db, i, "imported", Some(ts), ChangeSource::Migration).await?;
    assert_eq!(old.created_at, ts);
    let now = comments::add_comment(&db, i, "new", None, ChangeSource::Ui).await?;
    assert!(now.created_at > ts);
    let listed = comments::list(&db, i).await?;
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].body, "imported");
    let edited = comments::update(&db, old.id, "edited", ChangeSource::Ui).await?;
    assert_eq!(edited.body, "edited");
    assert_eq!(edited.created_at, ts);
    comments::delete(&db, now.id, ChangeSource::Ui).await?;
    assert_eq!(comments::list(&db, i).await?.len(), 1);
    let log = history::list_for(&db, "comment", &old.id.0.to_string()).await?;
    assert_eq!(log.len(), 2);
    Ok(())
}

#[tokio::test]
async fn work_block_end_before_start_rejected() -> R {
    let db = Db::open_in_memory().await?;
    let i = issue(&db, None, 1, false).await?;
    let t = time("2026-03-10T09:00:00Z");
    let same = blocks::create_block(&db, i, t, t).await;
    assert!(matches!(same, Err(Error::Invalid(_))), "end == start");
    let before = blocks::create_block(&db, i, t, t - chrono::Duration::minutes(1)).await;
    assert!(matches!(before, Err(Error::Invalid(_))), "end < start");
    let ok = blocks::create_block(&db, i, t, t + chrono::Duration::hours(1)).await?;
    let moved = blocks::move_block(&db, ok.id, t, t).await;
    assert!(
        matches!(moved, Err(Error::Invalid(_))),
        "move to zero length"
    );
    Ok(())
}

#[tokio::test]
async fn blocks_between_returns_overlapping_only() -> R {
    let db = Db::open_in_memory().await?;
    let i = issue(&db, None, 1, false).await?;
    let at = |s: &str| time(&format!("2026-03-10T{s}Z"));
    let before = blocks::create_block(&db, i, at("08:00:00"), at("08:59:59.500")).await?;
    let spans_start = blocks::create_block(&db, i, at("08:30:00"), at("09:30:00")).await?;
    let inside = blocks::create_block(&db, i, at("10:00:00"), at("11:00:00")).await?;
    let spans_end = blocks::create_block(&db, i, at("11:30:00"), at("12:30:00")).await?;
    let touches_end = blocks::create_block(&db, i, at("12:00:00"), at("13:00:00")).await?;
    // Ends 200 ms after the window opens: mixed fractional precision must still compare correctly.
    let just_inside = blocks::create_block(&db, i, at("08:00:00"), at("09:00:00.200")).await?;
    let found = blocks::blocks_between(&db, at("09:00:00"), at("12:00:00")).await?;
    let ids: Vec<_> = found.iter().map(|b| b.id).collect();
    assert_eq!(ids.len(), 4, "got {ids:?}");
    for wanted in [&spans_start, &inside, &spans_end, &just_inside] {
        assert!(ids.contains(&wanted.id), "missing {:?}", wanted.id);
    }
    assert!(!ids.contains(&before.id));
    assert!(!ids.contains(&touches_end.id));
    Ok(())
}

#[tokio::test]
async fn project_delete_refuses_while_scoped_defs_exist() -> R {
    let db = Db::open_in_memory().await?;
    let p = projects::create(&db, "alpha", "Alpha", None, None, ChangeSource::Ui).await?;
    let def = taskapp_core::PropertyDef {
        key: "area".into(),
        name: "Area".into(),
        kind: taskapp_core::PropertyKind::Text,
        options: vec![],
        project_id: Some(p.id),
        position: 0,
        applies_to: taskapp_core::PropertyAppliesTo::Issues,
    };
    properties::create_def(&db, def, ChangeSource::Ui).await?;
    let i = issue(&db, Some(p.id), 1, false).await?;
    let e = projects::delete(&db, p.id, ChangeSource::Ui).await;
    let Err(Error::Conflict(m)) = e else {
        panic!("expected Conflict, got {e:?}")
    };
    assert!(m.contains("area"), "{m}");
    assert_eq!(projects::list(&db).await?.len(), 1);
    let project: Option<i64> = sqlx::query_scalar("SELECT project_id FROM issues WHERE id = ?")
        .bind(i.0)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(project, Some(p.id.0));
    let logged = history::list_for(&db, "issue", &i.0.to_string()).await?;
    assert_eq!(logged.len(), 0);
    Ok(())
}

#[tokio::test]
async fn comment_edits_reach_the_search_index() -> R {
    let db = Db::open_in_memory().await?;
    let i = issue(&db, None, 1, false).await?;
    let c = comments::add_comment(&db, i, "zebra crossing", None, ChangeSource::Ui).await?;
    assert_eq!(search::search(&db, "zebra", 10).await?.len(), 1);
    comments::update(&db, c.id, "giraffe crossing", ChangeSource::Ui).await?;
    assert_eq!(search::search(&db, "zebra", 10).await?.len(), 0);
    assert_eq!(search::search(&db, "giraffe", 10).await?.len(), 1);
    comments::delete(&db, c.id, ChangeSource::Ui).await?;
    assert_eq!(search::search(&db, "giraffe", 10).await?.len(), 0);
    Ok(())
}
