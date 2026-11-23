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
/// The cached id is trusted only while a change with it exists: an earlier transaction that rolled back
/// left nothing behind, so then the step starts over and clears the redo stack again.
async fn current_step(tx: &mut SqliteConnection) -> Result<Option<(i64, String)>> {
    let Ok((label, id)) = STEP.try_with(|s| (s.label.clone(), s.id.get())) else {
        return Ok(None);
    };
    if let Some(id) = id {
        let live: Option<i64> = sqlx::query_scalar("SELECT 1 FROM changes WHERE step = ? LIMIT 1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
        if live.is_some() {
            return Ok(Some((id, label)));
        }
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
    let is_replay = matches!(source, ChangeSource::Undo | ChangeSource::Redo);
    let joined = if is_replay {
        None
    } else {
        current_step(tx).await?
    };
    if joined.is_none() && !is_replay {
        // A write outside any step is a new edit too: it ends the redo stack like a step does.
        sqlx::query("UPDATE changes SET undone = 2 WHERE undone = 1")
            .execute(&mut *tx)
            .await?;
    }
    let (step, label) = joined.unzip();
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

/// Appends a change dated `at`, outside any step so it can never be undone. For seeding history that
/// predates the database; ordinary writes go through [`record`].
///
/// # Errors
/// Fails when the insert fails.
pub async fn record_past(
    db: &Db,
    at: DateTime<Utc>,
    source: ChangeSource,
    entity: &str,
    entity_id: &str,
    edit: &Edit<'_>,
) -> Result<()> {
    let (source, old, new) = (
        enc(&source)?,
        edit.old.map(enc).transpose()?,
        edit.new.map(enc).transpose()?,
    );
    sqlx::query(
        "INSERT INTO changes (at, source, entity, entity_id, field, old_value, new_value) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(at)
    .bind(source)
    .bind(entity)
    .bind(entity_id)
    .bind(edit.field)
    .bind(old)
    .bind(new)
    .execute(&db.pool)
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

/// Renames the current step once its outcome is known (a created issue's key, an import's count).
/// Does nothing outside a step or before the step recorded a change.
///
/// # Errors
/// Fails when the update fails.
pub async fn relabel(db: &Db, label: &str) -> Result<()> {
    let Ok(Some(id)) = STEP.try_with(|s| s.id.get()) else {
        return Ok(());
    };
    sqlx::query("UPDATE changes SET step_label = ? WHERE step = ?")
        .bind(label)
        .bind(id)
        .execute(&db.pool)
        .await?;
    Ok(())
}
