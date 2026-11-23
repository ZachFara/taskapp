//! Sidebar folders of views. A view in a folder carries its id (`views.folder_id`); deleting a folder moves its
//! views to the top level, never deletes them.

use serde_json::json;
use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};

use crate::{
    ChangeSource, Db, Error, Result, ViewFolder, ViewFolderId, ViewId,
    history::{self, Edit},
    rows,
};

fn from_row(r: &SqliteRow) -> Result<ViewFolder> {
    Ok(ViewFolder {
        id: ViewFolderId(r.try_get("id")?),
        name: r.try_get("name")?,
        position: r.try_get("position")?,
    })
}

fn clean(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::Invalid("a folder needs a name".into()));
    }
    Ok(name)
}

async fn exists(tx: &mut SqliteConnection, id: ViewFolderId) -> Result<()> {
    sqlx::query_scalar::<_, i64>("SELECT id FROM view_folders WHERE id = ?")
        .bind(id.0)
        .fetch_optional(tx)
        .await?
        .map(drop)
        .ok_or_else(|| Error::NotFound(format!("view folder {}", id.0)))
}

async fn note(
    tx: &mut SqliteConnection,
    src: ChangeSource,
    id: i64,
    field: &str,
    old: Option<&serde_json::Value>,
    new: Option<&serde_json::Value>,
) -> Result<()> {
    let edit = Edit { field, old, new };
    history::record(tx, src, "view_folder", &id.to_string(), &edit).await
}

/// # Errors
///
/// Fails on a database error.
pub async fn list(db: &Db) -> Result<Vec<ViewFolder>> {
    sqlx::query("SELECT id, name, position FROM view_folders ORDER BY position, id")
        .fetch_all(&db.pool)
        .await?
        .iter()
        .map(from_row)
        .collect()
}

/// A new folder at the end.
///
/// # Errors
///
/// Fails with `Invalid` for an empty name, or on a database error.
pub async fn create(db: &Db, name: &str, src: ChangeSource) -> Result<ViewFolder> {
    let name = clean(name)?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let id = sqlx::query(
        "INSERT INTO view_folders (name, position)
         VALUES (?, (SELECT COALESCE(MAX(position) + 1, 0) FROM view_folders))",
    )
    .bind(name)
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();
    rows::created(&mut tx, src, "view_folders", id).await?;
    note(&mut tx, src, id, "created", None, Some(&json!(name))).await?;
    let row = sqlx::query("SELECT id, name, position FROM view_folders WHERE id = ?")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    from_row(&row)
}

/// # Errors
///
/// Fails with `NotFound` for an unknown id, `Invalid` for an empty name, or on a database error.
pub async fn rename(
    db: &Db,
    id: ViewFolderId,
    name: &str,
    src: ChangeSource,
) -> Result<ViewFolder> {
    let name = clean(name)?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    exists(&mut tx, id).await?;
    let before = rows::by_key(&mut tx, "view_folders", id.0).await?;
    let old: String = sqlx::query_scalar("SELECT name FROM view_folders WHERE id = ?")
        .bind(id.0)
        .fetch_one(&mut *tx)
        .await?;
    sqlx::query("UPDATE view_folders SET name = ? WHERE id = ?")
        .bind(name)
        .bind(id.0)
        .execute(&mut *tx)
        .await?;
    note(
        &mut tx,
        src,
        id.0,
        "name",
        Some(&json!(old)),
        Some(&json!(name)),
    )
    .await?;
    rows::changed(&mut tx, src, before).await?;
    let row = sqlx::query("SELECT id, name, position FROM view_folders WHERE id = ?")
        .bind(id.0)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    from_row(&row)
}

/// Deletes the folder; its views move to the top level.
///
/// # Errors
///
/// Fails with `NotFound` for an unknown id, or on a database error.
pub async fn delete(db: &Db, id: ViewFolderId, src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    exists(&mut tx, id).await?;
    let folder = rows::by_key(&mut tx, "view_folders", id.0).await?;
    let inside = rows::snap(&mut tx, "views", "folder_id = ?", id.0).await?;
    sqlx::query("UPDATE views SET folder_id = NULL WHERE folder_id = ?")
        .bind(id.0)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM view_folders WHERE id = ?")
        .bind(id.0)
        .execute(&mut *tx)
        .await?;
    note(&mut tx, src, id.0, "deleted", None, None).await?;
    // Children before parents, so undo restores the folder before the views that point at it.
    rows::changed(&mut tx, src, inside).await?;
    rows::changed(&mut tx, src, folder).await?;
    tx.commit().await?;
    Ok(())
}

/// Sets every folder's position to its index in `ids`.
///
/// # Errors
///
/// Fails with `Invalid` unless `ids` lists every folder exactly once.
pub async fn reorder(db: &Db, ids: &[ViewFolderId], src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let mut have: Vec<i64> = sqlx::query_scalar("SELECT id FROM view_folders")
        .fetch_all(&mut *tx)
        .await?;
    let mut want: Vec<i64> = ids.iter().map(|id| id.0).collect();
    have.sort_unstable();
    want.sort_unstable();
    if want != have {
        return Err(Error::Invalid(
            "reorder must list every folder exactly once".into(),
        ));
    }
    let before = rows::all(&mut tx, "view_folders").await?;
    for (position, id) in (0_i64..).zip(ids) {
        sqlx::query("UPDATE view_folders SET position = ? WHERE id = ?")
            .bind(position)
            .bind(id.0)
            .execute(&mut *tx)
            .await?;
    }
    rows::changed(&mut tx, src, before).await?;
    tx.commit().await?;
    Ok(())
}

/// Puts a view at the end of a folder, or at the end of the top level when `folder` is `None`.
///
/// # Errors
///
/// Fails with `NotFound` for an unknown view or folder, or on a database error.
pub async fn move_view(
    db: &Db,
    view: ViewId,
    folder: Option<ViewFolderId>,
    src: ChangeSource,
) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    if let Some(f) = folder {
        exists(&mut tx, f).await?;
    }
    let before = rows::by_key(&mut tx, "views", view.0).await?;
    let done = sqlx::query(
        "UPDATE views SET folder_id = ?, position = (SELECT COALESCE(MAX(position) + 1, 0) FROM views) WHERE id = ?",
    )
    .bind(folder.map(|f| f.0))
    .bind(view.0)
    .execute(&mut *tx)
    .await?;
    if done.rows_affected() == 0 {
        return Err(Error::NotFound(format!("view {}", view.0)));
    }
    rows::changed(&mut tx, src, before).await?;
    tx.commit().await?;
    Ok(())
}
