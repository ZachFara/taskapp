//! Each project's one flagged next action, and which active projects have none (stuck).

use serde_json::json;
use sqlx::Row;

use crate::{
    ChangeSource, Db, Error, IssueId, NextAction, ProjectId, Result, issues, projects, rows,
};

/// The flag counts while the issue is in the project, not archived and not finished, so the column is
/// never cleared by a write elsewhere (undo then brings the flag back with the issue).
const OPEN: &str = "SELECT i.id FROM issues i JOIN statuses s ON s.id = i.status_id
    WHERE i.id = p.next_issue_id AND i.project_id = p.id AND i.archived_at IS NULL
      AND s.category NOT IN ('completed', 'canceled')";

/// A project is active when its status is Started and it carries neither the Area nor the Collection label.
const ACTIVE: &str = "ps.category = 'started' AND NOT EXISTS (SELECT 1 FROM project_labels pl
    JOIN labels l ON l.id = pl.label_id WHERE pl.project_id = p.id AND lower(l.name) IN ('area', 'collection'))";

/// Every unarchived project with its open next action, if any. `stuck` is set for an active project without one.
///
/// # Errors
///
/// Fails on a database error.
pub async fn list(db: &Db) -> Result<Vec<NextAction>> {
    let found = sqlx::query(&format!(
        "SELECT p.id, ({OPEN}) AS next, ({ACTIVE}) AS active
         FROM projects p JOIN project_statuses ps ON ps.id = p.status_id
         WHERE p.archived_at IS NULL ORDER BY p.id"
    ))
    .fetch_all(&db.pool)
    .await?;
    let mut all = projects::list(db).await?;
    let mut out = Vec::new();
    for r in found {
        let id: i64 = r.try_get("id")?;
        let next: Option<i64> = r.try_get("next")?;
        let active: bool = r.try_get("active")?;
        let Some(at) = all.iter().position(|p| p.id.0 == id) else {
            continue;
        };
        let issue = match next {
            Some(n) => Some(issues::get(db, IssueId(n)).await?),
            None => None,
        };
        out.push(NextAction {
            project: all.swap_remove(at),
            stuck: active && issue.is_none(),
            issue,
        });
    }
    Ok(out)
}

/// Flags `issue` as the project's next action, or clears the flag with `None`.
///
/// # Errors
///
/// `NotFound` for an unknown project or issue; `Invalid` when the issue is not in the project or is finished.
pub async fn set(
    db: &Db,
    project: ProjectId,
    issue: Option<IssueId>,
    src: ChangeSource,
) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let old = projects::find(&mut tx, project).await?;
    if let Some(i) = issue {
        let got = issues::fetch(&mut tx, i).await?;
        if got.project_id != Some(project) {
            return Err(Error::Invalid(format!(
                "{} is not in project {}",
                got.key, old.slug
            )));
        }
        let done: bool = sqlx::query_scalar(
            "SELECT category IN ('completed', 'canceled') FROM statuses WHERE id = ?",
        )
        .bind(got.status_id.0)
        .fetch_one(&mut *tx)
        .await?;
        if done {
            return Err(Error::Invalid(format!("{} is already finished", got.key)));
        }
    }
    let before = rows::by_key(&mut tx, "projects", project.0).await?;
    let was: Option<i64> = sqlx::query_scalar("SELECT next_issue_id FROM projects WHERE id = ?")
        .bind(project.0)
        .fetch_one(&mut *tx)
        .await?;
    projects::edit(
        &mut tx,
        src,
        ("project", project.0),
        "next_issue_id",
        json!(was),
        json!(issue.map(|i| i.0)),
    )
    .await?;
    sqlx::query("UPDATE projects SET next_issue_id = ? WHERE id = ?")
        .bind(issue.map(|i| i.0))
        .bind(project.0)
        .execute(&mut *tx)
        .await?;
    rows::changed(&mut tx, src, before).await?;
    tx.commit().await?;
    Ok(())
}
