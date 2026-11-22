//! One patch applied to many issues in a single transaction.

use crate::{
    BulkPatch, ChangeSource, Db, Issue, IssueId, NewIssue, Relation, Result, issues, relations,
};

/// Applies `patch` to every issue in `ids`; any failure rolls all of it back. Run it inside
/// [`crate::history::step`] so the whole selection is one undo step.
///
/// # Errors
/// As [`issues::update_in`] and [`relations::add`]; an issue that would relate to itself is `Invalid`.
pub async fn update(db: &Db, ids: &[IssueId], patch: BulkPatch, src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    for &id in ids {
        let mut p = patch.patch.clone();
        if p.label_ids.is_none() && !(patch.add_labels.is_empty() && patch.remove_labels.is_empty())
        {
            let mut l = issues::fetch(&mut tx, id).await?.label_ids;
            l.retain(|x| !patch.remove_labels.contains(x));
            l.extend(
                patch
                    .add_labels
                    .iter()
                    .filter(|x| !l.contains(x))
                    .copied()
                    .collect::<Vec<_>>(),
            );
            p.label_ids = Some(l);
        }
        issues::update_in(&mut tx, id, p, src).await?;
        if let Some(a) = patch.archive {
            issues::set_archived_in(&mut tx, id, src, a).await?;
        }
        if let Some((kind, to)) = patch.add_relation {
            relations::add_in(&mut tx, Relation { from: id, to, kind }, src).await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

/// Copies every issue in `ids` (fields, labels and properties; not relations, comments or history) in a single
/// transaction, so a missing issue copies nothing. Run it inside [`crate::history::step`] for one undo step.
///
/// # Errors
/// `NotFound` for an unknown issue, otherwise as [`issues::create_in`].
pub async fn duplicate(db: &Db, ids: &[IssueId], src: ChangeSource) -> Result<Vec<Issue>> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let mut copies = Vec::with_capacity(ids.len());
    for &id in ids {
        let i = issues::fetch(&mut tx, id).await?;
        let new = NewIssue {
            title: i.title,
            description: Some(i.description),
            status_id: Some(i.status_id),
            priority: Some(i.priority),
            project_id: i.project_id,
            milestone_id: i.milestone_id,
            parent_id: i.parent_id,
            cycle_id: i.cycle_id,
            label_ids: Some(i.label_ids),
            start_date: i.start_date,
            due: i.due,
            estimate_minutes: i.estimate_minutes,
            points: i.points,
            properties: Some(i.properties),
            ..NewIssue::default()
        };
        copies.push(issues::create_in(&mut tx, new, src).await?);
    }
    tx.commit().await?;
    Ok(copies)
}
