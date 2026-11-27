//! Icon and color names the CLI accepts. The icon list mirrors `ui/src/ui/icons.ts`; a test in
//! `tests/cli.rs` fails when the two drift apart.

use taskapp_core::{Error, Result, palette};

pub const ICONS: &[&str] = &[
    "cairn",
    "inbox",
    "user",
    "user-circle",
    "pin",
    "home",
    "settings",
    "search",
    "command",
    "bell",
    "help",
    "list",
    "board",
    "table",
    "timeline",
    "calendar",
    "layers",
    "panel",
    "project",
    "cycle",
    "folder",
    "folder-open",
    "tag",
    "flag",
    "star",
    "bookmark",
    "target",
    "bolt",
    "circle",
    "branch",
    "sub-issue",
    "clock",
    "link",
    "rocket",
    "heart",
    "globe",
    "briefcase",
    "book",
    "book-open",
    "code",
    "terminal",
    "database",
    "cloud",
    "lock",
    "key",
    "shield",
    "leaf",
    "tree",
    "flower",
    "mountain",
    "compass",
    "map",
    "location",
    "airplane",
    "car",
    "bicycle",
    "music",
    "camera",
    "image",
    "video",
    "microphone",
    "headphones",
    "coffee",
    "gift",
    "trophy",
    "lightbulb",
    "wrench",
    "tools",
    "palette",
    "chart",
    "clipboard",
    "document",
    "mail",
    "chat",
    "users",
    "puzzle",
    "plus",
    "minus",
    "close",
    "check",
    "more",
    "filter",
    "sort",
    "group",
    "edit",
    "trash",
    "archive",
    "copy",
    "undo",
    "download",
    "eye",
    "sun",
    "moon",
    "arrow-right",
    "arrow-up",
    "arrow-down",
    "chevron-down",
    "chevron-up",
    "chevron-right",
    "chevron-left",
];

/// `NAME` from the app's icon list, stored as `icon:NAME`.
pub fn icon(name: &str) -> Result<String> {
    if ICONS.contains(&name) {
        Ok(format!("icon:{name}"))
    } else {
        Err(Error::Invalid(format!(
            "icon {name} is not an app icon; use one of: {}",
            ICONS.join(", ")
        )))
    }
}

/// A palette key, as listed by `taskapp colors`.
pub fn color(key: &str) -> Result<String> {
    if palette::COLORS.iter().any(|(k, _)| *k == key) {
        Ok(key.to_owned())
    } else {
        let keys: Vec<&str> = palette::COLORS.iter().map(|(k, _)| *k).collect();
        Err(Error::Invalid(format!(
            "color {key} is not a palette color; use one of: {}",
            keys.join(", ")
        )))
    }
}
