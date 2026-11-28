use serde_json::json;

use crate::{
    ChangeSource, Db, Error, LabelGroup, LabelGroupId, Result,
    history::{self, Edit},
    rows,
};

fn group(id: i64, name: String, exclusive: bool) -> LabelGroup {
    LabelGroup {
        id: LabelGroupId(id),
        name,
        exclusive,
    }
}

/// # Errors
///
/// Fails on a database error.
pub async fn list(db: &Db) -> Result<Vec<LabelGroup>> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!: i64", name, exclusive AS "exclusive: bool" FROM label_groups ORDER BY id"#
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| group(r.id, r.name, r.exclusive))
        .collect())
}

/// A new group is not exclusive.
///
/// # Errors
///
/// Fails when the name is empty or taken, or on a database error.
pub async fn create(db: &Db, name: &str, src: ChangeSource) -> Result<LabelGroup> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::Invalid("a label group needs a name".into()));
    }
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let id = sqlx::query!("INSERT INTO label_groups (name) VALUES (?)", name)
        .execute(&mut *tx)
        .await?
        .last_insert_rowid();
    rows::created(&mut tx, src, "label_groups", id).await?;
    let created = json!(name);
    let edit = Edit {
        field: "created",
        old: None,
        new: Some(&created),
    };
    history::record(&mut tx, src, "label_group", &id.to_string(), &edit).await?;
    tx.commit().await?;
    Ok(group(id, name.to_owned(), false))
}

/// Renames a group and sets whether it is exclusive. Labels already holding two of an exclusive group stay as they are.
///
/// # Errors
///
/// Fails when the group is missing, the name is empty or taken, or on a database error.
pub async fn update(db: &Db, g: LabelGroup, src: ChangeSource) -> Result<LabelGroup> {
    let name = g.name.trim().to_owned();
    if name.is_empty() {
        return Err(Error::Invalid("a label group needs a name".into()));
    }
    let id = g.id.0;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let before = rows::by_key(&mut tx, "label_groups", id).await?;
    let old = sqlx::query!(
        r#"SELECT name, exclusive AS "exclusive: bool" FROM label_groups WHERE id = ?"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| Error::NotFound(format!("label group {id}")))?;
    for (field, before, after) in [
        ("name", json!(old.name), json!(name)),
        ("exclusive", json!(old.exclusive), json!(g.exclusive)),
    ] {
        if before != after {
            let edit = Edit {
                field,
                old: Some(&before),
                new: Some(&after),
            };
            history::record(&mut tx, src, "label_group", &id.to_string(), &edit).await?;
        }
    }
    sqlx::query!(
        "UPDATE label_groups SET name = ?, exclusive = ? WHERE id = ?",
        name,
        g.exclusive,
        id
    )
    .execute(&mut *tx)
    .await?;
    rows::changed(&mut tx, src, before).await?;
    tx.commit().await?;
    Ok(group(id, name, g.exclusive))
}

/// # Errors
///
/// Fails when the group is missing, or on a database error.
pub async fn set_exclusive(
    db: &Db,
    id: LabelGroupId,
    exclusive: bool,
    src: ChangeSource,
) -> Result<LabelGroup> {
    let mut g = list(db)
        .await?
        .into_iter()
        .find(|g| g.id == id)
        .ok_or_else(|| Error::NotFound(format!("label group {}", id.0)))?;
    g.exclusive = exclusive;
    update(db, g, src).await
}

/// Its labels stay, outside any group.
///
/// # Errors
///
/// Fails when the group is missing, or on a database error.
pub async fn delete(db: &Db, id: LabelGroupId, src: ChangeSource) -> Result<()> {
    let gid = id.0;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let name: String = sqlx::query_scalar("SELECT name FROM label_groups WHERE id = ?")
        .bind(gid)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| Error::NotFound(format!("label group {gid}")))?;
    let members = rows::snap(&mut tx, "labels", "group_id = ?", gid).await?;
    let before = rows::by_key(&mut tx, "label_groups", gid).await?;
    sqlx::query!("DELETE FROM label_groups WHERE id = ?", gid)
        .execute(&mut *tx)
        .await?;
    rows::changed(&mut tx, src, members).await?;
    rows::changed(&mut tx, src, before).await?;
    let deleted = json!(name);
    let edit = Edit {
        field: "deleted",
        old: Some(&deleted),
        new: None,
    };
    history::record(&mut tx, src, "label_group", &gid.to_string(), &edit).await?;
    tx.commit().await?;
    Ok(())
}
