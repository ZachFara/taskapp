use std::{cell::Cell, future::Future};

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::SqliteConnection;

use crate::{Change, ChangeSource, Db, Error, Result};

fn enc<T: serde::Serialize>(v: &T) -> Result<String> {
    serde_json::to_string(v).map_err(|e| Error::Invalid(e.to_string()))
}

fn dec<T: serde::de::DeserializeOwned>(s: &str) -> Result<T> {
    serde_json::from_str(s).map_err(|e| Error::Invalid(e.to_string()))
}

struct Step {
    label: String,
    id: Cell<Option<i64>>,
}

tokio::task_local! { static STEP: Step; }

/// Runs `f` so that every change it records belongs to one undoable step. Inside another step it joins that one.
///
/// # Errors
/// Whatever `f` returns.
pub async fn step<T>(label: &str, f: impl Future<Output = Result<T>>) -> Result<T> {
    if STEP.try_with(|_| ()).is_ok() {
        return f.await;
    }
    let s = Step {
        label: label.to_owned(),
        id: Cell::new(None),
    };
    STEP.scope(s, f).await
}

/// The step the current change joins, allocated on the first change. A new step ends the redo stack.
async fn current_step(tx: &mut SqliteConnection) -> Result<Option<(i64, String)>> {
    let Ok((label, id)) = STEP.try_with(|s| (s.label.clone(), s.id.get())) else {
        return Ok(None);
    };
    if let Some(id) = id {
        return Ok(Some((id, label)));
    }
    let id: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(step), 0) + 1 FROM changes")
        .fetch_one(&mut *tx)
        .await?;
    sqlx::query("UPDATE changes SET undone = 2 WHERE undone = 1")
        .execute(&mut *tx)
        .await?;
    STEP.with(|s| s.id.set(Some(id)));
    Ok(Some((id, label)))
}

/// One field's before and after values.
pub struct Edit<'a> {
    pub field: &'a str,
    pub old: Option<&'a Value>,
    pub new: Option<&'a Value>,
}

/// Appends one change inside the caller's transaction.
///
/// # Errors
/// Fails when the insert fails.
pub async fn record(
    tx: &mut SqliteConnection,
    source: ChangeSource,
    entity: &str,
    entity_id: &str,
    edit: &Edit<'_>,
) -> Result<()> {
    let at = Utc::now();
    // Undo and redo are recorded but are never steps themselves, so they cannot loop.
    let (step, label) = match source {
        ChangeSource::Undo | ChangeSource::Redo => None,
        _ => current_step(tx).await?,
    }
    .unzip();
    let source = enc(&source)?;
    let old = edit.old.map(enc).transpose()?;
    let new = edit.new.map(enc).transpose()?;
    sqlx::query!(
        "INSERT INTO changes (at, source, entity, entity_id, field, old_value, new_value, step, step_label)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        at,
        source,
        entity,
        entity_id,
        edit.field,
        old,
        new,
        step,
        label
    )
    .execute(tx)
    .await?;
    Ok(())
}

/// Changes to one entity, oldest first. An unknown entity has none.
///
/// # Errors
/// Fails when the query fails or a stored row cannot be decoded.
pub async fn list_for(db: &Db, entity: &str, entity_id: &str) -> Result<Vec<Change>> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!: i64", at AS "at: DateTime<Utc>", source, field, old_value, new_value
           FROM changes WHERE entity = ? AND entity_id = ? ORDER BY id"#,
        entity,
        entity_id
    )
    .fetch_all(&db.pool)
    .await?;
    rows.into_iter()
        .map(|r| {
            Ok(Change {
                id: r.id,
                at: r.at,
                source: dec(&r.source)?,
                entity: entity.to_owned(),
                entity_id: entity_id.to_owned(),
                field: r.field,
                old_value: r.old_value.as_deref().map(dec).transpose()?,
                new_value: r.new_value.as_deref().map(dec).transpose()?,
            })
        })
        .collect()
}
