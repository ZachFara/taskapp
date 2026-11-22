use serde_json::json;
use taskapp_core::{ChangeSource, Db, ViewDef, history, settings};

type R = Result<(), Box<dyn std::error::Error>>;

async fn count(db: &Db, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
        .fetch_one(&db.pool)
        .await
        .expect("count")
}

#[tokio::test]
async fn migrations_seed_statuses_and_prefix() -> R {
    let db = Db::open_in_memory().await?;
    assert_eq!(count(&db, "statuses").await, 5);
    assert_eq!(
        settings::get::<String>(&db, "issue_prefix").await?,
        Some("ABC".into())
    );
    // The migration created issue_fts, so this build of SQLite has FTS5.
    sqlx::query("INSERT INTO issue_fts (rowid, title) VALUES (1, 'hello')")
        .execute(&db.pool)
        .await?;
    let hit: i64 = sqlx::query_scalar("SELECT rowid FROM issue_fts WHERE issue_fts MATCH 'hello'")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(hit, 1);
    Ok(())
}
#[tokio::test]
async fn seeded_views_parse_as_view_defs() -> R {
    let db = Db::open_in_memory().await?;
    let defs: Vec<String> = sqlx::query_scalar("SELECT definition FROM views ORDER BY position")
        .fetch_all(&db.pool)
        .await?;
    assert_eq!(defs.len(), 3);
    for d in defs {
        serde_json::from_str::<ViewDef>(&d)?;
    }
    Ok(())
}
#[tokio::test]
async fn settings_roundtrip() -> R {
    let db = Db::open_in_memory().await?;
    settings::set(&db, "gcal_calendar_id", &"abc").await?;
    assert_eq!(
        settings::get::<String>(&db, "gcal_calendar_id").await?,
        Some("abc".into())
    );
    settings::set(&db, "gcal_calendar_id", &"def").await?;
    assert_eq!(
        settings::get::<String>(&db, "gcal_calendar_id").await?,
        Some("def".into())
    );
    Ok(())
}
#[tokio::test]
async fn settings_get_missing_key_is_none() -> R {
    let db = Db::open_in_memory().await?;
    assert_eq!(settings::get::<String>(&db, "nope").await?, None);
    Ok(())
}
#[tokio::test]
async fn history_record_and_list() -> R {
    let db = Db::open_in_memory().await?;
    let mut tx = db.pool.begin().await?;
    let edit = history::Edit {
        field: "title",
        old: Some(&json!("a")),
        new: Some(&json!("b")),
    };
    history::record(&mut tx, ChangeSource::Agent, "issue", "1", &edit).await?;
    tx.commit().await?;
    let list = history::list_for(&db, "issue", "1").await?;
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].source, ChangeSource::Agent);
    assert_eq!(list[0].old_value, Some(json!("a")));
    assert_eq!(list[0].new_value, Some(json!("b")));
    Ok(())
}
#[tokio::test]
async fn history_list_for_unknown_entity_is_empty() -> R {
    let db = Db::open_in_memory().await?;
    assert_eq!(history::list_for(&db, "issue", "999").await?, vec![]);
    Ok(())
}
#[tokio::test]
async fn open_sets_wal_and_busy_timeout() -> R {
    let dir = tempfile::tempdir()?;
    let db = Db::open(&dir.path().join("t.db")).await?;
    let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(&db.pool)
        .await?;
    let timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(mode, "wal");
    assert_eq!(timeout, 5000);
    Ok(())
}
