//! Tauri wrappers: one per command in `taskapp_commands`, generated from its list, plus the commands that
//! need a sync crate (the app is the only place allowed to use those together).

use std::sync::Arc;

use crate::sync_loop::{Real, Syncer};
use taskapp_commands::AppState;
use taskapp_core::{Result, SyncStatus};
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
                gcal_list_calendars, gcal_connect, reminders_list_lists, sync_now
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
