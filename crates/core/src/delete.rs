//! Deleting issues for good. Everything that points at an issue goes with it; each removed row is snapshotted
//! (see `rows.rs`), so undo brings the issue back with all of it. Run it inside [`crate::history::step`].

use sqlx::SqliteConnection;

use crate::{ChangeSource, Db, IssueId, Result, issues, rows, search};

/// Tables with one `?` filter that finds an issue's rows (the `?` is the issue id unless noted).
const OWNED: &[(&str, &str)] = &[
    ("issue_labels", "issue_id = ?"),
    ("comments", "issue_id = ?"),
    ("relations", "CAST(? AS INTEGER) IN (from_id, to_id)"),
    ("links", "issue_id = ?"),
    ("focus_sessions", "issue_id = ?"),
    ("work_blocks", "issue_id = ?"),
    ("event_issues", "issue_id = ?"),
    ("decision_issues", "issue_id = ?"),
    ("import_log", "issue_id = ?"),
    // Sub-issues and a project's next action only lose their pointer; the rows stay.
    ("issues", "parent_id = ?"),
    ("projects", "next_issue_id = ?"),
];

async fn delete_in(tx: &mut SqliteConnection, id: IssueId, src: ChangeSource) -> Result<()> {
    let issue = issues::fetch(tx, id).await?;
    let mut snaps = Vec::new();
    for &(table, filter) in OWNED {
        snaps.push(rows::snap(tx, table, filter, id.0).await?);
    }
    // Pins name an issue by key; the prefix is display-only, so match on the number.
    let number = issue.key.rsplit('-').next().unwrap_or_default();
    snaps.push(
        rows::snap(
            tx,
            "pins",
            "kind = 'issue' AND target LIKE ?",
            format!("%-{number}"),
        )
        .await?,
    );
    let row = rows::by_key(tx, "issues", id.0).await?;
    sqlx::query("DELETE FROM pins WHERE kind = 'issue' AND target LIKE ?")
        .bind(format!("%-{number}"))
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE issues SET parent_id = NULL WHERE parent_id = ?")
        .bind(id.0)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM import_log WHERE issue_id = ?")
        .bind(id.0)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM issues WHERE id = ?")
        .bind(id.0)
        .execute(&mut *tx)
        .await?;
    search::reindex_in(tx, id).await?;
    // Owned rows first, the issue last: undo runs newest first, so the issue is restored before its rows.
    for s in snaps {
        rows::changed(tx, src, s).await?;
    }
    rows::changed(tx, src, row).await
}

/// Deletes the issues in one transaction; returns how many. Nothing is deleted when one is unknown.
///
/// # Errors
/// `NotFound` for an unknown id, or a database error.
pub async fn remove(db: &Db, ids: &[IssueId], src: ChangeSource) -> Result<u32> {
    let mut ids = ids.to_vec();
    ids.sort_by_key(|i| i.0);
    ids.dedup();
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    for &id in &ids {
        delete_in(&mut tx, id, src).await?;
    }
    tx.commit().await?;
    Ok(u32::try_from(ids.len()).unwrap_or(u32::MAX))
}

/// Deletes every archived issue; returns how many.
///
/// # Errors
/// A database error.
pub async fn empty_archive(db: &Db, src: ChangeSource) -> Result<u32> {
    let ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM issues WHERE archived_at IS NOT NULL")
        .fetch_all(&db.pool)
        .await?;
    remove(db, &ids.into_iter().map(IssueId).collect::<Vec<_>>(), src).await
}
