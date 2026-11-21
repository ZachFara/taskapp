//! The Tauri shell: wires the commands, plugins and bindings export together.

mod commands;
mod deeplink;
mod sync_loop;

use std::path::{Path, PathBuf};

use specta_typescript::Typescript;
use taskapp_commands::AppState;
use taskapp_core::Db;
use tauri::Manager;

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
        .invoke_handler(handler)
        .setup(|app| {
            let dir: PathBuf = app.path().data_dir()?.join("taskapp");
            std::fs::create_dir_all(&dir)?;
            let db = tauri::async_runtime::block_on(Db::open(&dir.join("taskapp.db")))?;
            let helper_path = app.path().resource_dir()?.join("reminders-helper");
            app.manage(AppState {
                db,
                helper_path,
                tz: system_tz(),
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("run tauri app");
}
