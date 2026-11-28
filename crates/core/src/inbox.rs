//! Process the Inbox: the issues waiting in the Inbox status, oldest first, and the one-step choices that
//! empty it. Every choice is built from ordinary issue writes, so one command call is one undo step.

use chrono::NaiveDate;

use crate::{
    ChangeSource, Db, Error, InboxChoice, Issue, IssueId, IssuePatch, Result, StatusCategory,
    StatusId, comments, issues, next_actions, statuses::category_str,
};

/// Issues in the status named Inbox (any case), or in the Backlog category when the workspace has none.
/// Archived issues and ones deferred past `today` are left out. Oldest number first.
///
/// # Errors
///
/// Fails on a database error.
pub async fn list(db: &Db, today: NaiveDate) -> Result<Vec<Issue>> {
    let ids: Vec<i64> = sqlx::query_scalar(
        "SELECT i.id FROM issues i JOIN statuses s ON s.id = i.status_id
         WHERE i.archived_at IS NULL AND (i.hidden_until IS NULL OR i.hidden_until <= ?1)
           AND (lower(s.name) = 'inbox'
                OR (s.category = 'backlog' AND NOT EXISTS (SELECT 1 FROM statuses WHERE lower(name) = 'inbox')))
         ORDER BY i.number",
    )
    .bind(today)
    .fetch_all(&db.pool)
    .await?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        out.push(issues::get(db, IssueId(id)).await?);
    }
    Ok(out)
}

/// The status called `name`, else the first of `fallback`'s category (when given).
async fn status(db: &Db, name: &str, fallback: Option<StatusCategory>) -> Result<StatusId> {
    let by_name: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM statuses WHERE lower(name) = lower(?) ORDER BY position, id LIMIT 1",
    )
    .bind(name)
    .fetch_optional(&db.pool)
    .await?;
    let found = match (by_name, fallback) {
        (Some(id), _) => Some(id),
        (None, Some(c)) => {
            sqlx::query_scalar(
                "SELECT id FROM statuses WHERE category = ? ORDER BY position, id LIMIT 1",
            )
            .bind(category_str(c))
            .fetch_optional(&db.pool)
            .await?
        }
        (None, None) => None,
    };
    found
        .map(StatusId)
        .ok_or_else(|| Error::Invalid(format!("no status called {name}; add one in Settings")))
}

/// Applies one Inbox choice. Done, Delegated and Next fall back to the first status of their category;
/// Someday needs a status of that name.
///
/// # Errors
///
/// `Invalid` for a blank delegate note, a defer date that is not after `today`, or a missing status;
/// `NotFound` for an unknown issue or project.
pub async fn process(
    db: &Db,
    id: IssueId,
    choice: InboxChoice,
    today: NaiveDate,
    src: ChangeSource,
) -> Result<Issue> {
    let mut patch = IssuePatch::default();
    let mut note = None;
    let mut flag = None;
    match choice {
        InboxChoice::Done => {
            patch.status_id = Some(status(db, "Done", Some(StatusCategory::Completed)).await?);
        }
        InboxChoice::Delegate { note: n } => {
            if n.trim().is_empty() {
                return Err(Error::Invalid("say who you are waiting on".into()));
            }
            patch.status_id = Some(status(db, "Delegated", Some(StatusCategory::Started)).await?);
            note = Some(format!("Waiting on: {}", n.trim()));
        }
        InboxChoice::Defer { until } => {
            if until <= today {
                return Err(Error::Invalid(
                    "the hide-until date must be after today".into(),
                ));
            }
            patch.hidden_until = Some(Some(until));
        }
        InboxChoice::Next {
            project,
            as_next_action,
        } => {
            if as_next_action && project.is_none() {
                return Err(Error::Invalid("a next action needs a project".into()));
            }
            patch.status_id = Some(status(db, "Next", Some(StatusCategory::Unstarted)).await?);
            patch.project_id = project.map(Some);
            flag = project.filter(|_| as_next_action);
        }
        InboxChoice::Someday => patch.status_id = Some(status(db, "Someday", None).await?),
        InboxChoice::Trash => {
            issues::archive(db, id, src).await?;
            return issues::get(db, id).await;
        }
    }
    let mut issue = issues::update(db, id, patch, src).await?;
    if let Some(note) = note {
        comments::add_comment(db, id, &note, None, src).await?;
    }
    if let Some(project) = flag {
        next_actions::set(db, project, Some(id), src).await?;
        issue = issues::get(db, id).await?;
    }
    Ok(issue)
}
