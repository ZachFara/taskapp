use sqlx::SqliteConnection;

use crate::{Db, Issue, IssueId, Result, issues};

/// Each whitespace-separated token becomes a quoted FTS5 phrase, so punctuation and operators in user
/// text are plain words. Archived issues are not searchable.
///
/// # Errors
///
/// Fails on a database error.
pub async fn search(db: &Db, text: &str, limit: u32) -> Result<Vec<Issue>> {
    let query = text
        .split_whitespace()
        .map(|t| format!("\"{}\"", t.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" ");
    if query.is_empty() {
        return Ok(vec![]);
    }
    let sql = format!(
        "SELECT {} FROM issue_fts JOIN issues ON issues.id = issue_fts.rowid
         WHERE issue_fts MATCH ? AND issues.archived_at IS NULL ORDER BY bm25(issue_fts) LIMIT ?",
        issues::ISSUE_COLS
    );
    let rows = sqlx::query(&sql)
        .bind(query)
        .bind(i64::from(limit))
        .fetch_all(&db.pool)
        .await?;
    issues::issues_from_rows(&mut *db.pool.acquire().await?, &rows).await
}

/// Rewrites the issue's FTS row from its title, description and every comment body.
///
/// # Errors
///
/// Fails on a database error.
pub async fn reindex(db: &Db, id: IssueId) -> Result<()> {
    let mut tx = db.pool.begin().await?;
    reindex_in(&mut tx, id).await?;
    tx.commit().await?;
    Ok(())
}

/// [`reindex`] inside the caller's transaction, so the index changes with the data.
pub(crate) async fn reindex_in(conn: &mut SqliteConnection, id: IssueId) -> Result<()> {
    sqlx::query("DELETE FROM issue_fts WHERE rowid = ?")
        .bind(id.0)
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "INSERT INTO issue_fts (rowid, title, description, comments)
         SELECT i.id, i.title, i.description,
                COALESCE((SELECT group_concat(body, char(10)) FROM comments WHERE issue_id = i.id), '')
         FROM issues i WHERE i.id = ?",
    )
    .bind(id.0)
    .execute(conn)
    .await?;
    Ok(())
}
