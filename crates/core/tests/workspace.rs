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

#[tokio::test]
async fn setup_is_one_undoable_step() -> R {
    use taskapp_core::{history, undo};
    let db = Db::open_in_memory().await?;
    let before = workspace::status(&db).await?;
    history::step(
        "workspace_setup",
        workspace::setup(&db, "Cairn", "XY", Some("Europe/Berlin")),
    )
    .await?;
    assert!(workspace::status(&db).await?.setup_done);
    undo::undo(&db).await?;
    assert_eq!(workspace::status(&db).await?, before);
    Ok(())
}

#[tokio::test]
async fn settings_from_the_ui_are_checked_like_their_commands() -> R {
    use serde_json::{Value, json};
    use taskapp_core::{Error, settings};
    let db = Db::open_in_memory().await?;
    let bad = [
        ("issue_prefix", json!("bad key!")),
        ("timezone", json!("Mars/Base")),
        ("theme", json!("neon")),
        ("week_start", json!("friday")),
        ("default_view_template", json!("four")),
        ("workspace_name", json!("   ")),
        ("made_up_key", json!(1)),
    ];
    for (key, value) in &bad {
        let e = settings::set_from_user(&db, key, value).await;
        assert!(matches!(e, Err(Error::Invalid(_))), "{key}={value}: {e:?}");
    }
    settings::set_from_user(&db, "theme", &json!("dark")).await?;
    settings::set_from_user(&db, "insights.view.3", &json!("{}")).await?;
    settings::set_from_user(&db, "default_view_template", &Value::Null).await?;
    Ok(())
}

#[test]
fn a_folder_typed_with_a_tilde_means_the_home_folder() {
    use std::path::PathBuf;
    use taskapp_core::settings::user_path;
    let home = PathBuf::from("/Users/someone");
    assert_eq!(
        user_path("~/Documents/Cairn", &home).expect("tilde"),
        home.join("Documents/Cairn")
    );
    assert_eq!(user_path("~", &home).expect("bare tilde"), home);
    assert_eq!(
        user_path(" /Volumes/disk/x ", &home).expect("absolute"),
        PathBuf::from("/Volumes/disk/x")
    );
    // A relative path would resolve against the app's working folder (the disk root when opened from Finder).
    let err = user_path("Documents/Cairn", &home)
        .expect_err("relative")
        .to_string();
    assert!(err.contains("full path"), "{err}");
    assert!(user_path("", &home).is_err());
    assert!(user_path("~other/x", &home).is_err());
}
