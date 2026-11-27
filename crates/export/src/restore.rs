//! Restore from an export folder or a Git URL into an open database.

use std::path::{Path, PathBuf};

use taskapp_core::{Db, Error, Result, backup, settings};

use crate::{git, rebuild};

/// Where a restore came from.
pub struct Restored {
    /// The folder the export was read from; the export settings now point at it.
    pub dir: PathBuf,
    /// The backup of what the restore replaced.
    pub backup: PathBuf,
}

/// Replaces the database's content with the export at `source`: a folder, or a Git URL cloned into
/// `clone_to` (which must not exist or be empty). Writes a backup of the current database into
/// `backup_dir` first. Afterwards `export_dir` (and `export_remote` for a URL) point at the
/// restored folder, so the next export continues the same history; `local` also applies the
/// per-machine settings the export carries. Nothing secret is read or stored.
///
/// # Errors
///
/// `Invalid` for a bad source or an export that does not parse (the database is then unchanged);
/// git's message when the clone fails.
pub async fn restore_from_export(
    db: &Db,
    source: &str,
    clone_to: &Path,
    backup_dir: &Path,
    local: bool,
) -> Result<Restored> {
    let source = source.trim();
    if git::is_url(source) {
        git::clone(source, clone_to).await?;
        let remote = crate::write::without_credentials(source);
        return restore_from_dir(db, clone_to, Some(&remote), backup_dir, local).await;
    }
    let dir = settings::home_path(source)?;
    restore_from_dir(db, &dir, None, backup_dir, local).await
}

/// [`restore_from_export`] for a folder that is already on disk; `remote` is the URL it came from.
///
/// # Errors
///
/// As [`restore_from_export`].
pub async fn restore_from_dir(
    db: &Db,
    dir: &Path,
    remote: Option<&str>,
    backup_dir: &Path,
    local: bool,
) -> Result<Restored> {
    if !dir.is_dir() {
        return Err(Error::Invalid(format!("{} is not a folder", dir.display())));
    }
    let backup = backup::snapshot(db, backup_dir, "before-restore").await?;
    rebuild::restore_into(db, dir).await?;
    if local {
        rebuild::apply_local(db, dir).await?;
    }
    // Restoring a workspace means it is set up, whatever an older export's settings said.
    settings::set(db, "setup_done", &true).await?;
    settings::set(db, "export_dir", &dir.display().to_string()).await?;
    if let Some(url) = remote {
        settings::set(db, "export_remote", &url).await?;
    }
    settings::set(db, "export_enabled", &true).await?;
    Ok(Restored {
        dir: dir.to_path_buf(),
        backup,
    })
}
