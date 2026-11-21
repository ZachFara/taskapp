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
    let source = enc(&source)?;
    let old = edit.old.map(enc).transpose()?;
    let new = edit.new.map(enc).transpose()?;
    sqlx::query!(
        "INSERT INTO changes (at, source, entity, entity_id, field, old_value, new_value) VALUES (?, ?, ?, ?, ?, ?, ?)",
        at,
        source,
        entity,
        entity_id,
        edit.field,
        old,
        new
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
