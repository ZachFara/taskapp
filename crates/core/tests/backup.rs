use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::{TimeDelta, TimeZone, Utc};
use taskapp_core::{Db, backup};
use tempfile::TempDir;

type R = Result<(), Box<dyn std::error::Error>>;

async fn add_issue(db: &Db, title: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO issues (number, title, status_id, created_at, updated_at)
         SELECT COALESCE(MAX(number), 0) + 1, ?, (SELECT MIN(id) FROM statuses),
                '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'
         FROM issues",
    )
    .bind(title)
    .execute(&db.pool)
    .await?;
    Ok(())
}

async fn issue_count(db: &Db) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT count(*) FROM issues")
        .fetch_one(&db.pool)
        .await
}

async fn integrity(db: &Db) -> Result<String, sqlx::Error> {
    sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&db.pool)
        .await
}

async fn open_with_three_issues(dir: &Path) -> Result<Db, Box<dyn std::error::Error>> {
    let db = Db::open(&dir.join("live.db")).await?;
    for title in ["a", "b", "c"] {
        add_issue(&db, title).await?;
    }
    Ok(db)
}

#[tokio::test]
async fn snapshot_is_openable_and_complete() -> R {
    let dir = TempDir::new()?;
    let db = open_with_three_issues(dir.path()).await?;
    let path = backup::snapshot(&db, dir.path(), "manual test").await?;
    assert_eq!(path.parent(), Some(dir.path().join("snapshots").as_path()));
    assert!(path.to_string_lossy().ends_with("-manual-test.db"));
    let copy = Db::open(&path).await?;
    assert_eq!(issue_count(&copy).await?, 3);
    assert_eq!(integrity(&copy).await?, "ok");
    Ok(())
}

#[tokio::test]
async fn nightly_once_per_day() -> R {
    let dir = TempDir::new()?;
    let db = open_with_three_issues(dir.path()).await?;
    let morning = Utc
        .with_ymd_and_hms(2026, 11, 21, 9, 0, 0)
        .single()
        .ok_or("date")?;
    let day_one = dir.path().join("nightly/2026-11-21.db");
    assert_eq!(
        backup::nightly(&db, dir.path(), morning).await?,
        Some(day_one.clone())
    );
    assert!(day_one.is_file());
    let later = morning + TimeDelta::hours(5);
    assert_eq!(backup::nightly(&db, dir.path(), later).await?, None);
    let next_day = morning + TimeDelta::days(1);
    let day_two = dir.path().join("nightly/2026-11-22.db");
    assert_eq!(
        backup::nightly(&db, dir.path(), next_day).await?,
        Some(day_two)
    );
    Ok(())
}

#[tokio::test]
async fn prune_keeps_14_and_30() -> R {
    let dir = TempDir::new()?;
    let nightly = dir.path().join("nightly");
    let snapshots = dir.path().join("snapshots");
    fs::create_dir_all(&nightly)?;
    fs::create_dir_all(&snapshots)?;
    for i in 1..=16 {
        fs::write(nightly.join(format!("2026-01-{i:02}.db")), b"")?;
    }
    for i in 0..32 {
        fs::write(snapshots.join(format!("20260101T000000{i:09}Z-s.db")), b"")?;
    }
    // A leftover partial nightly is not a backup and must survive pruning.
    fs::write(nightly.join("2026-01-01.db.partial"), b"")?;

    let deleted = backup::prune(dir.path())?;

    assert_eq!(deleted.len(), 2 + 2);
    assert!(deleted.contains(&nightly.join("2026-01-01.db")));
    assert!(deleted.contains(&nightly.join("2026-01-02.db")));
    assert!(deleted.contains(&snapshots.join(format!("20260101T000000{:09}Z-s.db", 0))));
    assert!(nightly.join("2026-01-01.db.partial").is_file());
    assert_eq!(fs::read_dir(&nightly)?.count(), 14 + 1);
    assert_eq!(fs::read_dir(&snapshots)?.count(), 30);
    Ok(())
}

#[tokio::test]
async fn prune_empty_dir_returns_empty() -> R {
    let dir = TempDir::new()?;
    assert_eq!(backup::prune(dir.path())?, Vec::<PathBuf>::new());
    fs::create_dir_all(dir.path().join("nightly"))?;
    assert_eq!(backup::prune(dir.path())?, Vec::<PathBuf>::new());
    Ok(())
}

#[tokio::test]
async fn restore_roundtrip() -> R {
    let dir = TempDir::new()?;
    let db = open_with_three_issues(dir.path()).await?;
    let snap = backup::snapshot(&db, dir.path(), "before").await?;
    add_issue(&db, "d").await?;
    assert_eq!(issue_count(&db).await?, 4);
    db.pool.close().await;

    backup::restore(&snap, &dir.path().join("live.db")).await?;

    let db = Db::open(&dir.path().join("live.db")).await?;
    assert_eq!(issue_count(&db).await?, 3);
    Ok(())
}

#[tokio::test]
async fn restore_rejects_corrupt_file() -> R {
    let dir = TempDir::new()?;
    let db = open_with_three_issues(dir.path()).await?;
    let snap = backup::snapshot(&db, dir.path(), "good").await?;
    db.pool.close().await;

    // Overwrite the middle of a table page so the header still looks like SQLite.
    let corrupt = dir.path().join("corrupt.db");
    fs::copy(&snap, &corrupt)?;
    let mut bytes = fs::read(&corrupt)?;
    let start = bytes.len() / 2;
    for byte in bytes.iter_mut().skip(start).take(512) {
        *byte = 0xFF;
    }
    fs::write(&corrupt, &bytes)?;

    let target = dir.path().join("live.db");
    let before = fs::read(&target)?;
    assert!(backup::restore(&corrupt, &target).await.is_err());
    assert_eq!(fs::read(&target)?, before);
    Ok(())
}

#[tokio::test]
async fn snapshot_reason_is_slugged_inside_snapshots() -> R {
    let dir = TempDir::new()?;
    let db = open_with_three_issues(dir.path()).await?;
    let path = backup::snapshot(&db, dir.path(), "../../etc/passwd / x").await?;
    assert_eq!(path.parent(), Some(dir.path().join("snapshots").as_path()));
    assert!(path.is_file());
    assert!(path.to_string_lossy().ends_with("-etc-passwd-x.db"));
    Ok(())
}

#[tokio::test]
async fn restore_missing_parent_touches_nothing() -> R {
    let dir = TempDir::new()?;
    let db = open_with_three_issues(dir.path()).await?;
    let snap = backup::snapshot(&db, dir.path(), "before").await?;
    db.pool.close().await;

    let to = dir.path().join("no-such-dir/live.db");
    assert!(backup::restore(&snap, &to).await.is_err());
    assert!(!to.exists());
    assert!(!dir.path().join("no-such-dir").exists());
    assert!(snap.is_file());
    Ok(())
}
