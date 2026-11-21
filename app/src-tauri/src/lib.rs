//! The Tauri shell: wires the commands, plugins and bindings export together.

mod commands;
pub mod deeplink;
pub mod sync_loop;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use specta_typescript::Typescript;
use taskapp_commands::AppState;
use taskapp_core::{Db, settings};
use tauri::{Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

use crate::sync_loop::{Real, Syncer, Timing};

const BINDINGS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/lib/bindings.ts");

/// Every command, with ids exported as plain numbers (row ids stay far below 2^53).
#[must_use]
pub fn builder() -> tauri_specta::Builder<tauri::Wry> {
    commands::builder()
        .dangerously_cast_bigints_to_number()
        .disable_serde_phases()
}

/// Writes the TypeScript bindings to `path`.
///
/// # Errors
/// Fails when the directory or file cannot be written.
pub fn export_bindings(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    builder().export(Typescript::default(), path)?;
    Ok(())
}

/// IANA name of the system time zone, from the `/etc/localtime` symlink.
fn system_tz() -> String {
    std::fs::read_link("/etc/localtime")
        .ok()
        .and_then(|p| {
            p.to_str()
                .and_then(|s| s.split_once("zoneinfo/").map(|(_, tz)| tz.to_owned()))
        })
        .unwrap_or_else(|| "UTC".to_owned())
}

/// Records the system zone as the `timezone` setting when the owner has not set one, so the
/// calendar's 08:00 alerts land at local time.
async fn default_timezone(db: &Db, tz: &str) -> taskapp_core::Result<()> {
    if settings::get::<String>(db, "timezone").await?.is_none() {
        settings::set(db, "timezone", &tz).await?;
    }
    Ok(())
}

/// The sync loop, the reminders watcher and the live-refresh poll, all in the app's runtime.
fn start_background(
    app: &tauri::AppHandle,
    db: &Db,
    helper: PathBuf,
    backup_dir: PathBuf,
    syncer: Arc<Syncer<Real>>,
) {
    let (cal_tx, cal_rx) = tokio::sync::mpsc::unbounded_channel();
    let (rem_tx, rem_rx) = tokio::sync::mpsc::unbounded_channel();
    let handle = app.clone();
    let poll_db = db.clone();
    tauri::async_runtime::spawn(async move {
        let result = sync_loop::poll_db(poll_db, Duration::from_secs(1), move |calendar| {
            handle.emit("db-changed", ()).ok();
            if calendar {
                cal_tx.send(()).ok();
            }
        })
        .await;
        eprintln!("live refresh stopped: {result:?}");
    });
    tauri::async_runtime::spawn(sync_loop::watch_reminders(db.clone(), helper, rem_tx));
    tauri::async_runtime::spawn(sync_loop::run(
        syncer,
        Timing::PRODUCTION,
        backup_dir,
        cal_rx,
        rem_rx,
    ));
}

/// Runs the app.
///
/// # Panics
/// When the database cannot be opened or Tauri fails to start.
pub fn run() {
    #[cfg(debug_assertions)]
    export_bindings(Path::new(BINDINGS)).expect("export bindings");
    let handler = builder().invoke_handler();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(handler)
        .setup(|app| {
            let dir: PathBuf = app.path().data_dir()?.join("taskapp");
            std::fs::create_dir_all(&dir)?;
            let db = tauri::async_runtime::block_on(Db::open(&dir.join("taskapp.db")))?;
            let bundled = std::env::current_exe()?
                .parent()
                .map_or_else(PathBuf::new, |p| p.join("reminders-helper"));
            let helper_path = sync_loop::resolve_helper(bundled);
            let backup_dir =
                tauri::async_runtime::block_on(settings::get::<String>(&db, "backup_dir"))?
                    .map_or_else(|| dir.join("backups"), PathBuf::from);
            let tz = system_tz();
            tauri::async_runtime::block_on(default_timezone(&db, &tz))?;
            app.manage(AppState {
                db: db.clone(),
                helper_path: helper_path.clone(),
                tz: tz.clone(),
            });
            let real = Real {
                db: db.clone(),
                helper: helper_path.clone(),
                tz,
                backup_dir: backup_dir.clone(),
            };
            let syncer = Arc::new(Syncer::new(real, db.clone()));
            app.manage(syncer.clone());
            start_background(app.handle(), &db, helper_path, backup_dir, syncer);
            let handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                let urls: Vec<String> = event.urls().iter().map(ToString::to_string).collect();
                deeplink::open(&handle, &urls);
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("run tauri app");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn timezone_defaults_to_system_but_keeps_an_owner_value() {
        let db = Db::open_in_memory().await.expect("db");
        default_timezone(&db, "America/New_York")
            .await
            .expect("set");
        assert_eq!(
            settings::get::<String>(&db, "timezone")
                .await
                .expect("get")
                .as_deref(),
            Some("America/New_York")
        );
        default_timezone(&db, "UTC").await.expect("keep");
        assert_eq!(
            settings::get::<String>(&db, "timezone")
                .await
                .expect("get")
                .as_deref(),
            Some("America/New_York")
        );
    }
}
