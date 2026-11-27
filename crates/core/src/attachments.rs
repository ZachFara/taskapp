//! Pasted images: content-addressed files next to the database, one row each.

use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::Row;

use crate::{Attachment, AttachmentData, AttachmentId, Db, Error, Result};

pub const MAX_BYTES: usize = 10 * 1024 * 1024;

/// An image that passed the magic-byte check.
pub struct Image {
    pub sha256: String,
    pub mime: &'static str,
    ext: &'static str,
}

impl Image {
    /// The file name on disk: built from the hash and a fixed extension, never from user text.
    #[must_use]
    pub fn file(&self) -> String {
        format!("{}.{}", self.sha256, self.ext)
    }
}

/// Decides what `bytes` are by their first bytes. Only PNG, JPEG, GIF and WebP pass; SVG never does.
///
/// # Errors
/// `Invalid` when too large or not one of those formats.
pub fn inspect(bytes: &[u8]) -> Result<Image> {
    if bytes.len() > MAX_BYTES {
        return Err(Error::Invalid("image is larger than 10 MB".into()));
    }
    let (mime, ext) = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        ("image/png", "png")
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        ("image/jpeg", "jpg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        ("image/gif", "gif")
    } else if bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        ("image/webp", "webp")
    } else {
        return Err(Error::Invalid(
            "only PNG, JPEG, GIF and WebP images can be attached".into(),
        ));
    };
    Ok(Image {
        sha256: format!("{:x}", Sha256::digest(bytes)),
        mime,
        ext,
    })
}

/// Writes the file unless it is already there. Written under another name and renamed, so a crash leaves no half file.
///
/// # Errors
/// Fails when the folder or file cannot be written.
pub fn write_file(db: &Db, img: &Image, bytes: &[u8]) -> Result<()> {
    let path = db.files.join(img.file());
    if path.exists() {
        return Ok(());
    }
    fs::create_dir_all(&db.files)?;
    let partial = db.files.join(format!("{}.partial", img.sha256));
    fs::write(&partial, bytes)?;
    fs::rename(&partial, path)?;
    Ok(())
}

fn row(r: &sqlx::sqlite::SqliteRow) -> Result<Attachment> {
    Ok(Attachment {
        id: AttachmentId(r.try_get("id")?),
        sha256: r.try_get("sha256")?,
        mime: r.try_get("mime")?,
        bytes: r.try_get("bytes")?,
        created_at: r.try_get::<DateTime<Utc>, _>("created_at")?,
    })
}

/// Stores an image. The same bytes pasted twice give the same attachment. `claimed_mime` must agree with the bytes.
///
/// # Errors
/// `Invalid` for a wrong type, a spoofed type or more than 10 MB; otherwise a file or database error.
pub async fn add(db: &Db, bytes: &[u8], claimed_mime: &str) -> Result<Attachment> {
    let img = inspect(bytes)?;
    if claimed_mime != img.mime {
        return Err(Error::Invalid(format!(
            "the file says {claimed_mime} but its bytes are {}",
            img.mime
        )));
    }
    write_file(db, &img, bytes)?;
    sqlx::query(
        "INSERT OR IGNORE INTO attachments (sha256, file, mime, bytes, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&img.sha256)
    .bind(img.file())
    .bind(img.mime)
    .bind(i64::try_from(bytes.len()).map_err(|e| Error::Invalid(e.to_string()))?)
    .bind(Utc::now())
    .execute(&db.pool)
    .await?;
    let r = sqlx::query("SELECT * FROM attachments WHERE sha256 = ?")
        .bind(&img.sha256)
        .fetch_one(&db.pool)
        .await?;
    row(&r)
}

/// # Errors
/// `NotFound` for an unknown id or a missing file.
pub async fn get(db: &Db, id: AttachmentId) -> Result<AttachmentData> {
    let r = sqlx::query("SELECT file, mime FROM attachments WHERE id = ?")
        .bind(id.0)
        .fetch_optional(&db.pool)
        .await?
        .ok_or_else(|| Error::NotFound(format!("attachment {}", id.0)))?;
    let file: String = r.try_get("file")?;
    let bytes = fs::read(db.files.join(file))
        .map_err(|_| Error::NotFound(format!("file of attachment {}", id.0)))?;
    Ok(AttachmentData {
        mime: r.try_get("mime")?,
        bytes,
    })
}

/// Every attachment some issue, comment or project text points at with `attachment:<id>`.
///
/// # Errors
/// Fails on a database error.
pub async fn referenced(db: &Db) -> Result<Vec<Attachment>> {
    let rows = sqlx::query(
        "SELECT a.* FROM attachments a WHERE EXISTS (
            SELECT 1 FROM (SELECT description AS t FROM issues UNION ALL SELECT body FROM comments
                           UNION ALL SELECT description FROM projects)
            WHERE t LIKE '%(attachment:' || a.id || ')%') ORDER BY a.id",
    )
    .fetch_all(&db.pool)
    .await?;
    rows.iter().map(row).collect()
}

/// One attachment's row and the path of its file. The file is not opened.
///
/// # Errors
/// `NotFound` for an unknown id.
pub async fn locate(db: &Db, id: AttachmentId) -> Result<(Attachment, PathBuf)> {
    let r = sqlx::query("SELECT * FROM attachments WHERE id = ?")
        .bind(id.0)
        .fetch_optional(&db.pool)
        .await?
        .ok_or_else(|| Error::NotFound(format!("attachment {}", id.0)))?;
    let file: String = r.try_get("file")?;
    Ok((row(&r)?, db.files.join(file)))
}

/// The attachments `text` points at with `(attachment:<id>)`, in the order first mentioned. Ids with no row are skipped.
///
/// # Errors
/// Fails on a database error.
pub async fn in_text(db: &Db, text: &str) -> Result<Vec<(Attachment, PathBuf)>> {
    let mut ids: Vec<i64> = Vec::new();
    for part in text.split("(attachment:").skip(1) {
        let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
        let Ok(id) = digits.parse::<i64>() else {
            continue;
        };
        if part[digits.len()..].starts_with(')') && !ids.contains(&id) {
            ids.push(id);
        }
    }
    let mut found = Vec::new();
    for id in ids {
        match locate(db, AttachmentId(id)).await {
            Ok(hit) => found.push(hit),
            Err(Error::NotFound(_)) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(found)
}
