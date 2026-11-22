use taskapp_core::{ChangeSource, Db, NewIssue, issues, workspace};

type R = Result<(), Box<dyn std::error::Error>>;

#[test]
fn key_must_be_two_to_five_capital_letters() {
    for ok in ["AB", "ABC", "ABCDE"] {
        assert!(workspace::validate_key(ok).is_ok(), "{ok} should be valid");
    }
    for bad in ["", "A", "ABCDEF", "ab", "A1", "A-B", "ÄB"] {
        assert!(
            workspace::validate_key(bad).is_err(),
            "{bad} should be rejected"
        );
    }
}

#[tokio::test]
async fn fresh_database_defaults_to_abc_and_is_not_set_up() -> R {
    let db = Db::open_in_memory().await?;
    let s = workspace::status(&db).await?;
    assert_eq!(
        (s.setup_done, s.name, s.key),
        (false, None, "ABC".to_string())
    );
    assert_eq!(s.timezone, workspace::system_tz());
    let issue = issues::create(
        &db,
        NewIssue {
            title: "First".into(),
            ..Default::default()
        },
        ChangeSource::Ui,
    )
    .await?;
    assert!(issue.key.starts_with("ABC-"), "key was {}", issue.key);
    Ok(())
}

#[tokio::test]
async fn setup_round_trips_through_status() -> R {
    let db = Db::open_in_memory().await?;
    let s = workspace::setup(&db, "  Cairn ", "XY", Some("Europe/Berlin")).await?;
    assert!(s.setup_done);
    assert_eq!(s.name.as_deref(), Some("Cairn"));
    assert_eq!(s.key, "XY");
    assert_eq!(s.timezone, "Europe/Berlin");
    assert_eq!(workspace::status(&db).await?, s);
    Ok(())
}

#[tokio::test]
async fn setup_without_zone_keeps_the_stored_zone() -> R {
    let db = Db::open_in_memory().await?;
    workspace::setup(&db, "Cairn", "ABC", Some("Europe/Berlin")).await?;
    let s = workspace::setup(&db, "Cairn", "ABC", None).await?;
    assert_eq!(s.timezone, "Europe/Berlin");
    Ok(())
}

#[tokio::test]
async fn setup_rejects_bad_input_and_changes_nothing() -> R {
    let db = Db::open_in_memory().await?;
    let before = workspace::status(&db).await?;
    assert!(workspace::setup(&db, "Cairn", "A", None).await.is_err());
    assert!(workspace::setup(&db, "  ", "ABC", None).await.is_err());
    assert!(
        workspace::setup(&db, "Cairn", "ABC", Some("Mars/Base"))
            .await
            .is_err()
    );
    assert_eq!(workspace::status(&db).await?, before);
    Ok(())
}

#[tokio::test]
async fn changing_the_key_keeps_issue_numbers() -> R {
    let db = Db::open_in_memory().await?;
    let first = issues::create(
        &db,
        NewIssue {
            title: "First".into(),
            ..Default::default()
        },
        ChangeSource::Ui,
    )
    .await?;
    workspace::setup(&db, "Cairn", "XY", None).await?;
    let after = issues::get(&db, first.id).await?;
    assert_eq!(
        after.key,
        format!("XY-{}", first.key.trim_start_matches("ABC-"))
    );
    Ok(())
}

#[tokio::test]
async fn prefix_migration_only_moves_an_empty_default_database() -> R {
    let migration = include_str!("../migrations/0007_generic_prefix.sql");
    let db = Db::open_in_memory().await?;
    taskapp_core::settings::set(&db, "issue_prefix", &"ZF").await?;
    sqlx::raw_sql(migration).execute(&db.pool).await?;
    assert_eq!(workspace::status(&db).await?.key, "ABC");

    taskapp_core::settings::set(&db, "issue_prefix", &"ZF").await?;
    issues::create(
        &db,
        NewIssue {
            title: "old".into(),
            ..Default::default()
        },
        ChangeSource::Ui,
    )
    .await?;
    sqlx::raw_sql(migration).execute(&db.pool).await?;
    assert_eq!(workspace::status(&db).await?.key, "ZF");
    assert!(
        issues::get(&db, taskapp_core::IssueId(1))
            .await?
            .key
            .starts_with("ZF-")
    );
    Ok(())
}
