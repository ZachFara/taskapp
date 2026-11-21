use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

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

/// Replaces `to` with the backup `from`, but only after `from` passes `PRAGMA integrity_check`.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
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
    let mut staging_name = to.as_os_str().to_owned();
    staging_name.push(".restoring");
    let staging = PathBuf::from(staging_name);
    fs::copy(from, &staging)?;
    // A leftover WAL or shm from the old database must not be replayed onto the restored file.
    for suffix in ["-wal", "-shm"] {
        let mut sidecar = to.as_os_str().to_owned();
        sidecar.push(suffix);
        let sidecar = PathBuf::from(sidecar);
        if sidecar.exists() {
            fs::remove_file(sidecar)?;
        }
    }
    fs::rename(&staging, to)?;
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

fn prune_folder(folder: &Path, keep: usize) -> Result<Vec<PathBuf>> {
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
    let cut = files.len().saturating_sub(keep);
    let old: Vec<PathBuf> = files.drain(..cut).collect();
    for path in &old {
        fs::remove_file(path)?;
    }
    Ok(old)
}

/// Opens the backup read-only and fails unless SQLite reports it intact.
async fn check_backup(from: &Path) -> Result<()> {
    let opts = SqliteConnectOptions::new().filename(from).read_only(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await?;
    let verdict: std::result::Result<String, sqlx::Error> =
        sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&pool)
            .await;
    pool.close().await;
    let report = verdict?;
    if report == "ok" {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "backup failed integrity check: {report}"
        )))
    }
}
