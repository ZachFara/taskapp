use sqlx::SqliteConnection;

use crate::{Db, Issue, IssueId, Result, issues};

/// Each whitespace-separated token becomes a quoted FTS5 phrase with a prefix star, so punctuation and operators in
/// user text are plain words and "cras" finds "crash". Archived issues are not searchable.
///
/// # Errors
///
/// Fails on a database error.
pub async fn search(db: &Db, text: &str, limit: u32) -> Result<Vec<Issue>> {
    let query = text
        .split_whitespace()
        .map(|t| format!("\"{}\"*", t.replace('"', "\"\"")))
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

/// Lowercased letter/digit runs of two or more characters, deduplicated.
fn tokens(text: &str) -> std::collections::BTreeSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.chars().count() >= 2)
        .map(str::to_owned)
        .collect()
}

/// Minimum word overlap (Jaccard) for an issue to count as a likely duplicate.
pub const SIMILAR_MIN: f64 = 0.3;

fn count(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// Open issues that look like `text` (a title, optionally followed by a description), best first, at most `limit`.
/// The full-text index picks up to 100 candidates sharing a word; the score is the Jaccard overlap of whole words with the
/// issue's title (a long description in `text` lowers scores; shortcut: titles only, since a long description on the
/// candidate swamps a short query), at least [`SIMILAR_MIN`]. Ties go to the lower id. `project` narrows to one project.
///
/// # Errors
///
/// Fails on a database error.
pub async fn similar(
    db: &Db,
    text: &str,
    project: Option<crate::ProjectId>,
    limit: u32,
) -> Result<Vec<Issue>> {
    let want = tokens(text);
    if want.is_empty() {
        return Ok(vec![]);
    }
    let query = want
        .iter()
        .map(|t| format!("\"{t}\""))
        .collect::<Vec<_>>()
        .join(" OR ");
    let sql = format!(
        "SELECT {} FROM issue_fts JOIN issues ON issues.id = issue_fts.rowid
         WHERE issue_fts MATCH ? AND issues.archived_at IS NULL AND issues.completed_at IS NULL
           AND (? IS NULL OR issues.project_id = ?)
         ORDER BY bm25(issue_fts) LIMIT 100",
        issues::ISSUE_COLS
    );
    let rows = sqlx::query(&sql)
        .bind(query)
        .bind(project.map(|p| p.0))
        .bind(project.map(|p| p.0))
        .fetch_all(&db.pool)
        .await?;
    let candidates = issues::issues_from_rows(&mut *db.pool.acquire().await?, &rows).await?;
    let mut scored: Vec<(f64, Issue)> = candidates
        .into_iter()
        .filter_map(|i| {
            let have = tokens(&i.title);
            let both = want.intersection(&have).count();
            let score = count(both) / count(want.len() + have.len() - both);
            (score >= SIMILAR_MIN).then_some((score, i))
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.id.0.cmp(&b.1.id.0)));
    Ok(scored
        .into_iter()
        .take(limit as usize)
        .map(|s| s.1)
        .collect())
}
