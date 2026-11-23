//! Whole-row snapshots, so undo can reverse any write to the tables below without a handler per entity.
//! A writer takes a [`snap`] of the rows it is about to change, then calls [`changed`] once it has written;
//! [`created`] covers a new row. Each difference becomes one `row:<table>` change holding the full old and new row.

use serde_json::Value;
use sqlx::SqliteConnection;

use crate::{ChangeSource, Error, Result, history, history::Edit, search};

pub(crate) const TABLES: &[&str] = &[
    "statuses",
    "labels",
    "projects",
    "milestones",
    "cycles",
    "comments",
    "property_defs",
    "relation_kinds",
    "relations",
    "views",
    "view_templates",
    "pins",
    "issue_templates",
    "work_blocks",
    "settings",
    "import_log",
    "events",
    "event_issues",
];

pub(crate) const ENTITY_PREFIX: &str = "row:";

/// The rows a writer is about to change, keyed by primary key.
pub struct Snap {
    table: &'static str,
    rows: Vec<(String, Value)>,
}

async fn names(tx: &mut SqliteConnection, table: &str, pk: bool) -> Result<Vec<String>> {
    let sql = if pk {
        "SELECT name FROM pragma_table_info(?) WHERE pk = 1"
    } else {
        "SELECT name FROM pragma_table_info(?) ORDER BY cid"
    };
    Ok(sqlx::query_scalar(sql).bind(table).fetch_all(tx).await?)
}

fn key_of(v: &Value) -> String {
    v.as_str().map_or_else(|| v.to_string(), str::to_owned)
}

async fn select(
    tx: &mut SqliteConnection,
    table: &str,
    filter: &str,
    bind: &str,
) -> Result<Vec<Value>> {
    let obj = names(tx, table, false)
        .await?
        .iter()
        .map(|c| format!("'{c}', \"{c}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("SELECT json_object({obj}) FROM {table} WHERE {filter} ORDER BY rowid");
    let rows: Vec<String> = sqlx::query_scalar(&sql).bind(bind).fetch_all(tx).await?;
    rows.iter()
        .map(|r| serde_json::from_str(r).map_err(|e| Error::Invalid(e.to_string())))
        .collect()
}

async fn pk(tx: &mut SqliteConnection, table: &str) -> Result<String> {
    names(tx, table, true)
        .await?
        .pop()
        .ok_or_else(|| Error::Invalid(format!("{table} has no single primary key")))
}

async fn current(tx: &mut SqliteConnection, table: &str, key: &str) -> Result<Value> {
    let filter = format!("\"{}\" = ?", pk(tx, table).await?);
    Ok(select(tx, table, &filter, key)
        .await?
        .pop()
        .unwrap_or(Value::Null))
}

impl Snap {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// Rows of `table` matching `filter` (SQL with one `?`, bound to `bind`).
///
/// # Errors
/// Fails when the query fails.
pub async fn snap(
    tx: &mut SqliteConnection,
    table: &'static str,
    filter: &str,
    bind: impl ToString,
) -> Result<Snap> {
    let pk = pk(tx, table).await?;
    let rows = select(tx, table, filter, &bind.to_string()).await?;
    let rows = rows
        .into_iter()
        .map(|r| (key_of(&r[pk.as_str()]), r))
        .collect();
    Ok(Snap { table, rows })
}

/// Every row of `table`.
///
/// # Errors
/// Fails when the query fails.
pub async fn all(tx: &mut SqliteConnection, table: &'static str) -> Result<Snap> {
    snap(tx, table, "? IS NOT NULL", 1).await
}

/// The row with primary key `key`, if it exists.
///
/// # Errors
/// Fails when the query fails.
pub async fn by_key(
    tx: &mut SqliteConnection,
    table: &'static str,
    key: impl ToString,
) -> Result<Snap> {
    let filter = format!("\"{}\" = ?", pk(tx, table).await?);
    snap(tx, table, &filter, key).await
}

async fn log(
    tx: &mut SqliteConnection,
    src: ChangeSource,
    table: &str,
    key: &str,
    old: &Value,
    new: &Value,
) -> Result<()> {
    let edit = Edit {
        field: "row",
        old: (!old.is_null()).then_some(old),
        new: (!new.is_null()).then_some(new),
    };
    history::record(tx, src, &format!("{ENTITY_PREFIX}{table}"), key, &edit).await
}

/// Records how each snapshotted row differs from now. Call it after writing; children before parents, so
/// undo (which runs newest first) restores parents first.
///
/// # Errors
/// Fails when a query or the insert fails.
pub async fn changed(tx: &mut SqliteConnection, src: ChangeSource, before: Snap) -> Result<()> {
    for (key, old) in &before.rows {
        let new = current(tx, before.table, key).await?;
        if new != *old {
            log(tx, src, before.table, key, old, &new).await?;
        }
    }
    Ok(())
}

/// Records a row that did not exist before.
///
/// # Errors
/// Fails when a query or the insert fails.
pub async fn created(
    tx: &mut SqliteConnection,
    src: ChangeSource,
    table: &'static str,
    key: impl ToString,
) -> Result<()> {
    let key = key.to_string();
    let new = current(tx, table, &key).await?;
    log(tx, src, table, &key, &Value::Null, &new).await
}

/// Moves one row from `expect` to `target` (either may be null: absent), refusing when it is not as expected.
pub(crate) async fn apply(
    tx: &mut SqliteConnection,
    src: ChangeSource,
    table: &str,
    key: &str,
    target: &Value,
    expect: &Value,
) -> Result<()> {
    let table = TABLES
        .iter()
        .find(|t| **t == table)
        .ok_or_else(|| Error::Invalid(format!("unknown table {table}")))?;
    let now = current(tx, table, key).await?;
    if now != *expect {
        return Err(Error::Conflict(format!(
            "{table} {key} changed since this step"
        )));
    }
    let pk = pk(tx, table).await?;
    let cols = names(tx, table, false).await?;
    let sql = if target.is_null() {
        format!("DELETE FROM {table} WHERE \"{pk}\" = ?1")
    } else if now.is_null() {
        let names = cols.iter().map(|c| format!("\"{c}\"")).collect::<Vec<_>>();
        let vals = cols
            .iter()
            .map(|c| format!("json_extract(?1, '$.\"{c}\"')"))
            .collect::<Vec<_>>();
        format!(
            "INSERT INTO {table} ({}) VALUES ({})",
            names.join(", "),
            vals.join(", ")
        )
    } else {
        let set = cols
            .iter()
            .map(|c| format!("\"{c}\" = json_extract(?1, '$.\"{c}\"')"))
            .collect::<Vec<_>>();
        format!("UPDATE {table} SET {} WHERE \"{pk}\" = ?2", set.join(", "))
    };
    // Insert needs only the row; delete only the key; update both.
    let q = sqlx::query(&sql);
    let q = if target.is_null() {
        q.bind(key)
    } else if now.is_null() {
        q.bind(target.to_string())
    } else {
        q.bind(target.to_string()).bind(key)
    };
    q.execute(&mut *tx).await?;
    log(tx, src, table, key, expect, target).await?;
    if *table == "comments" {
        let row = if target.is_null() { expect } else { target };
        if let Some(issue) = row["issue_id"].as_i64() {
            search::reindex_in(tx, crate::IssueId(issue)).await?;
        }
    }
    Ok(())
}
