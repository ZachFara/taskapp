use crate::{Db, Issue, IssueId, Result, views};

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
        views::ISSUE_COLS
    );
    let rows = sqlx::query(&sql)
        .bind(query)
        .bind(i64::from(limit))
        .fetch_all(&db.pool)
        .await?;
    views::issues_from_rows(db, &rows).await
}

/// Rewrites the issue's FTS row from its title, description and every comment body.
///
/// shortcut: `issue_fts` is contentless without `contentless_delete`, so one row cannot be deleted;
/// this clears and rebuilds every row, which is O(issues) per call. When the migration adds
/// `contentless_delete=1`, delete and insert only the one row.
///
/// # Errors
///
/// Fails on a database error.
pub async fn reindex(db: &Db, _id: IssueId) -> Result<()> {
    let mut tx = db.pool.begin().await?;
    sqlx::query("INSERT INTO issue_fts (issue_fts) VALUES ('delete-all')")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO issue_fts (rowid, title, description, comments)
         SELECT i.id, i.title, i.description,
                COALESCE((SELECT group_concat(body, char(10)) FROM comments WHERE issue_id = i.id), '')
         FROM issues i",
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}
