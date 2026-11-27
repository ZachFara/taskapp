use serde_json::{Value, json};

use crate::{
    ChangeSource, Db, Error, IssueId, Label, LabelId, Result,
    history::{self, Edit},
    palette, rows,
};

fn label(id: i64, name: String, color: String, icon: Option<String>) -> Label {
    Label {
        id: LabelId(id),
        name,
        color_hex: palette::resolve(&color),
        color,
        icon,
    }
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn list(db: &Db) -> Result<Vec<Label>> {
    let rows =
        sqlx::query!(r#"SELECT id AS "id!: i64", name, color, icon FROM labels ORDER BY id"#)
            .fetch_all(&db.pool)
            .await?;
    Ok(rows
        .into_iter()
        .map(|r| label(r.id, r.name, r.color, r.icon))
        .collect())
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn create(
    db: &Db,
    name: &str,
    color: &str,
    icon: Option<String>,
    src: ChangeSource,
) -> Result<Label> {
    let color = palette::key_for(color)?;
    if let Some(icon) = &icon {
        palette::check_icon(icon)?;
    }
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let id = sqlx::query!(
        "INSERT INTO labels (name, color, icon) VALUES (?, ?, ?)",
        name,
        color,
        icon
    )
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();
    rows::created(&mut tx, src, "labels", id).await?;
    let created = json!(name);
    let edit = Edit {
        field: "created",
        old: None,
        new: Some(&created),
    };
    history::record(&mut tx, src, "label", &id.to_string(), &edit).await?;
    tx.commit().await?;
    Ok(label(id, name.to_owned(), color.to_owned(), icon))
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn update(db: &Db, label: Label, src: ChangeSource) -> Result<Label> {
    let color = palette::key_for(&label.color)?;
    if let Some(icon) = &label.icon {
        palette::check_icon(icon)?;
    }
    let id = label.id.0;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let before = rows::by_key(&mut tx, "labels", id).await?;
    let old = sqlx::query!("SELECT name, color, icon FROM labels WHERE id = ?", id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| Error::NotFound(format!("label {id}")))?;
    for (field, before, after) in [
        ("name", json!(old.name), json!(label.name)),
        ("color", json!(old.color), json!(color)),
        ("icon", json!(old.icon), json!(label.icon)),
    ] {
        if before != after {
            let edit = Edit {
                field,
                old: Some(&before),
                new: Some(&after),
            };
            history::record(&mut tx, src, "label", &id.to_string(), &edit).await?;
        }
    }
    sqlx::query!(
        "UPDATE labels SET name = ?, color = ?, icon = ? WHERE id = ?",
        label.name,
        color,
        label.icon,
        id
    )
    .execute(&mut *tx)
    .await?;
    rows::changed(&mut tx, src, before).await?;
    tx.commit().await?;
    Ok(self::label(id, label.name, color.to_owned(), label.icon))
}

/// Removing a label takes it off every issue; each removal is recorded on that issue.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn delete(db: &Db, id: LabelId, src: ChangeSource) -> Result<()> {
    let lid = id.0;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let name: String = sqlx::query_scalar("SELECT name FROM labels WHERE id = ?")
        .bind(lid)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| Error::NotFound(format!("label {lid}")))?;
    let issues: Vec<i64> =
        sqlx::query_scalar("SELECT issue_id FROM issue_labels WHERE label_id = ?")
            .bind(lid)
            .fetch_all(&mut *tx)
            .await?;
    for issue_id in issues {
        let old = json!(lid);
        let edit = Edit {
            field: "label",
            old: Some(&old),
            new: None,
        };
        history::record(&mut tx, src, "issue", &issue_id.to_string(), &edit).await?;
    }
    let on_projects = rows::snap(&mut tx, "project_labels", "label_id = ?", lid).await?;
    let before = rows::by_key(&mut tx, "labels", lid).await?;
    sqlx::query!("DELETE FROM labels WHERE id = ?", lid)
        .execute(&mut *tx)
        .await?;
    rows::changed(&mut tx, src, on_projects).await?;
    rows::changed(&mut tx, src, before).await?;
    let deleted = json!(name);
    let edit = Edit {
        field: "deleted",
        old: Some(&deleted),
        new: None,
    };
    history::record(&mut tx, src, "label", &lid.to_string(), &edit).await?;
    tx.commit().await?;
    Ok(())
}

/// Adding a label the issue already has changes nothing and records nothing.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn add_label(db: &Db, issue: IssueId, label: LabelId, src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let added = sqlx::query!(
        "INSERT OR IGNORE INTO issue_labels (issue_id, label_id) VALUES (?, ?)",
        issue.0,
        label.0
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if added == 1 {
        let new = json!(label.0);
        let edit = Edit {
            field: "label",
            old: None,
            new: Some(&new),
        };
        history::record(&mut tx, src, "issue", &issue.0.to_string(), &edit).await?;
    }
    tx.commit().await?;
    Ok(())
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn remove_label(
    db: &Db,
    issue: IssueId,
    label: LabelId,
    src: ChangeSource,
) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let removed = sqlx::query!(
        "DELETE FROM issue_labels WHERE issue_id = ? AND label_id = ?",
        issue.0,
        label.0
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if removed == 1 {
        let old: Value = json!(label.0);
        let edit = Edit {
            field: "label",
            old: Some(&old),
            new: None,
        };
        history::record(&mut tx, src, "issue", &issue.0.to_string(), &edit).await?;
    }
    tx.commit().await?;
    Ok(())
}
