use sqlx::Row;
use sqlx::{SqliteConnection, sqlite::SqliteRow};

use crate::{ChangeSource, Db, Error, RelationKind, RelationKindDef, Result, rows};

/// The id a name or an id spelled any way maps to: lower case, runs of other characters as one `_`.
#[must_use]
pub fn slug(s: &str) -> String {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join("_")
}

const COLS: &str = "id, name, forward, reverse, symmetric, built_in, position";

fn def(r: &SqliteRow) -> Result<RelationKindDef> {
    Ok(RelationKindDef {
        id: RelationKind(r.try_get("id")?),
        name: r.try_get("name")?,
        forward: r.try_get("forward")?,
        reverse: r.try_get("reverse")?,
        symmetric: r.try_get("symmetric")?,
        built_in: r.try_get("built_in")?,
        position: r.try_get("position")?,
    })
}

/// # Errors
///
/// Fails on a database error.
pub async fn list(db: &Db) -> Result<Vec<RelationKindDef>> {
    let rows = sqlx::query(&format!(
        "SELECT {COLS} FROM relation_kinds ORDER BY position, id"
    ))
    .fetch_all(&db.pool)
    .await?;
    rows.iter().map(def).collect()
}

/// The stored id for a kind given by id or name, in any case ("Blocks", "caused by").
///
/// # Errors
///
/// `Invalid` when no such kind exists.
pub(crate) async fn resolve(
    tx: &mut SqliteConnection,
    kind: &RelationKind,
) -> Result<RelationKind> {
    let id = slug(&kind.0);
    let found: Option<String> = sqlx::query_scalar("SELECT id FROM relation_kinds WHERE id = ?")
        .bind(&id)
        .fetch_optional(tx)
        .await?;
    found
        .map(RelationKind)
        .ok_or_else(|| Error::Invalid(format!("unknown relation kind {}", kind.0)))
}

fn labels(forward: &str, reverse: &str, symmetric: bool) -> Result<(String, String)> {
    let (f, r) = (forward.trim(), reverse.trim());
    let r = if symmetric { f } else { r };
    if f.is_empty() || r.is_empty() {
        return Err(Error::Invalid("a relation kind needs both labels".into()));
    }
    Ok((f.to_owned(), r.to_owned()))
}

/// # Errors
///
/// `Invalid` for an empty name or label, `Conflict` when a kind with that name exists.
pub async fn create(
    db: &Db,
    name: &str,
    forward: &str,
    reverse: &str,
    symmetric: bool,
    src: ChangeSource,
) -> Result<RelationKindDef> {
    let name = name.trim();
    let id = slug(name);
    if id.is_empty() {
        return Err(Error::Invalid("a relation kind needs a name".into()));
    }
    let (forward, reverse) = labels(forward, reverse, symmetric)?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let taken: i64 =
        sqlx::query_scalar("SELECT count(*) FROM relation_kinds WHERE id = ?1 OR name = ?2")
            .bind(&id)
            .bind(name)
            .fetch_one(&mut *tx)
            .await?;
    if taken > 0 {
        return Err(Error::Conflict(format!(
            "relation kind {name} already exists"
        )));
    }
    sqlx::query(
        "INSERT INTO relation_kinds (id, name, forward, reverse, symmetric, built_in, position)
         VALUES (?, ?, ?, ?, ?, 0, (SELECT COALESCE(MAX(position) + 1, 0) FROM relation_kinds))",
    )
    .bind(&id)
    .bind(name)
    .bind(&forward)
    .bind(&reverse)
    .bind(symmetric)
    .execute(&mut *tx)
    .await?;
    rows::created(&mut tx, src, "relation_kinds", &id).await?;
    let out = fetch(&mut tx, &id).await?;
    tx.commit().await?;
    Ok(out)
}

async fn fetch(tx: &mut SqliteConnection, id: &str) -> Result<RelationKindDef> {
    let row = sqlx::query(&format!("SELECT {COLS} FROM relation_kinds WHERE id = ?"))
        .bind(id)
        .fetch_optional(tx)
        .await?
        .ok_or_else(|| Error::NotFound(format!("relation kind {id}")))?;
    def(&row)
}

/// Renames a custom kind and changes its labels; the id, and so every relation of the kind, stays.
///
/// # Errors
///
/// `NotFound` for an unknown id, `Invalid` for a built-in kind or an empty name or label, `Conflict`
/// when another kind has the name.
pub async fn update(db: &Db, kind: RelationKindDef, src: ChangeSource) -> Result<RelationKindDef> {
    let name = kind.name.trim();
    let (forward, reverse) = labels(&kind.forward, &kind.reverse, kind.symmetric)?;
    if name.is_empty() {
        return Err(Error::Invalid("a relation kind needs a name".into()));
    }
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let old = fetch(&mut tx, &kind.id.0).await?;
    if old.built_in {
        return Err(Error::Invalid(
            "a built-in relation kind cannot be changed".into(),
        ));
    }
    let clash: i64 =
        sqlx::query_scalar("SELECT count(*) FROM relation_kinds WHERE name = ? AND id <> ?")
            .bind(name)
            .bind(&old.id.0)
            .fetch_one(&mut *tx)
            .await?;
    if clash > 0 {
        return Err(Error::Conflict(format!(
            "relation kind {name} already exists"
        )));
    }
    let before = rows::by_key(&mut tx, "relation_kinds", &old.id.0).await?;
    sqlx::query(
        "UPDATE relation_kinds SET name = ?, forward = ?, reverse = ?, symmetric = ? WHERE id = ?",
    )
    .bind(name)
    .bind(&forward)
    .bind(&reverse)
    .bind(kind.symmetric)
    .bind(&old.id.0)
    .execute(&mut *tx)
    .await?;
    rows::changed(&mut tx, src, before).await?;
    let out = fetch(&mut tx, &old.id.0).await?;
    tx.commit().await?;
    Ok(out)
}

/// A kind that relations still use is refused: the owner removes those first, so nothing is lost silently.
///
/// # Errors
///
/// `NotFound` for an unknown id, `Invalid` for a built-in kind, `Conflict` while relations use it.
pub async fn delete(db: &Db, id: &RelationKind, src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let old = fetch(&mut tx, &id.0).await?;
    if old.built_in {
        return Err(Error::Invalid(
            "a built-in relation kind cannot be deleted".into(),
        ));
    }
    let used: i64 = sqlx::query_scalar("SELECT count(*) FROM relations WHERE kind = ?")
        .bind(&id.0)
        .fetch_one(&mut *tx)
        .await?;
    if used > 0 {
        return Err(Error::Conflict(format!(
            "{used} relations use {}; remove them first",
            old.name
        )));
    }
    let before = rows::by_key(&mut tx, "relation_kinds", &id.0).await?;
    sqlx::query("DELETE FROM relation_kinds WHERE id = ?")
        .bind(&id.0)
        .execute(&mut *tx)
        .await?;
    rows::changed(&mut tx, src, before).await?;
    tx.commit().await?;
    Ok(())
}
