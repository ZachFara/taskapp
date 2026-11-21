use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

use chrono::{NaiveDate, TimeZone, Utc};
use taskapp_core::{
    ChangeSource, Db, Due, Issue, SyncState, SyncStatus, issues, quickadd, settings,
};
use taskapp_reminders::import::import;
use tempfile::TempDir;

type R = Result<(), Box<dyn std::error::Error>>;

const TZ: &str = "America/New_York";

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 21).expect("valid date")
}

/// A fake helper: prints `<subcommand>.out` and logs every call to `calls.log`.
struct Fake {
    dir: TempDir,
}

impl Fake {
    fn new() -> Self {
        let dir = TempDir::new().expect("tempdir");
        let script = "#!/bin/sh\nd=$(dirname \"$0\")\necho \"$@\" >> \"$d/calls.log\"\n\
            cat \"$d/$1.out\"\nexit $(cat \"$d/$1.code\" 2>/dev/null || echo 0)\n";
        let path = dir.path().join("helper");
        fs::write(&path, script).expect("write script");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
        Fake { dir }
    }
    fn path(&self) -> PathBuf {
        self.dir.path().join("helper")
    }
    fn say(&self, sub: &str, out: &str, code: u8) {
        fs::write(self.dir.path().join(format!("{sub}.out")), out).expect("write out");
        fs::write(
            self.dir.path().join(format!("{sub}.code")),
            code.to_string(),
        )
        .expect("write code");
    }
    fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.dir.path().join("calls.log"))
            .unwrap_or_default()
            .lines()
            .map(String::from)
            .collect()
    }
}

async fn db_with_list() -> Result<Db, Box<dyn std::error::Error>> {
    let db = Db::open_in_memory().await?;
    settings::set(&db, "reminders_list_id", &"L1").await?;
    Ok(db)
}

async fn count(db: &Db, sql: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(sql).fetch_one(&db.pool).await
}

async fn reminders_state(db: &Db) -> Result<SyncState, Box<dyn std::error::Error>> {
    let s: SyncStatus = settings::get(db, "sync_status")
        .await?
        .expect("status written");
    Ok(s.reminders)
}

async fn only_issue(db: &Db) -> Result<Issue, Box<dyn std::error::Error>> {
    Ok(issues::get(db, taskapp_core::IssueId(1)).await?)
}

#[tokio::test]
async fn imports_and_deletes() -> R {
    let db = db_with_list().await?;
    let fake = Fake::new();
    fake.say(
        "fetch",
        r#"[{"external_id":"a","title":"Buy milk","notes":"2 litres","due":null},
            {"external_id":"b","title":"Call Sam","notes":null,"due":null}]"#,
        0,
    );
    fake.say("delete", r#"{"deleted":["a","b"],"missing":[]}"#, 0);
    let r = import(&db, &fake.path(), today(), TZ).await?;
    assert_eq!(
        (r.imported, r.skipped_duplicates, r.failed.len()),
        (2, 0, 0)
    );
    assert_eq!(only_issue(&db).await?.description, "2 litres");
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) FROM import_log WHERE source_deleted = 1"
        )
        .await?,
        2
    );
    // One delete call, scoped to the configured list.
    let deletes: Vec<_> = fake
        .calls()
        .into_iter()
        .filter(|c| c.starts_with("delete"))
        .collect();
    assert_eq!(deletes, ["delete --list L1 a b"]);
    assert!(matches!(reminders_state(&db).await?, SyncState::Ok { .. }));
    Ok(())
}

#[tokio::test]
async fn reminder_due_overrides_parsed_due() -> R {
    let db = db_with_list().await?;
    let fake = Fake::new();
    fake.say(
        "fetch",
        r#"[{"external_id":"a","title":"Pay rent tomorrow","notes":null,"due":{"date":"2026-12-01"}},
            {"external_id":"b","title":"Dentist","notes":null,
             "due":{"datetime":"2026-12-02T15:00:00Z","tz":"America/New_York"}}]"#,
        0,
    );
    fake.say("delete", r#"{"deleted":["a","b"],"missing":[]}"#, 0);
    import(&db, &fake.path(), today(), TZ).await?;
    let a = only_issue(&db).await?;
    assert_eq!(
        a.due,
        Some(Due::Date(
            NaiveDate::from_ymd_opt(2026, 12, 1).expect("date")
        ))
    );
    let b = issues::get(&db, taskapp_core::IssueId(2)).await?;
    let utc = Utc
        .with_ymd_and_hms(2026, 12, 2, 15, 0, 0)
        .single()
        .expect("utc");
    assert_eq!(
        b.due,
        Some(Due::DateTime {
            utc,
            tz: "America/New_York".into()
        })
    );
    Ok(())
}

#[tokio::test]
async fn reimport_after_crash_is_idempotent() -> R {
    let db = db_with_list().await?;
    // The app died after the issue and log row were written but before the delete.
    let issue =
        quickadd::create_from(&db, "Buy milk", today(), TZ, ChangeSource::RemindersImport).await?;
    sqlx::query("INSERT INTO import_log (external_id, source, issue_id, imported_at) VALUES ('a','reminders',?,'2026-11-21T00:00:00Z')")
        .bind(issue.id.0)
        .execute(&db.pool)
        .await?;
    let fake = Fake::new();
    fake.say(
        "fetch",
        r#"[{"external_id":"a","title":"Buy milk","notes":null,"due":null}]"#,
        0,
    );
    fake.say("delete", r#"{"deleted":["a"],"missing":[]}"#, 0);
    let r = import(&db, &fake.path(), today(), TZ).await?;
    assert_eq!((r.imported, r.skipped_duplicates), (0, 1));
    assert_eq!(count(&db, "SELECT COUNT(*) FROM issues").await?, 1);
    assert!(fake.calls().contains(&"delete --list L1 a".to_string()));
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) FROM import_log WHERE source_deleted = 1"
        )
        .await?,
        1
    );
    Ok(())
}

#[tokio::test]
async fn no_list_configured_is_noop() -> R {
    let db = Db::open_in_memory().await?;
    let fake = Fake::new();
    let r = import(&db, &fake.path(), today(), TZ).await?;
    assert_eq!(
        (r.imported, r.skipped_duplicates, r.failed.len()),
        (0, 0, 0)
    );
    assert_eq!(fake.calls().len(), 0);
    Ok(())
}

#[tokio::test]
async fn helper_error_sets_status() -> R {
    let db = db_with_list().await?;
    let fake = Fake::new();
    fake.say("fetch", r#"{"error":"Reminders access denied"}"#, 1);
    assert!(import(&db, &fake.path(), today(), TZ).await.is_err());
    match reminders_state(&db).await? {
        SyncState::Error { message, .. } => assert!(message.contains("access denied")),
        other => panic!("expected Error, got {other:?}"),
    }
    Ok(())
}

#[tokio::test]
async fn whitespace_title_is_skipped_and_reported() -> R {
    let db = db_with_list().await?;
    let fake = Fake::new();
    fake.say(
        "fetch",
        r#"[{"external_id":"a","title":"  \t ","notes":null,"due":null},
            {"external_id":"b","title":"Real","notes":null,"due":null}]"#,
        0,
    );
    fake.say("delete", r#"{"deleted":["b"],"missing":[]}"#, 0);
    let r = import(&db, &fake.path(), today(), TZ).await?;
    assert_eq!(r.imported, 1);
    assert_eq!(r.failed.len(), 1);
    assert!(r.failed[0].contains('a'));
    assert_eq!(count(&db, "SELECT COUNT(*) FROM issues").await?, 1);
    // The blank reminder stays in Reminders.
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) FROM import_log WHERE external_id = 'a'"
        )
        .await?,
        0
    );
    assert!(fake.calls().contains(&"delete --list L1 b".to_string()));
    Ok(())
}

#[tokio::test]
async fn invalid_json_sets_error_and_imports_nothing() -> R {
    let db = db_with_list().await?;
    let fake = Fake::new();
    fake.say("fetch", "not json {", 0);
    assert!(import(&db, &fake.path(), today(), TZ).await.is_err());
    assert!(matches!(
        reminders_state(&db).await?,
        SyncState::Error { .. }
    ));
    assert_eq!(count(&db, "SELECT COUNT(*) FROM issues").await?, 0);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM import_log").await?, 0);
    Ok(())
}

#[tokio::test]
async fn missing_ids_still_marked_deleted() -> R {
    let db = db_with_list().await?;
    let fake = Fake::new();
    fake.say(
        "fetch",
        r#"[{"external_id":"a","title":"One","notes":null,"due":null},
            {"external_id":"b","title":"Two","notes":null,"due":null}]"#,
        0,
    );
    fake.say("delete", r#"{"deleted":["a"],"missing":["b"]}"#, 0);
    import(&db, &fake.path(), today(), TZ).await?;
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) FROM import_log WHERE source_deleted = 1"
        )
        .await?,
        2
    );
    Ok(())
}

#[tokio::test]
async fn failed_log_write_leaves_no_issue() -> R {
    let db = db_with_list().await?;
    sqlx::query("CREATE TRIGGER no_log BEFORE INSERT ON import_log BEGIN SELECT RAISE(ABORT, 'log down'); END")
        .execute(&db.pool)
        .await?;
    let fake = Fake::new();
    fake.say(
        "fetch",
        r#"[{"external_id":"a","title":"Buy milk","notes":null,"due":null}]"#,
        0,
    );
    assert!(import(&db, &fake.path(), today(), TZ).await.is_err());
    assert_eq!(count(&db, "SELECT COUNT(*) FROM issues").await?, 0);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM import_log").await?, 0);
    Ok(())
}

#[tokio::test]
async fn notes_and_due_land_with_the_issue() -> R {
    let db = db_with_list().await?;
    // Any later update to an issue fails: notes and due must not depend on one.
    sqlx::query("CREATE TRIGGER no_update BEFORE UPDATE ON issues BEGIN SELECT RAISE(ABORT, 'update down'); END")
        .execute(&db.pool)
        .await?;
    let fake = Fake::new();
    fake.say(
        "fetch",
        r#"[{"external_id":"a","title":"Pay rent","notes":"IMPORTANT NOTES","due":{"date":"2026-12-01"}}]"#,
        0,
    );
    fake.say("delete", r#"{"deleted":["a"],"missing":[]}"#, 0);
    let r = import(&db, &fake.path(), today(), TZ).await?;
    assert_eq!((r.imported, r.failed.len()), (1, 0));
    let a = only_issue(&db).await?;
    assert_eq!(a.description, "IMPORTANT NOTES");
    assert_eq!(
        a.due,
        Some(Due::Date(
            NaiveDate::from_ymd_opt(2026, 12, 1).expect("date")
        ))
    );
    Ok(())
}
