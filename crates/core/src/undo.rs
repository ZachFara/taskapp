//! Undo and redo of whole steps (everything one command call changed), in the database only.
//! Sync targets are not told: the next sync sees the reverted values like any other edit.
//!
//! Covered: issue fields, creation (undone by archiving), archive and restore, labels on an issue, custom
//! property values, and every row of the tables in `rows.rs` (restored whole, same id). An entity this module
//! does not know makes the step refuse with an error, so a step is applied whole or not at all.

use serde_json::{Map, Value, json};
use sqlx::{Row, SqliteConnection};

use crate::{
    ChangeSource, Db, Error, Issue, IssueId, IssuePatch, LabelId, Result, UndoStatus, issues, rows,
};

const ISSUE_FIELDS: &[&str] = &[
    "title",
    "key",
    "archived_at",
    "label",
    "description",
    "status_id",
    "priority",
    "project_id",
    "milestone_id",
    "parent_id",
    "cycle_id",
    "label_ids",
    "start_date",
    "due",
    "estimate_minutes",
    "points",
    "hidden_until",
    "depth",
    "repeat",
    "waiting_on",
    "follow_up",
    "properties",
];

struct Row_ {
    entity: String,
    entity_id: String,
    field: String,
    old: Value,
    new: Value,
}

fn parse(s: Option<String>) -> Result<Value> {
    s.map_or(Ok(Value::Null), |s| {
        serde_json::from_str(&s).map_err(|e| Error::Invalid(e.to_string()))
    })
}

async fn label_of(tx: &mut SqliteConnection, sql: &str) -> Result<Option<String>> {
    Ok(sqlx::query_scalar(sql).fetch_optional(tx).await?.flatten())
}

/// What undo and redo would act on next.
///
/// # Errors
/// Fails when the query fails.
pub async fn undo_status(db: &Db) -> Result<UndoStatus> {
    let mut c = db.pool.acquire().await?;
    Ok(UndoStatus {
        undo: label_of(&mut c, "SELECT step_label FROM changes WHERE undone = 0 AND step IS NOT NULL ORDER BY step DESC LIMIT 1").await?,
        redo: label_of(&mut c, "SELECT step_label FROM changes WHERE undone = 1 ORDER BY step LIMIT 1").await?,
    })
}

/// Reverts the newest step that is not undone.
///
/// # Errors
/// `Invalid` when there is nothing to undo or the step is not covered; `Conflict` naming the field when
/// something changed it since. Nothing is applied on error.
pub async fn undo(db: &Db) -> Result<UndoStatus> {
    run(db, true).await
}

/// Reapplies the step most recently undone, until a new change is made.
///
/// # Errors
/// As [`undo`].
pub async fn redo(db: &Db) -> Result<UndoStatus> {
    run(db, false).await
}

async fn run(db: &Db, undo: bool) -> Result<UndoStatus> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let pick = if undo {
        "SELECT step, step_label FROM changes WHERE undone = 0 AND step IS NOT NULL ORDER BY step DESC LIMIT 1"
    } else {
        "SELECT step, step_label FROM changes WHERE undone = 1 ORDER BY step LIMIT 1"
    };
    let verb = if undo { "undo" } else { "redo" };
    let Some(r) = sqlx::query(pick).fetch_optional(&mut *tx).await? else {
        return Err(Error::Invalid(format!("nothing to {verb}")));
    };
    let step: i64 = r.get(0);
    let label: String = r.get::<Option<String>, _>(1).unwrap_or_default();
    let order = if undo { "DESC" } else { "ASC" };
    let rows = sqlx::query(&format!(
        "SELECT entity, entity_id, field, old_value, new_value FROM changes WHERE step = ? ORDER BY id {order}"
    ))
    .bind(step)
    .fetch_all(&mut *tx)
    .await?;
    let mut changes = Vec::new();
    for r in rows {
        changes.push(Row_ {
            entity: r.get(0),
            entity_id: r.get(1),
            field: r.get(2),
            old: parse(r.get(3))?,
            new: parse(r.get(4))?,
        });
    }
    if let Some(c) = changes.iter().find(|c| !covered(c)) {
        return Err(Error::Invalid(format!(
            "cannot {verb} \"{label}\": changes to {} {} are not covered",
            c.entity, c.field
        )));
    }
    let src = if undo {
        ChangeSource::Undo
    } else {
        ChangeSource::Redo
    };
    for c in &changes {
        // Undo goes back to the old value and expects the new one; redo is the reverse.
        let (target, expect) = if undo {
            (&c.old, &c.new)
        } else {
            (&c.new, &c.old)
        };
        apply(&mut tx, src, c, target, expect, undo).await?;
    }
    sqlx::query("UPDATE changes SET undone = ? WHERE step = ?")
        .bind(i64::from(undo))
        .bind(step)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    undo_status(db).await
}

/// Entities whose old descriptive rows ("created", "deleted", "name") sit beside the row snapshots that undo uses.
const DESCRIPTIVE: &[&str] = &[
    "label",
    "label_group",
    "comment",
    "project",
    "milestone",
    "cycle",
    "property_def",
    "status",
    "project_status",
    "view",
    "view_template",
    "view_folder",
    "template",
];

fn covered(c: &Row_) -> bool {
    c.entity.starts_with(rows::ENTITY_PREFIX)
        || DESCRIPTIVE.contains(&c.entity.as_str())
        || (c.entity == "issue" && c.field == "relation")
        || (c.entity == "issue"
            && c.entity_id.parse::<i64>().is_ok()
            && (ISSUE_FIELDS.contains(&c.field.as_str()) || c.field.starts_with("properties.")))
}

fn conflict(key: &str, field: &str) -> Error {
    Error::Conflict(format!("{field} of {key} changed since this step"))
}

async fn apply(
    tx: &mut SqliteConnection,
    src: ChangeSource,
    c: &Row_,
    target: &Value,
    expect: &Value,
    undo: bool,
) -> Result<()> {
    if let Some(table) = c.entity.strip_prefix(rows::ENTITY_PREFIX) {
        return rows::apply(tx, src, table, &c.entity_id, target, expect).await;
    }
    // Descriptive rows and relation edits are reversed through their row snapshots.
    if c.entity != "issue" || c.field == "relation" {
        return Ok(());
    }
    let id = IssueId(
        c.entity_id
            .parse()
            .map_err(|_| Error::Invalid("bad issue id".into()))?,
    );
    let issue = issues::fetch(tx, id).await?;
    let key = issue.key.as_str();
    let field = c.field.as_str();
    match field {
        // A create is recorded as title and key with no old value; only the key row acts.
        "title" if c.old.is_null() => return Ok(()),
        "key" => return issues::set_archived_in(tx, id, src, undo).await,
        "archived_at" => {
            if issue.archived_at.is_some() == expect.is_null() {
                return Err(conflict(key, field));
            }
            return issues::set_archived_in(tx, id, src, !target.is_null()).await;
        }
        _ => {}
    }
    let patch = patch_for(&issue, field, target, expect)?;
    let patch: IssuePatch =
        serde_json::from_value(Value::Object(patch)).map_err(|e| Error::Invalid(e.to_string()))?;
    issues::restore_in(tx, id, patch, src).await?;
    Ok(())
}

/// The patch that moves `field` to `target`, after checking it still holds `expect`.
fn patch_for(
    issue: &Issue,
    field: &str,
    target: &Value,
    expect: &Value,
) -> Result<Map<String, Value>> {
    let key = issue.key.as_str();
    let current = serde_json::to_value(issue).map_err(|e| Error::Invalid(e.to_string()))?;
    let mut patch = Map::new();
    match field {
        "label" => {
            let label = if target.is_null() { expect } else { target };
            let lid = label
                .as_i64()
                .map(LabelId)
                .ok_or_else(|| Error::Invalid("bad label".into()))?;
            let mut ids = issue.label_ids.clone();
            if ids.contains(&lid) == expect.is_null() {
                return Err(conflict(key, field));
            }
            ids.retain(|l| *l != lid);
            if !target.is_null() {
                ids.push(lid);
            }
            patch.insert("label_ids".into(), json!(ids));
        }
        "properties" => {
            if current["properties"] != *expect {
                return Err(conflict(key, field));
            }
            let want = target.as_object().cloned().unwrap_or_default();
            let mut diff: Map<String, Value> = issue
                .properties
                .keys()
                .filter(|k| !want.contains_key(*k))
                .map(|k| (k.clone(), Value::Null))
                .collect();
            diff.extend(want);
            patch.insert("properties".into(), Value::Object(diff));
        }
        _ if field.starts_with("properties.") => {
            let k = &field["properties.".len()..];
            if issue.properties.get(k).unwrap_or(&Value::Null) != expect {
                return Err(conflict(key, field));
            }
            patch.insert("properties".into(), json!({ k: target }));
        }
        _ => {
            if current[field] != *expect {
                return Err(conflict(key, field));
            }
            patch.insert(field.into(), target.clone());
        }
    }
    Ok(patch)
}
