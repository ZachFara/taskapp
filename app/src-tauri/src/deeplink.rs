//! Handling of `taskapp://` links.

use tauri::{AppHandle, Emitter, Manager};

/// The issue key in `taskapp://issue/<KEY>`, or `None` for anything else, including a malformed key.
#[must_use]
pub fn parse_key(url: &str) -> Option<String> {
    let key = url.strip_prefix("taskapp://issue/")?.trim_end_matches('/');
    let (prefix, number) = key.split_once('-')?;
    let valid = (1..=32).contains(&prefix.len())
        && prefix.starts_with(|c: char| c.is_ascii_uppercase())
        && prefix
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        && (1..=9).contains(&number.len())
        && number.bytes().all(|b| b.is_ascii_digit());
    valid.then(|| key.to_owned())
}

/// The link `taskapp://<kind>/<arg>` for every item type but issues (see [`parse_key`]), or `None` for anything
/// else. `arg` is a slug for a project and a plain number otherwise, so it can never name another route.
#[must_use]
pub fn parse_item(url: &str) -> Option<String> {
    let (kind, arg) = url.strip_prefix("taskapp://")?.split_once('/')?;
    let arg = arg.trim_end_matches('/');
    let ok = match kind {
        "project" => {
            (1..=40).contains(&arg.len())
                && !arg.starts_with('-')
                && arg
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        }
        "view" | "cycle" | "milestone" | "event" | "decision" => {
            (1..=9).contains(&arg.len()) && arg.bytes().all(|b| b.is_ascii_digit())
        }
        _ => false,
    };
    ok.then(|| format!("taskapp://{kind}/{arg}"))
}

/// Focuses the main window and tells the UI to open each linked item.
pub fn open(app: &AppHandle, urls: &[String]) {
    let keys: Vec<String> = urls.iter().filter_map(|u| parse_key(u)).collect();
    let items: Vec<String> = urls.iter().filter_map(|u| parse_item(u)).collect();
    if keys.is_empty() && items.is_empty() {
        return;
    }
    if let Some(w) = app.get_webview_window("main") {
        // Focus is best effort: the link still opens the issue when the window cannot take it.
        w.show().and_then(|()| w.set_focus()).ok();
    }
    for key in keys {
        app.emit("open-issue", key).ok();
    }
    for item in items {
        app.emit("open-link", item).ok();
    }
}
