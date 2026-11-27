//! Sidebar folders for pins, views, projects and cycles. An item in a folder carries its id (`folder_id`); a folder
//! carries its section and its parent, so folders nest up to [`MAX_DEPTH`] deep. Deleting a folder moves what it
//! holds up one level, never deletes it.

use std::collections::HashMap;

use serde_json::json;
use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};

use crate::{
    ChangeSource, Db, Error, FolderSection, Placement, Result, ViewFolder, ViewFolderId,
    history::{self, Edit},
    rows,
};

/// The most folders one inside another (folder > folder > folder > item).
pub const MAX_DEPTH: usize = 3;

const COLS: &str = "id, name, position, section, parent_id";

impl FolderSection {
    fn table(self) -> &'static str {
        match self {
            Self::Pin => "pins",
            Self::View => "views",
            Self::Project => "projects",
            Self::Cycle => "cycles",
        }
    }
    fn word(self) -> &'static str {
        match self {
            Self::Pin => "pin",
            Self::View => "view",
            Self::Project => "project",
            Self::Cycle => "cycle",
        }
    }
    fn parse(s: &str) -> Result<Self> {
        [Self::Pin, Self::View, Self::Project, Self::Cycle]
            .into_iter()
            .find(|x| x.word() == s)
            .ok_or_else(|| Error::Invalid(format!("unknown folder section {s}")))
    }
}

fn from_row(r: &SqliteRow) -> Result<ViewFolder> {
    Ok(ViewFolder {
        id: ViewFolderId(r.try_get("id")?),
        name: r.try_get("name")?,
        position: r.try_get("position")?,
        section: FolderSection::parse(&r.try_get::<String, _>("section")?)?,
        parent_id: r.try_get::<Option<i64>, _>("parent_id")?.map(ViewFolderId),
    })
}

fn clean(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::Invalid("a folder needs a name".into()));
    }
    Ok(name)
}

async fn get(tx: &mut SqliteConnection, id: ViewFolderId) -> Result<ViewFolder> {
    let row = sqlx::query(&format!("SELECT {COLS} FROM view_folders WHERE id = ?"))
        .bind(id.0)
        .fetch_optional(tx)
        .await?
        .ok_or_else(|| Error::NotFound(format!("folder {}", id.0)))?;
    from_row(&row)
}

async fn all(tx: &mut SqliteConnection) -> Result<Vec<ViewFolder>> {
    sqlx::query(&format!(
        "SELECT {COLS} FROM view_folders ORDER BY position, id"
    ))
    .fetch_all(tx)
    .await?
    .iter()
    .map(from_row)
    .collect()
}

/// The folder (when given) must exist and belong to `section`.
async fn target(
    tx: &mut SqliteConnection,
    section: FolderSection,
    folder: Option<ViewFolderId>,
) -> Result<Option<ViewFolder>> {
    let Some(id) = folder else { return Ok(None) };
    let f = get(tx, id).await?;
    if f.section != section {
        return Err(Error::Invalid(
            "a folder only holds items of its own section".into(),
        ));
    }
    Ok(Some(f))
}

/// Folders above `id` (not counting it), nearest first.
fn ancestors(folders: &[ViewFolder], id: ViewFolderId) -> Vec<ViewFolderId> {
    let by_id: HashMap<_, _> = folders.iter().map(|f| (f.id, f.parent_id)).collect();
    let mut out = vec![];
    let mut at = by_id.get(&id).copied().flatten();
    while let Some(p) = at {
        if out.contains(&p) {
            break;
        }
        out.push(p);
        at = by_id.get(&p).copied().flatten();
    }
    out
}

/// 1 for a folder with no folders inside, plus the deepest chain below it.
fn height(folders: &[ViewFolder], id: ViewFolderId) -> usize {
    1 + folders
        .iter()
        .filter(|f| f.parent_id == Some(id))
        .map(|f| height(folders, f.id))
        .max()
        .unwrap_or(0)
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

/// Every folder of every section.
///
/// # Errors
///
/// Fails on a database error.
pub async fn list(db: &Db) -> Result<Vec<ViewFolder>> {
    let mut conn = db.pool.acquire().await?;
    all(&mut conn).await
}

/// Where every pin, view, project and cycle sits, in its folder's order.
///
/// # Errors
///
/// Fails on a database error.
pub async fn placements(db: &Db) -> Result<Vec<Placement>> {
    let mut out = vec![];
    for s in [
        FolderSection::Pin,
        FolderSection::View,
        FolderSection::Project,
        FolderSection::Cycle,
    ] {
        let sql = format!(
            "SELECT id, folder_id, position FROM {} ORDER BY position, id",
            s.table()
        );
        for r in sqlx::query(&sql).fetch_all(&db.pool).await? {
            out.push(Placement {
                section: s,
                id: r.try_get("id")?,
                folder_id: r.try_get::<Option<i64>, _>("folder_id")?.map(ViewFolderId),
                position: r.try_get("position")?,
            });
        }
    }
    Ok(out)
}

/// A new folder at the end of `parent` (or of the section's top level).
///
/// # Errors
///
/// Fails with `Invalid` for an empty name, a parent of another section, or nesting deeper than [`MAX_DEPTH`].
pub async fn create(
    db: &Db,
    section: FolderSection,
    parent: Option<ViewFolderId>,
    name: &str,
    src: ChangeSource,
) -> Result<ViewFolder> {
    let name = clean(name)?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    if let Some(p) = target(&mut tx, section, parent).await?
        && ancestors(&all(&mut tx).await?, p.id).len() + 2 > MAX_DEPTH
    {
        return Err(Error::Invalid(format!(
            "folders nest at most {MAX_DEPTH} deep"
        )));
    }
    let id = sqlx::query(
        "INSERT INTO view_folders (name, position, section, parent_id)
         VALUES (?, (SELECT COALESCE(MAX(position) + 1, 0) FROM view_folders), ?, ?)",
    )
    .bind(name)
    .bind(section.word())
    .bind(parent.map(|p| p.0))
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();
    rows::created(&mut tx, src, "view_folders", id).await?;
    note(&mut tx, src, id, "created", None, Some(&json!(name))).await?;
    let f = get(&mut tx, ViewFolderId(id)).await?;
    tx.commit().await?;
    Ok(f)
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
    let old = get(&mut tx, id).await?.name;
    let before = rows::by_key(&mut tx, "view_folders", id.0).await?;
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
    let f = get(&mut tx, id).await?;
    tx.commit().await?;
    Ok(f)
}

/// Moves what the folder holds (items and sub-folders) to `up`.
async fn lift(
    tx: &mut SqliteConnection,
    table: &str,
    id: ViewFolderId,
    up: Option<i64>,
) -> Result<()> {
    for sql in [
        format!("UPDATE {table} SET folder_id = ? WHERE folder_id = ?"),
        "UPDATE view_folders SET parent_id = ? WHERE parent_id = ?".to_string(),
    ] {
        sqlx::query(&sql)
            .bind(up)
            .bind(id.0)
            .execute(&mut *tx)
            .await?;
    }
    Ok(())
}

/// Removes the folder's pin, returning its snapshot so the caller can log it for undo.
async fn drop_pin(tx: &mut SqliteConnection, id: ViewFolderId) -> Result<rows::Snap> {
    let pin = rows::snap(tx, "pins", "kind = 'folder' AND target = ?", id.0).await?;
    sqlx::query("DELETE FROM pins WHERE kind = 'folder' AND target = ?")
        .bind(id.0.to_string())
        .execute(tx)
        .await?;
    Ok(pin)
}

/// Deletes the folder; the items and folders in it move up to its parent (the top level for a top-level folder).
///
/// # Errors
///
/// Fails with `NotFound` for an unknown id, or on a database error.
pub async fn delete(db: &Db, id: ViewFolderId, src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let f = get(&mut tx, id).await?;
    let table = f.section.table();
    let up = f.parent_id.map(|p| p.0);
    let folder = rows::by_key(&mut tx, "view_folders", id.0).await?;
    let kids = rows::snap(&mut tx, "view_folders", "parent_id = ?", id.0).await?;
    let inside = rows::snap(&mut tx, table, "folder_id = ?", id.0).await?;
    let pin = drop_pin(&mut tx, id).await?;
    lift(&mut tx, table, id, up).await?;
    sqlx::query("DELETE FROM view_folders WHERE id = ?")
        .bind(id.0)
        .execute(&mut *tx)
        .await?;
    note(&mut tx, src, id.0, "deleted", None, None).await?;
    // Children before parents, so undo restores the folder before the rows that point at it.
    rows::changed(&mut tx, src, inside).await?;
    rows::changed(&mut tx, src, kids).await?;
    rows::changed(&mut tx, src, folder).await?;
    rows::changed(&mut tx, src, pin).await?;
    tx.commit().await?;
    Ok(())
}

/// Puts `id` at `index` among `siblings` (the end when `None`) and returns the order to write.
fn inserted(mut siblings: Vec<i64>, id: i64, index: Option<u32>) -> Vec<i64> {
    siblings.retain(|s| *s != id);
    let at = index.map_or(siblings.len(), |i| usize::try_from(i).unwrap_or(usize::MAX));
    siblings.insert(at.min(siblings.len()), id);
    siblings
}

/// Moves a folder under `parent` (`None` is the top level of its section) at `index` among its siblings (the end
/// when `None`).
///
/// # Errors
///
/// Fails with `Invalid` when the parent is of another section, is the folder or inside it, or when the result
/// would nest deeper than [`MAX_DEPTH`]; `NotFound` for an unknown folder.
pub async fn place_folder(
    db: &Db,
    id: ViewFolderId,
    parent: Option<ViewFolderId>,
    index: Option<u32>,
    src: ChangeSource,
) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let f = get(&mut tx, id).await?;
    let folders = all(&mut tx).await?;
    if let Some(p) = target(&mut tx, f.section, parent).await? {
        let above = ancestors(&folders, p.id);
        if p.id == id || above.contains(&id) {
            return Err(Error::Invalid("a folder cannot go inside itself".into()));
        }
        if above.len() + 1 + height(&folders, id) > MAX_DEPTH {
            return Err(Error::Invalid(format!(
                "folders nest at most {MAX_DEPTH} deep"
            )));
        }
    }
    let before = rows::all(&mut tx, "view_folders").await?;
    let siblings = folders
        .iter()
        .filter(|x| x.section == f.section && x.parent_id == parent)
        .map(|x| x.id.0)
        .collect();
    sqlx::query("UPDATE view_folders SET parent_id = ? WHERE id = ?")
        .bind(parent.map(|p| p.0))
        .bind(id.0)
        .execute(&mut *tx)
        .await?;
    for (position, sib) in (0_i64..).zip(inserted(siblings, id.0, index)) {
        sqlx::query("UPDATE view_folders SET position = ? WHERE id = ?")
            .bind(position)
            .bind(sib)
            .execute(&mut *tx)
            .await?;
    }
    rows::changed(&mut tx, src, before).await?;
    tx.commit().await?;
    Ok(())
}

/// Moves a pin, view, project or cycle into `folder` (`None` is the section's top level) at `index` among what is
/// already there (the end when `None`).
///
/// # Errors
///
/// Fails with `NotFound` for an unknown item or folder, `Invalid` for a folder of another section.
pub async fn place_item(
    db: &Db,
    section: FolderSection,
    id: i64,
    folder: Option<ViewFolderId>,
    index: Option<u32>,
    src: ChangeSource,
) -> Result<()> {
    let table = section.table();
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    target(&mut tx, section, folder).await?;
    if sqlx::query_scalar::<_, i64>(&format!("SELECT id FROM {table} WHERE id = ?"))
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .is_none()
    {
        return Err(Error::NotFound(format!("{} {id}", section.word())));
    }
    let before = rows::all(&mut tx, table).await?;
    let siblings = sqlx::query_scalar(&format!(
        "SELECT id FROM {table} WHERE folder_id IS ? ORDER BY position, id"
    ))
    .bind(folder.map(|f| f.0))
    .fetch_all(&mut *tx)
    .await?;
    sqlx::query(&format!("UPDATE {table} SET folder_id = ? WHERE id = ?"))
        .bind(folder.map(|f| f.0))
        .bind(id)
        .execute(&mut *tx)
        .await?;
    for (position, sib) in (0_i64..).zip(inserted(siblings, id, index)) {
        sqlx::query(&format!("UPDATE {table} SET position = ? WHERE id = ?"))
            .bind(position)
            .bind(sib)
            .execute(&mut *tx)
            .await?;
    }
    rows::changed(&mut tx, src, before).await?;
    tx.commit().await?;
    Ok(())
}
