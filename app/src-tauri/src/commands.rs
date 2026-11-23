//! Tauri wrappers: one per command in `taskapp_commands`, generated from its list, plus the commands that
//! need a sync crate (the app is the only place allowed to use those together).

use std::sync::Arc;

use crate::sync_loop::{Real, Syncer};
use taskapp_commands::AppState;

use taskapp_core::{Db, Error, Result, SyncStatus, settings};
use tauri::State;
use tauri_specta::{Builder, collect_commands};

macro_rules! wrap {
    ($($name:ident($s:ident $(, $a:ident : $t:ty)*) -> $r:ty $body:block)*) => {
        mod wrappers {
            use chrono::{DateTime, NaiveDate, Utc};
            use serde_json::Value;
            use taskapp_commands::AppState;
            use taskapp_core::*;
            use tauri::State;

            $(
                #[tauri::command]
                #[specta::specta]
                #[doc = "Forwards to the command of the same name.\n\n# Errors\nAs that command."]
                pub async fn $name(state: State<'_, AppState> $(, $a: $t)*) -> Result<$r> {
                    taskapp_commands::$name(&state $(, $a)*).await
                }
            )*
        }
        use wrappers::*;

        /// Every command, ready to export bindings from and to build the invoke handler.
        pub fn builder() -> Builder<tauri::Wry> {
            Builder::new().commands(collect_commands![
                $($name,)*
                gcal_list_calendars, gcal_connect, reminders_list_lists, sync_now, export_now
            ])
        }
    };
}

taskapp_commands::with_commands!(wrap);

/// `(id, name)` of each calendar the connected account can write to.
///
/// # Errors
/// Fails when not connected or the request fails.
#[tauri::command]
#[specta::specta]
pub async fn gcal_list_calendars(state: State<'_, AppState>) -> Result<Vec<(String, String)>> {
    taskapp_gcal::client::list_calendars(&state.db).await
}

/// Starts the browser sign-in.
///
/// # Errors
/// Fails when sign-in is cancelled or the token cannot be stored.
#[tauri::command]
#[specta::specta]
pub async fn gcal_connect(state: State<'_, AppState>) -> Result<()> {
    taskapp_gcal::auth::connect(&state.db).await
}

/// `(id, name)` of each Reminders list, read through the bundled helper.
///
/// # Errors
/// Fails when the helper cannot run.
#[tauri::command]
#[specta::specta]
pub async fn reminders_list_lists(state: State<'_, AppState>) -> Result<Vec<(String, String)>> {
    taskapp_reminders::helper::list_reminder_lists(&state.helper_path).await
}

/// Runs every enabled sync now, after any sync already running, and returns the new status.
///
/// # Errors
/// Fails when the status cannot be read.
#[tauri::command]
#[specta::specta]
pub async fn sync_now(syncer: State<'_, Arc<Syncer<Real>>>) -> Result<SyncStatus> {
    syncer.sync_all().await
}

/// Writes the markdown export to the configured directory and commits it, pushing when a remote is set.
///
/// # Errors
/// Fails when no export directory is set, or when writing or git fails.
#[tauri::command]
#[specta::specta]
pub async fn export_now(state: State<'_, AppState>) -> Result<()> {
    export_snapshot(&state.db).await
}

async fn export_snapshot(db: &Db) -> Result<()> {
    let setting = |key: &'static str| async move {
        Ok::<_, Error>(
            settings::get::<String>(db, key)
                .await?
                .filter(|s| !s.is_empty()),
        )
    };
    let dir = setting("export_dir")
        .await?
        .ok_or_else(|| Error::Invalid("choose an export directory first".into()))?;
    let remote = setting("export_remote").await?;
    let dir = settings::home_path(&dir)?;
    taskapp_export::write::write_snapshot(db, &dir).await?;
    taskapp_export::git::commit_and_push(&dir, "export", remote.as_deref()).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_needs_a_directory_then_writes_and_commits() {
        tauri::async_runtime::block_on(async {
            let db = Db::open_in_memory().await.expect("db");
            let e = export_snapshot(&db).await;
            assert!(matches!(e, Err(Error::Invalid(_))), "{e:?}");

            let dir = tempfile::tempdir().expect("tempdir");
            let git = |args: &[&str]| {
                let ok = std::process::Command::new("git")
                    .arg("-C")
                    .arg(dir.path())
                    .args(args)
                    .status()
                    .expect("git")
                    .success();
                assert!(ok, "git {args:?}");
            };
            git(&["init", "-q"]);
            // Commits need an identity; the throwaway repo gets one through its own config.
            git(&["config", "user.name", "t"]);
            git(&["config", "user.email", "t@example.com"]);
            settings::set(&db, "export_dir", &dir.path().to_str().expect("utf-8"))
                .await
                .expect("set");
            export_snapshot(&db).await.expect("export");
            git(&["rev-parse", "--verify", "HEAD"]);
        });
    }
}
