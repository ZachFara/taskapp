//! Links on an issue: the pull request that solved it, a doc, a commit. Every write is a row snapshot (rows.rs),
//! so undo covers adding and removing without a handler here.

use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};

use crate::{ChangeSource, Db, Error, IssueId, Link, LinkId, LinkKind, Result, rows};

const SCHEMES: &[&str] = &["http", "https", "mailto", "obsidian", "taskapp"];

fn bad(msg: impl Into<String>) -> Error {
    Error::Invalid(msg.into())
}

fn word(kind: LinkKind) -> Result<String> {
    match serde_json::to_value(kind).map_err(|e| bad(e.to_string()))? {
        serde_json::Value::String(s) => Ok(s),
        other => Err(bad(format!("not a word: {other}"))),
    }
}

fn from_row(r: &SqliteRow) -> Result<Link> {
    let kind: String = r.try_get("kind")?;
    Ok(Link {
        id: LinkId(r.try_get("id")?),
        issue_id: IssueId(r.try_get("issue_id")?),
        url: r.try_get("url")?,
        title: r.try_get("title")?,
        kind: serde_json::from_value(serde_json::Value::String(kind))
            .map_err(|e| bad(e.to_string()))?,
        created_at: r.try_get("created_at")?,
    })
}

/// The URL's scheme must be one the app can hand on; `javascript:` and `file:` never get in.
fn check(url: &str) -> Result<()> {
    let scheme = url.split_once(':').map(|(s, _)| s.to_ascii_lowercase());
    let ok = scheme.is_some_and(|s| SCHEMES.contains(&s.as_str()))
        && !url.chars().any(char::is_whitespace);
    if ok {
        Ok(())
    } else {
        Err(bad(format!(
            "a link must be one URL starting with {}",
            SCHEMES.join(", ")
        )))
    }
}

async fn fetch(tx: &mut SqliteConnection, issue: i64, url: &str) -> Result<Link> {
    let r = sqlx::query("SELECT * FROM links WHERE issue_id = ? AND url = ?")
        .bind(issue)
        .bind(url)
        .fetch_one(tx)
        .await?;
    from_row(&r)
}

/// The issue's links, oldest first.
///
/// # Errors
///
/// Fails on a database error.
pub async fn list(db: &Db, issue: IssueId) -> Result<Vec<Link>> {
    let rows = sqlx::query("SELECT * FROM links WHERE issue_id = ? ORDER BY id")
        .bind(issue.0)
        .fetch_all(&db.pool)
        .await?;
    rows.iter().map(from_row).collect()
}

/// Idempotent: a URL the issue already has is returned unchanged and records nothing.
///
/// # Errors
///
/// `Invalid` for a URL that is not one allowed link, `NotFound` for an unknown issue.
pub async fn add(
    db: &Db,
    issue: IssueId,
    url: &str,
    title: Option<String>,
    kind: Option<LinkKind>,
    src: ChangeSource,
) -> Result<Link> {
    let url = url.trim();
    check(url)?;
    let kind = word(kind.unwrap_or(LinkKind::Other))?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let done =
        sqlx::query("INSERT OR IGNORE INTO links (issue_id, url, title, kind) VALUES (?, ?, ?, ?)")
            .bind(issue.0)
            .bind(url)
            .bind(title.unwrap_or_default().trim())
            .bind(kind)
            .execute(&mut *tx)
            .await
            .map_err(|e| match &e {
                sqlx::Error::Database(d) if d.is_foreign_key_violation() => {
                    Error::NotFound(format!("issue {}", issue.0))
                }
                _ => Error::Db(e),
            })?;
    if done.rows_affected() == 1 {
        rows::created(&mut tx, src, "links", done.last_insert_rowid()).await?;
    }
    let link = fetch(&mut tx, issue.0, url).await?;
    tx.commit().await?;
    Ok(link)
}

/// Removing a link that does not exist is not an error.
///
/// # Errors
///
/// Fails on a database error.
pub async fn remove(db: &Db, id: LinkId, src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let before = rows::by_key(&mut tx, "links", id.0).await?;
    sqlx::query("DELETE FROM links WHERE id = ?")
        .bind(id.0)
        .execute(&mut *tx)
        .await?;
    rows::changed(&mut tx, src, before).await?;
    tx.commit().await?;
    Ok(())
}
