use taskapp_core::{Db, Error, PinId, PinKind, history, pins};

type R = Result<(), Box<dyn std::error::Error>>;

async fn add_issue(db: &Db, number: i64, title: &str, archived: bool) -> Result<(), sqlx::Error> {
    let archived_at = archived.then_some("2026-01-02T00:00:00Z");
    sqlx::query(
        "INSERT INTO issues (number, title, status_id, created_at, updated_at, archived_at)
         VALUES (?, ?, (SELECT id FROM statuses ORDER BY position LIMIT 1), '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', ?)",
    )
    .bind(number)
    .bind(title)
    .bind(archived_at)
    .execute(&db.pool)
    .await?;
    Ok(())
}

async fn first_view_id(db: &Db) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT id FROM views ORDER BY position LIMIT 1")
        .fetch_one(&db.pool)
        .await
}

#[tokio::test]
async fn pin_is_idempotent() -> R {
    let db = Db::open_in_memory().await?;
    let first = pins::pin(&db, PinKind::Url, "https://a.example", None).await?;
    let again = pins::pin(
        &db,
        PinKind::Url,
        "https://a.example",
        Some("ignored".into()),
    )
    .await?;
    assert_eq!(first.id, again.id);
    assert_eq!(again.title, None);
    let other = pins::pin(&db, PinKind::Url, "https://b.example", None).await?;
    assert!(other.position > first.position);
    assert_eq!(pins::list_resolved(&db).await?.len(), 2);
    Ok(())
}

#[tokio::test]
async fn resolved_labels_and_routes() -> R {
    let db = Db::open_in_memory().await?;
    add_issue(&db, 3, "Fix lease", false).await?;
    sqlx::query("INSERT INTO projects (slug, name, created_at) VALUES ('home', 'Home', '2026-01-01T00:00:00Z')")
        .execute(&db.pool)
        .await?;
    sqlx::query("INSERT INTO cycles (number, name, start_date, end_date) VALUES (2, NULL, '2026-01-05', '2026-01-18')")
        .execute(&db.pool)
        .await?;
    let view = first_view_id(&db).await?;
    let view_name: String = sqlx::query_scalar("SELECT name FROM views WHERE id = ?")
        .bind(view)
        .fetch_one(&db.pool)
        .await?;

    pins::pin(&db, PinKind::Issue, "ABC-3", None).await?;
    pins::pin(&db, PinKind::Project, "home", None).await?;
    pins::pin(&db, PinKind::View, &view.to_string(), None).await?;
    pins::pin(&db, PinKind::Cycle, "2", None).await?;
    pins::pin(&db, PinKind::Note, "obsidian://open?file=a", None).await?;
    pins::pin(&db, PinKind::Url, "https://x.example", Some("Docs".into())).await?;

    let resolved = pins::list_resolved(&db).await?;
    let got: Vec<(&str, &str, &str)> = resolved
        .iter()
        .map(|r| (r.label.as_str(), r.route.as_str(), r.pin.target.as_str()))
        .collect();
    assert_eq!(
        got,
        vec![
            ("Fix lease", "/issue/ABC-3", "ABC-3"),
            ("Home", "/project/home", "home"),
            (
                view_name.as_str(),
                &*format!("/view/{view}"),
                &*view.to_string()
            ),
            ("Cycle 2", "/cycles#2", "2"),
            (
                "obsidian://open?file=a",
                "obsidian://open?file=a",
                "obsidian://open?file=a"
            ),
            ("Docs", "https://x.example", "https://x.example"),
        ]
    );
    assert!(resolved.iter().all(|r| !r.archived));
    Ok(())
}

#[tokio::test]
async fn archived_issue_pin_kept_and_flagged() -> R {
    let db = Db::open_in_memory().await?;
    add_issue(&db, 4, "Old work", true).await?;
    add_issue(&db, 5, "Live work", false).await?;
    pins::pin(&db, PinKind::Issue, "ABC-4", None).await?;
    pins::pin(&db, PinKind::Issue, "ABC-5", None).await?;

    let resolved = pins::list_resolved(&db).await?;
    assert_eq!(resolved.len(), 2);
    assert_eq!(resolved[0].label, "Old work");
    assert!(resolved[0].archived);
    assert!(!resolved[1].archived);
    Ok(())
}

#[tokio::test]
async fn deleted_target_pin_removed() -> R {
    let db = Db::open_in_memory().await?;
    add_issue(&db, 9, "Soon gone", false).await?;
    pins::pin(&db, PinKind::Issue, "ABC-9", None).await?;
    pins::pin(&db, PinKind::Issue, "ABC-99", None).await?;
    sqlx::query("DELETE FROM issues WHERE number = 9")
        .execute(&db.pool)
        .await?;

    assert_eq!(pins::list_resolved(&db).await?.len(), 0);
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM pins")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(left, 0);
    Ok(())
}

#[tokio::test]
async fn reorder_requires_full_set() -> R {
    let db = Db::open_in_memory().await?;
    let a = pins::pin(&db, PinKind::Url, "https://a.example", None).await?;
    let b = pins::pin(&db, PinKind::Url, "https://b.example", None).await?;
    let c = pins::pin(&db, PinKind::Url, "https://c.example", None).await?;

    pins::reorder(&db, &[c.id, a.id, b.id]).await?;
    let order: Vec<PinId> = pins::list_resolved(&db)
        .await?
        .iter()
        .map(|r| r.pin.id)
        .collect();
    assert_eq!(order, vec![c.id, a.id, b.id]);

    assert!(matches!(
        pins::reorder(&db, &[a.id, b.id]).await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        pins::reorder(&db, &[a.id, b.id, c.id, PinId(999)]).await,
        Err(Error::Invalid(_))
    ));
    Ok(())
}

#[tokio::test]
async fn reorder_duplicate_id_invalid() -> R {
    let db = Db::open_in_memory().await?;
    let a = pins::pin(&db, PinKind::Url, "https://a.example", None).await?;
    let b = pins::pin(&db, PinKind::Url, "https://b.example", None).await?;
    let c = pins::pin(&db, PinKind::Url, "https://c.example", None).await?;

    // Same length as the current set, but `b` is missing and `a` is repeated.
    assert!(matches!(
        pins::reorder(&db, &[a.id, a.id, c.id]).await,
        Err(Error::Invalid(_))
    ));
    let order: Vec<PinId> = pins::list_resolved(&db)
        .await?
        .iter()
        .map(|r| r.pin.id)
        .collect();
    assert_eq!(order, vec![a.id, b.id, c.id]);
    Ok(())
}

#[tokio::test]
async fn url_scheme_must_be_http_or_obsidian() -> R {
    let db = Db::open_in_memory().await?;
    for bad in [
        "javascript:alert(1)",
        "file:///etc/passwd",
        "ftp://x.example",
        "x.example",
        "",
    ] {
        assert!(
            matches!(
                pins::pin(&db, PinKind::Url, bad, None).await,
                Err(Error::Invalid(_))
            ),
            "url {bad:?} should be Invalid"
        );
    }
    pins::pin(&db, PinKind::Url, "HTTPS://x.example", None).await?;
    pins::pin(&db, PinKind::Url, "obsidian://open?file=a", None).await?;
    assert_eq!(pins::list_resolved(&db).await?.len(), 2);
    Ok(())
}

#[tokio::test]
async fn note_must_be_obsidian_link() -> R {
    let db = Db::open_in_memory().await?;
    assert!(matches!(
        pins::pin(&db, PinKind::Note, "https://x.example/note", None).await,
        Err(Error::Invalid(_))
    ));
    Ok(())
}

#[tokio::test]
async fn unpin_missing_is_not_found() -> R {
    let db = Db::open_in_memory().await?;
    assert!(matches!(
        pins::unpin(&db, PinId(999)).await,
        Err(Error::NotFound(_))
    ));
    let a = pins::pin(&db, PinKind::Url, "https://a.example", None).await?;
    pins::unpin(&db, a.id).await?;
    assert_eq!(pins::list_resolved(&db).await?.len(), 0);
    Ok(())
}

#[tokio::test]
async fn pins_survive_project_slug_rename_and_prefix_change() -> R {
    use taskapp_core::{ChangeSource, projects, settings};
    let db = Db::open_in_memory().await?;
    let mut p = projects::create(&db, "home", "Home", None, None, ChangeSource::Ui).await?;
    add_issue(&db, 1, "Fix roof", false).await?;
    pins::pin(&db, PinKind::Project, "home", None).await?;
    pins::pin(&db, PinKind::Issue, "ABC-1", None).await?;
    p.slug = "house".into();
    projects::update(&db, p, ChangeSource::Ui).await?;
    settings::set(&db, "issue_prefix", &"AB").await?;
    let routes: Vec<_> = pins::list_resolved(&db)
        .await?
        .into_iter()
        .map(|p| p.route)
        .collect();
    assert_eq!(routes, ["/project/house", "/issue/AB-1"]);
    Ok(())
}

#[tokio::test]
async fn reorder_undo_restores_positions() -> R {
    let db = Db::open_in_memory().await?;
    let a = pins::pin(&db, PinKind::Url, "https://a.example", None).await?;
    let b = pins::pin(&db, PinKind::Url, "https://b.example", None).await?;
    history::step("Reorder", pins::reorder(&db, &[b.id, a.id])).await?;
    taskapp_core::undo::undo(&db).await?;
    let ids: Vec<_> = pins::list_resolved(&db)
        .await?
        .into_iter()
        .map(|r| r.pin.id)
        .collect();
    assert_eq!(ids, vec![a.id, b.id]);
    Ok(())
}

#[tokio::test]
async fn set_pins_and_unpins_many_targets_in_one_undo_step() -> R {
    let db = Db::open_in_memory().await?;
    let urls = [
        "https://a.example".to_owned(),
        "https://b.example".to_owned(),
    ];
    history::step("Pin", pins::set(&db, PinKind::Url, &urls, true)).await?;
    assert_eq!(pins::list_resolved(&db).await?.len(), 2);
    // Pinning again is a no-op, not a second pin.
    pins::set(&db, PinKind::Url, &urls, true).await?;
    assert_eq!(pins::list_resolved(&db).await?.len(), 2);

    history::step("Unpin", pins::set(&db, PinKind::Url, &urls, false)).await?;
    assert_eq!(pins::list_resolved(&db).await?.len(), 0);
    taskapp_core::undo::undo(&db).await?;
    assert_eq!(pins::list_resolved(&db).await?.len(), 2);
    taskapp_core::undo::undo(&db).await?;
    assert_eq!(pins::list_resolved(&db).await?.len(), 0);
    Ok(())
}

#[tokio::test]
async fn set_with_one_invalid_target_changes_nothing() -> R {
    let db = Db::open_in_memory().await?;
    let mixed = ["https://a.example".to_owned(), "ftp://nope".to_owned()];
    assert!(matches!(
        pins::set(&db, PinKind::Url, &mixed, true).await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(pins::list_resolved(&db).await?.len(), 0);
    Ok(())
}

#[tokio::test]
async fn a_pin_records_the_source_that_made_it() -> R {
    let db = Db::open_in_memory().await?;
    let p = pins::pin_as(
        &db,
        taskapp_core::ChangeSource::Agent,
        PinKind::Url,
        "https://a.example",
        None,
    )
    .await?;
    let log = history::list_for(&db, "row:pins", &p.id.0.to_string()).await?;
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].source, taskapp_core::ChangeSource::Agent);
    Ok(())
}
