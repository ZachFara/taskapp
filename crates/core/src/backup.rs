use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};

use crate::{Db, Error, Result};

const SNAPSHOTS_KEPT: usize = 30;
const NIGHTLY_KEPT: usize = 14;

/// Writes a consistent copy of the database to `<dir>/snapshots/`. The reason is slugged into the file name.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn snapshot(db: &Db, dir: &Path, reason: &str) -> Result<PathBuf> {
    let folder = dir.join("snapshots");
    fs::create_dir_all(&folder)?;
    // Fixed-width UTC timestamp, so file names sort chronologically for pruning.
    let name = format!(
        "{}-{}.db",
        Utc::now().format("%Y%m%dT%H%M%S%9fZ"),
        slug(reason)
    );
    let path = folder.join(name);
    vacuum_into(db, &path).await?;
    Ok(path)
}

/// Writes `<dir>/nightly/<UTC date>.db` unless that day's file already exists.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn nightly(db: &Db, dir: &Path, now: DateTime<Utc>) -> Result<Option<PathBuf>> {
    let folder = dir.join("nightly");
    let day = now.format("%Y-%m-%d");
    let path = folder.join(format!("{day}.db"));
    if path.exists() {
        return Ok(None);
    }
    fs::create_dir_all(&folder)?;
    // Written under another name and renamed, so a crash never leaves a partial file that looks finished.
    let partial = folder.join(format!("{day}.db.partial"));
    if partial.exists() {
        fs::remove_file(&partial)?;
    }
    vacuum_into(db, &partial).await?;
    fs::rename(&partial, &path)?;
    Ok(Some(path))
}

/// Keeps the newest 14 nightly and 30 snapshot files and deletes the rest. Returns the deleted paths.
///
/// # Errors
///
/// Fails when a file cannot be listed or deleted.
pub fn prune(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut deleted = prune_folder(&dir.join("nightly"), NIGHTLY_KEPT)?;
    deleted.extend(prune_folder(&dir.join("snapshots"), SNAPSHOTS_KEPT)?);
    Ok(deleted)
}

/// Every backup file under `dir`: snapshots newest first, then nightly files newest first.
///
/// # Errors
///
/// Fails when a folder cannot be listed.
pub fn list(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut all = db_files(&dir.join("snapshots"))?;
    all.extend(db_files(&dir.join("nightly"))?);
    Ok(all)
}

/// Replaces `to` with the backup `from`, but only after `from` passes `PRAGMA integrity_check` and is a taskapp
/// database. The replaced file is first copied to `<to>.pre-restore-<UTC timestamp>`.
///
/// # Errors
///
/// Fails with `Conflict` while another connection has `to` open, otherwise on a database error or
/// invalid arguments.
pub async fn restore(from: &Path, to: &Path) -> Result<()> {
    let dir = to
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if !dir.is_dir() {
        return Err(Error::Invalid(format!(
            "directory does not exist: {}",
            dir.display()
        )));
    }
    check_backup(from).await?;
    ensure_unused(to).await?;
    if to.exists() {
        // Kept under another name, not deleted, so a mistaken restore can be undone.
        let mut kept = to.as_os_str().to_owned();
        kept.push(format!(
            ".pre-restore-{}",
            Utc::now().format("%Y%m%dT%H%M%S%9fZ")
        ));
        fs::copy(to, PathBuf::from(kept))?;
    }
    let mut staging_name = to.as_os_str().to_owned();
    staging_name.push(".restoring");
    let staging = PathBuf::from(staging_name);
    fs::copy(from, &staging)?;
    // A leftover WAL or shm from the old database must not be replayed onto the restored file.
    for suffix in ["-wal", "-shm"] {
        let mut sidecar = to.as_os_str().to_owned();
        sidecar.push(suffix);
        // SQLite may delete the sidecar between a check and the remove, so NotFound is not an error.
        match fs::remove_file(PathBuf::from(sidecar)) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    fs::rename(&staging, to)?;
    Ok(())
}

/// A WAL database cannot leave WAL mode while any other connection has it open, so a refused switch
/// means someone else holds it.
async fn ensure_unused(to: &Path) -> Result<()> {
    // Retried: a pool's connections can finish closing just after `close()` returns.
    let mut result = try_unused(to).await;
    for _ in 0..9 {
        if !matches!(result, Err(Error::Conflict(_))) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        result = try_unused(to).await;
    }
    result
}

async fn try_unused(to: &Path) -> Result<()> {
    if !to.exists() {
        return Ok(());
    }
    let opts = SqliteConnectOptions::new().filename(to);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await?;
    let mode: std::result::Result<String, sqlx::Error> =
        sqlx::query_scalar("PRAGMA journal_mode = DELETE")
            .fetch_one(&pool)
            .await;
    pool.close().await;
    // SQLite reports the refused switch either as the unchanged mode or as SQLITE_BUSY (primary code 5).
    let held = match &mode {
        Ok(m) => !m.eq_ignore_ascii_case("delete"),
        Err(sqlx::Error::Database(e)) => e
            .code()
            .is_some_and(|c| c.parse::<i32>().is_ok_and(|n| n & 0xff == 5)),
        Err(_) => false,
    };
    if held {
        return Err(Error::Conflict(
            "the database is open elsewhere; close the app before restoring".into(),
        ));
    }
    mode?;
    Ok(())
}

async fn vacuum_into(db: &Db, path: &Path) -> Result<()> {
    let target = path
        .to_str()
        .ok_or_else(|| Error::Invalid(format!("path is not UTF-8: {}", path.display())))?;
    sqlx::query("VACUUM INTO ?")
        .bind(target)
        .execute(&db.pool)
        .await?;
    Ok(())
}

/// Lowercase ASCII letters and digits, other runs become one `-`. Never empty, never contains a separator.
fn slug(reason: &str) -> String {
    let mut out = String::new();
    for c in reason.chars().take(60) {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "snapshot".into()
    } else {
        trimmed.to_string()
    }
}

/// The `.db` files in `folder`, newest (by name) first; empty when the folder does not exist.
fn db_files(folder: &Path) -> Result<Vec<PathBuf>> {
    let mut files = prune_candidates(folder)?;
    files.reverse();
    Ok(files)
}

fn prune_candidates(folder: &Path) -> Result<Vec<PathBuf>> {
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let mut files = Vec::new();
    for entry in entries {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "db") && path.is_file() {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn prune_folder(folder: &Path, keep: usize) -> Result<Vec<PathBuf>> {
    let mut files = prune_candidates(folder)?;
    let cut = files.len().saturating_sub(keep);
    let old: Vec<PathBuf> = files.drain(..cut).collect();
    for path in &old {
        fs::remove_file(path)?;
    }
    Ok(old)
}

/// Opens the backup read-only and fails unless SQLite reports it intact and it is a taskapp database.
async fn check_backup(from: &Path) -> Result<()> {
    let opts = SqliteConnectOptions::new().filename(from).read_only(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await?;
    let verdict = check_pool(&pool, from).await;
    pool.close().await;
    verdict
}

async fn check_pool(pool: &SqlitePool, from: &Path) -> Result<()> {
    let report: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(pool)
        .await?;
    if report != "ok" {
        return Err(Error::Invalid(format!(
            "backup failed integrity check: {report}"
        )));
    }
    let tables: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN ('issues', 'settings')",
    )
    .fetch_one(pool)
    .await?;
    let prefix: i64 = if tables == 2 {
        sqlx::query_scalar("SELECT count(*) FROM settings WHERE key = 'issue_prefix'")
            .fetch_one(pool)
            .await?
    } else {
        0
    };
    if tables != 2 || prefix == 0 {
        return Err(Error::Invalid(format!(
            "not a taskapp database: {}",
            from.display()
        )));
    }
    Ok(())
}
