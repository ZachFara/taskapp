use chrono::{DateTime, Utc};
use serde_json::json;

use crate::{
    ChangeSource, Comment, CommentId, Db, Error, IssueId, Result,
    history::{self, Edit},
    search,
};

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn list(db: &Db, issue: IssueId) -> Result<Vec<Comment>> {
    // Ordered by time in SQL, not text: stored timestamps can differ in fractional digits.
    let rows = sqlx::query!(
        r#"SELECT id AS "id!: i64", issue_id, body, created_at AS "created_at: DateTime<Utc>",
                  updated_at AS "updated_at: DateTime<Utc>"
           FROM comments WHERE issue_id = ? ORDER BY julianday(created_at), id"#,
        issue.0
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| Comment {
            id: CommentId(r.id),
            issue_id: IssueId(r.issue_id),
            body: r.body,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
        .collect())
}

/// `created_at` defaults to now; a migration passes the original time.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn add_comment(
    db: &Db,
    issue: IssueId,
    body: &str,
    created_at: Option<DateTime<Utc>>,
    src: ChangeSource,
) -> Result<Comment> {
    let at = created_at.unwrap_or_else(Utc::now);
    let mut tx = db.pool.begin().await?;
    let id = sqlx::query!(
        "INSERT INTO comments (issue_id, body, created_at, updated_at) VALUES (?, ?, ?, ?)",
        issue.0,
        body,
        at,
        at
    )
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();
    let new = json!(body);
    let edit = Edit {
        field: "body",
        old: None,
        new: Some(&new),
    };
    history::record(&mut tx, src, "comment", &id.to_string(), &edit).await?;
    search::reindex_in(&mut tx, issue).await?;
    tx.commit().await?;
    Ok(Comment {
        id: CommentId(id),
        issue_id: issue,
        body: body.to_owned(),
        created_at: at,
        updated_at: at,
    })
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn update(db: &Db, id: CommentId, body: &str, src: ChangeSource) -> Result<Comment> {
    let mut tx = db.pool.begin().await?;
    let old = sqlx::query!(
        r#"SELECT issue_id, body, created_at AS "created_at: DateTime<Utc>" FROM comments WHERE id = ?"#,
        id.0
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| Error::NotFound(format!("comment {}", id.0)))?;
    if old.body != body {
        let (before, after) = (json!(old.body), json!(body));
        let edit = Edit {
            field: "body",
            old: Some(&before),
            new: Some(&after),
        };
        history::record(&mut tx, src, "comment", &id.0.to_string(), &edit).await?;
    }
    let now = Utc::now();
    sqlx::query!(
        "UPDATE comments SET body = ?, updated_at = ? WHERE id = ?",
        body,
        now,
        id.0
    )
    .execute(&mut *tx)
    .await?;
    search::reindex_in(&mut tx, IssueId(old.issue_id)).await?;
    tx.commit().await?;
    Ok(Comment {
        id,
        issue_id: IssueId(old.issue_id),
        body: body.to_owned(),
        created_at: old.created_at,
        updated_at: now,
    })
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn delete(db: &Db, id: CommentId, src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin().await?;
    let (body, issue): (String, i64) =
        sqlx::query_as("SELECT body, issue_id FROM comments WHERE id = ?")
            .bind(id.0)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| Error::NotFound(format!("comment {}", id.0)))?;
    sqlx::query!("DELETE FROM comments WHERE id = ?", id.0)
        .execute(&mut *tx)
        .await?;
    let old = json!(body);
    let edit = Edit {
        field: "body",
        old: Some(&old),
        new: None,
    };
    history::record(&mut tx, src, "comment", &id.0.to_string(), &edit).await?;
    search::reindex_in(&mut tx, IssueId(issue)).await?;
    tx.commit().await?;
    Ok(())
}
