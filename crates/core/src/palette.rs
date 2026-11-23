//! Icons and colors. A color is a key from one fixed palette; the hex beside each key is a reference
//! default for showing it.
//! shortcut: the reference hex values are placeholders until the design round picks per-theme values.

use sqlx::SqlitePool;

use crate::{Error, Result};

pub const COLORS: [(&str, &str); 12] = [
    ("gray", "#6b7280"),
    ("red", "#ef4444"),
    ("orange", "#f97316"),
    ("amber", "#f59e0b"),
    ("yellow", "#eab308"),
    ("green", "#22c55e"),
    ("teal", "#14b8a6"),
    ("cyan", "#06b6d4"),
    ("blue", "#3b82f6"),
    ("indigo", "#6366f1"),
    ("purple", "#a855f7"),
    ("pink", "#ec4899"),
];

/// The icon and color an entity is created or updated with. On create `None` means unset; on update it
/// means keep the current value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Look {
    pub icon: Option<String>,
    pub color: Option<String>,
}

/// The reference hex for a palette key.
#[must_use]
pub fn hex_of(key: &str) -> Option<&'static str> {
    COLORS.iter().find(|(k, _)| *k == key).map(|(_, hex)| *hex)
}

fn rgb(hex: &str) -> Option<[i32; 3]> {
    let digits = hex.strip_prefix('#')?;
    if digits.len() != 6 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let n = i32::from_str_radix(digits, 16).ok()?;
    Some([(n >> 16) & 255, (n >> 8) & 255, n & 255])
}

/// The palette key whose reference hex is nearest (squared RGB distance) to `hex`.
fn nearest_key(hex: &str) -> Option<&'static str> {
    let [r, g, b] = rgb(hex)?;
    COLORS
        .iter()
        .min_by_key(|(_, reference)| {
            rgb(reference).map_or(i32::MAX, |[pr, pg, pb]| {
                (pr - r).pow(2) + (pg - g).pow(2) + (pb - b).pow(2)
            })
        })
        .map(|(key, _)| *key)
}

/// The reference hex for a stored key. A key outside the palette (hand-edited data) shows as gray.
#[must_use]
pub fn resolve(key: &str) -> String {
    hex_of(key).unwrap_or(COLORS[0].1).to_owned()
}

/// A palette key stays as it is. A `#rrggbb` value maps to its nearest key, so clients that still send
/// hex keep working.
///
/// # Errors
/// `Invalid` when the value is neither a palette key nor a hex color.
pub fn key_for(color: &str) -> Result<&'static str> {
    if let Some((key, _)) = COLORS.iter().find(|(k, _)| *k == color) {
        return Ok(key);
    }
    nearest_key(color).ok_or_else(|| Error::Invalid(format!("color {color} is not a palette key")))
}

/// `icon:<id>` with a lowercase id, or `emoji:<emoji>`.
///
/// shortcut: an `emoji:` value is accepted as any run of 1 to 8 non-ASCII code points, not checked against
/// emoji data; upgrade with an emoji table when the icon picker lands.
///
/// # Errors
/// `Invalid` for any other shape.
pub fn check_icon(icon: &str) -> Result<()> {
    let valid = if let Some(id) = icon.strip_prefix("icon:") {
        !id.is_empty()
            && id.len() <= 40
            && id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    } else if let Some(emoji) = icon.strip_prefix("emoji:") {
        (1..=8).contains(&emoji.chars().count())
            && emoji.chars().all(|c| !c.is_ascii() && !c.is_control())
    } else {
        false
    };
    if valid {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "icon {icon} must be icon:<id> or emoji:<emoji>"
        )))
    }
}

/// Rewrites hex colors left in statuses and labels to the nearest palette key. Rows already on a key,
/// and hex that does not parse, are left alone.
///
/// # Errors
/// Fails on a database error.
pub async fn map_legacy_colors(pool: &SqlitePool) -> Result<()> {
    for table in ["statuses", "labels"] {
        let rows: Vec<(i64, String)> = sqlx::query_as(&format!(
            "SELECT id, color FROM {table} WHERE color LIKE '#%'"
        ))
        .fetch_all(pool)
        .await?;
        for (id, hex) in rows {
            if let Some(key) = nearest_key(&hex) {
                sqlx::query(&format!("UPDATE {table} SET color = ? WHERE id = ?"))
                    .bind(key)
                    .bind(id)
                    .execute(pool)
                    .await?;
                // The recorded snapshots must follow, or undo of an older step reads the change as an edit.
                for col in ["old_value", "new_value"] {
                    sqlx::query(&format!(
                        "UPDATE changes SET {col} = json_set({col}, '$.color', ?1) WHERE entity = ?2 AND json_extract({col}, '$.color') = ?3"
                    ))
                    .bind(key)
                    .bind(format!("row:{table}"))
                    .bind(&hex)
                    .execute(pool)
                    .await?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_passes_and_unknown_key_is_rejected() {
        assert_eq!(key_for("teal").ok(), Some("teal"));
        assert!(key_for("teal2").is_err());
        assert!(key_for("").is_err());
    }

    #[test]
    fn hex_maps_to_nearest_key() {
        assert_eq!(key_for("#5e6ad2").ok(), Some("indigo"));
        assert_eq!(key_for("#f2c94c").ok(), Some("yellow"));
        assert!(key_for("#f00").is_err());
    }

    #[test]
    fn icon_shapes() {
        assert!(check_icon("icon:folder-open").is_ok());
        assert!(check_icon("emoji:🚀").is_ok());
        assert!(check_icon("icon:Folder").is_err());
        assert!(check_icon("icon:").is_err());
        assert!(check_icon("emoji:").is_err());
        assert!(check_icon("emoji:rocket").is_err());
        assert!(check_icon("rocket").is_err());
    }
}
