use serde_json::{Value, json};

use crate::{
    ChangeSource, Db, Error, IssueId, Label, LabelGroupId, LabelId, Result,
    history::{self, Edit},
    palette, rows,
};

fn label(
    id: i64,
    name: String,
    color: String,
    icon: Option<String>,
    group_id: Option<i64>,
) -> Label {
    Label {
        group_id: group_id.map(LabelGroupId),
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
    let rows = sqlx::query!(
        r#"SELECT id AS "id!: i64", name, color, icon, group_id FROM labels ORDER BY id"#
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| label(r.id, r.name, r.color, r.icon, r.group_id))
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
    Ok(label(id, name.to_owned(), color.to_owned(), icon, None))
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
    let group = label.group_id.map(|g| g.0);
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let before = rows::by_key(&mut tx, "labels", id).await?;
    let old = sqlx::query!(
        "SELECT name, color, icon, group_id FROM labels WHERE id = ?",
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| Error::NotFound(format!("label {id}")))?;
    for (field, before, after) in [
        ("name", json!(old.name), json!(label.name)),
        ("color", json!(old.color), json!(color)),
        ("icon", json!(old.icon), json!(label.icon)),
        ("group", json!(old.group_id), json!(group)),
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
        "UPDATE labels SET name = ?, color = ?, icon = ?, group_id = ? WHERE id = ?",
        label.name,
        color,
        label.icon,
        group,
        id
    )
    .execute(&mut *tx)
    .await?;
    rows::changed(&mut tx, src, before).await?;
    tx.commit().await?;
    Ok(self::label(
        id,
        label.name,
        color.to_owned(),
        label.icon,
        group,
    ))
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
    // In an exclusive group the new label takes the place of the issue's other label from it.
    let swapped: Vec<i64> = sqlx::query_scalar(
        "SELECT il.label_id FROM issue_labels il JOIN labels o ON o.id = il.label_id
         JOIN labels n ON n.group_id = o.group_id JOIN label_groups g ON g.id = n.group_id
         WHERE il.issue_id = ? AND n.id = ? AND o.id != n.id AND g.exclusive = 1",
    )
    .bind(issue.0)
    .bind(label.0)
    .fetch_all(&mut *tx)
    .await?;
    for old in swapped {
        sqlx::query!(
            "DELETE FROM issue_labels WHERE issue_id = ? AND label_id = ?",
            issue.0,
            old
        )
        .execute(&mut *tx)
        .await?;
        let old = json!(old);
        let edit = Edit {
            field: "label",
            old: Some(&old),
            new: None,
        };
        history::record(&mut tx, src, "issue", &issue.0.to_string(), &edit).await?;
    }
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

/// Refuses a label set holding two labels of one exclusive group.
///
/// # Errors
///
/// Fails when two labels share an exclusive group, or on a database error.
pub(crate) async fn check_exclusive(
    conn: &mut sqlx::SqliteConnection,
    ids: &[LabelId],
) -> Result<()> {
    for l in ids {
        let clash: Option<String> = sqlx::query_scalar(
            "SELECT g.name FROM labels a JOIN labels b ON b.group_id = a.group_id AND b.id != a.id
             JOIN label_groups g ON g.id = a.group_id
             WHERE g.exclusive = 1 AND a.id = ? AND b.id IN (SELECT value FROM json_each(?))",
        )
        .bind(l.0)
        .bind(json!(ids.iter().map(|i| i.0).collect::<Vec<_>>()).to_string())
        .fetch_optional(&mut *conn)
        .await?;
        if let Some(g) = clash {
            return Err(Error::Invalid(format!(
                "an issue can hold only one label from the group {g}"
            )));
        }
    }
    Ok(())
}
