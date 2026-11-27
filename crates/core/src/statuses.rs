use serde_json::json;
use sqlx::SqliteConnection;

use crate::{
    ChangeSource, Db, Error, FlowKind, Result, Status, StatusCategory, StatusId, history,
    palette::{self, Look},
    rows,
};

pub(crate) fn category_str(c: StatusCategory) -> &'static str {
    match c {
        StatusCategory::Backlog => "backlog",
        StatusCategory::Unstarted => "unstarted",
        StatusCategory::Started => "started",
        StatusCategory::Completed => "completed",
        StatusCategory::Canceled => "canceled",
    }
}

pub(crate) fn parse_category(s: &str) -> Result<StatusCategory> {
    Ok(match s {
        "backlog" => StatusCategory::Backlog,
        "unstarted" => StatusCategory::Unstarted,
        "started" => StatusCategory::Started,
        "completed" => StatusCategory::Completed,
        "canceled" => StatusCategory::Canceled,
        other => return Err(Error::Invalid(format!("status category {other}"))),
    })
}

const fn kind_str(k: FlowKind) -> &'static str {
    match k {
        FlowKind::Active => "active",
        FlowKind::Waiting => "waiting",
        FlowKind::Idle => "idle",
    }
}

fn parse_kind(s: &str) -> Result<FlowKind> {
    Ok(match s {
        "active" => FlowKind::Active,
        "waiting" => FlowKind::Waiting,
        "idle" => FlowKind::Idle,
        other => return Err(Error::Invalid(format!("flow kind {other}"))),
    })
}

fn check_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        return Err(Error::Invalid("status name is empty".into()));
    }
    Ok(())
}

fn unique_to_conflict(e: sqlx::Error, name: &str) -> Error {
    if e.as_database_error()
        .is_some_and(sqlx::error::DatabaseError::is_unique_violation)
    {
        return Error::Conflict(format!("status {name} already exists"));
    }
    e.into()
}

async fn fetch(conn: &mut SqliteConnection, id: StatusId) -> Result<Status> {
    let r = sqlx::query!(
        r#"SELECT id AS "id!: i64", name, category, position, color, icon, definition, flow_kind, nudge_days FROM statuses WHERE id = ?"#,
        id.0
    )
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| Error::NotFound(format!("status {}", id.0)))?;
    Ok(Status {
        id: StatusId(r.id),
        name: r.name,
        category: parse_category(&r.category)?,
        position: i32::try_from(r.position).map_err(|e| Error::Invalid(e.to_string()))?,
        color_hex: palette::resolve(&r.color),
        color: r.color,
        icon: r.icon,
        definition: r.definition,
        flow_kind: parse_kind(&r.flow_kind)?,
        nudge_days: r
            .nudge_days
            .map(i32::try_from)
            .transpose()
            .map_err(|e| Error::Invalid(e.to_string()))?,
    })
}

/// # Errors
///
/// Fails on a database error.
pub async fn list(db: &Db) -> Result<Vec<Status>> {
    let ids = sqlx::query_scalar!(r#"SELECT id AS "id!: i64" FROM statuses ORDER BY CASE category WHEN 'backlog' THEN 0 WHEN 'unstarted' THEN 1
           WHEN 'started' THEN 2 WHEN 'completed' THEN 3 ELSE 4 END, position, id"#)
        .fetch_all(&db.pool)
        .await?;
    let mut conn = db.pool.acquire().await?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        out.push(fetch(&mut conn, StatusId(id)).await?);
    }
    Ok(out)
}

/// # Errors
///
/// `Invalid` for an empty name or a color that is not `#rrggbb`, `Conflict` for a duplicate name.
pub async fn create(
    db: &Db,
    name: &str,
    category: StatusCategory,
    color: &str,
    icon: Option<String>,
    src: ChangeSource,
) -> Result<Status> {
    check_name(name)?;
    let color = palette::key_for(color)?;
    if let Some(icon) = &icon {
        palette::check_icon(icon)?;
    }
    let cat = category_str(category);
    let kind = kind_str(if category == StatusCategory::Started {
        FlowKind::Active
    } else {
        FlowKind::Idle
    });
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let id = sqlx::query!(
        "INSERT INTO statuses (name, category, position, color, icon, flow_kind)
         VALUES (?, ?, (SELECT COALESCE(MAX(position), -1) + 1 FROM statuses), ?, ?, ?)",
        name,
        cat,
        color,
        icon,
        kind
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| unique_to_conflict(e, name))?
    .last_insert_rowid();
    let created = json!(name);
    let edit = history::Edit {
        field: "created",
        old: None,
        new: Some(&created),
    };
    history::record(&mut tx, src, "status", &id.to_string(), &edit).await?;
    rows::created(&mut tx, src, "statuses", id).await?;
    let status = fetch(&mut tx, StatusId(id)).await?;
    tx.commit().await?;
    Ok(status)
}

/// # Errors
///
/// `NotFound` for an unknown id, `Invalid` for an empty name or bad color or icon, `Conflict` for a duplicate
/// name. A `None` field keeps its current value.
pub async fn update(
    db: &Db,
    id: StatusId,
    name: Option<String>,
    position: Option<i32>,
    look: Look,
    src: ChangeSource,
) -> Result<Status> {
    let Look { icon, color } = look;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let old = fetch(&mut tx, id).await?;
    let before = rows::by_key(&mut tx, "statuses", id.0).await?;
    if let Some(n) = &name {
        check_name(n)?;
    }
    let color = match color {
        Some(c) => palette::key_for(&c)?.to_owned(),
        None => old.color.clone(),
    };
    if let Some(i) = &icon {
        palette::check_icon(i)?;
    }
    let name = name.unwrap_or(old.name);
    let position = position.unwrap_or(old.position);
    let icon = icon.or_else(|| old.icon.clone());
    sqlx::query!(
        "UPDATE statuses SET name = ?, color = ?, position = ?, icon = ? WHERE id = ?",
        name,
        color,
        position,
        icon,
        id.0
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| unique_to_conflict(e, &name))?;
    rows::changed(&mut tx, src, before).await?;
    for (field, before, after) in [
        ("color", json!(old.color), json!(color)),
        ("icon", json!(old.icon), json!(icon)),
    ] {
        if before != after {
            let edit = history::Edit {
                field,
                old: Some(&before),
                new: Some(&after),
            };
            history::record(&mut tx, src, "status", &id.0.to_string(), &edit).await?;
        }
    }
    let status = fetch(&mut tx, id).await?;
    tx.commit().await?;
    Ok(status)
}

/// Edits the management fields. A `None` keeps the current value; `nudge_days` of 0 clears the nudge.
///
/// # Errors
///
/// `NotFound` for an unknown id, `Invalid` for a negative `nudge_days`.
pub async fn set_flow(
    db: &Db,
    id: StatusId,
    definition: Option<String>,
    kind: Option<FlowKind>,
    nudge_days: Option<i32>,
    src: ChangeSource,
) -> Result<Status> {
    if nudge_days.is_some_and(|n| n < 0) {
        return Err(Error::Invalid("nudge days cannot be negative".into()));
    }
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let old = fetch(&mut tx, id).await?;
    let before = rows::by_key(&mut tx, "statuses", id.0).await?;
    let definition = definition.map_or(old.definition, |d| d.trim().to_owned());
    let kind = kind_str(kind.unwrap_or(old.flow_kind));
    let nudge = nudge_days.map_or(old.nudge_days, |n| (n > 0).then_some(n));
    sqlx::query!(
        "UPDATE statuses SET definition = ?, flow_kind = ?, nudge_days = ? WHERE id = ?",
        definition,
        kind,
        nudge,
        id.0
    )
    .execute(&mut *tx)
    .await?;
    rows::changed(&mut tx, src, before).await?;
    let status = fetch(&mut tx, id).await?;
    tx.commit().await?;
    Ok(status)
}

/// Issues in the deleted status move to `move_issues_to`.
///
/// shortcut: a moved issue keeps its `completed_at` even when the target is in another category,
/// upgrade when statuses can be deleted from the UI across categories.
///
/// # Errors
///
/// `Invalid` when both ids are the same, `NotFound` when either status does not exist.
pub async fn delete(db: &Db, id: StatusId, move_issues_to: StatusId) -> Result<()> {
    if id == move_issues_to {
        return Err(Error::Invalid(
            "cannot move issues into the deleted status".into(),
        ));
    }
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let gone = fetch(&mut tx, id).await?;
    fetch(&mut tx, move_issues_to).await?;
    // Every category keeps a status: the Completed one stamps completed_at, and a category with none has no home.
    let same: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM statuses WHERE category = ?")
        .bind(category_str(gone.category))
        .fetch_one(&mut *tx)
        .await?;
    if same <= 1 {
        return Err(Error::Invalid(format!(
            "cannot delete the last {} status",
            category_str(gone.category)
        )));
    }
    let moved = sqlx::query_scalar!(
        r#"SELECT id AS "id!: i64" FROM issues WHERE status_id = ?"#,
        id.0
    )
    .fetch_all(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE issues SET status_id = ? WHERE status_id = ?",
        move_issues_to.0,
        id.0
    )
    .execute(&mut *tx)
    .await?;
    for issue in moved {
        let (old, new) = (serde_json::json!(id.0), serde_json::json!(move_issues_to.0));
        let edit = history::Edit {
            field: "status_id",
            old: Some(&old),
            new: Some(&new),
        };
        history::record(
            &mut tx,
            ChangeSource::System,
            "issue",
            &issue.to_string(),
            &edit,
        )
        .await?;
    }
    let before = rows::by_key(&mut tx, "statuses", id.0).await?;
    sqlx::query!("DELETE FROM statuses WHERE id = ?", id.0)
        .execute(&mut *tx)
        .await?;
    rows::changed(&mut tx, ChangeSource::System, before).await?;
    tx.commit().await?;
    Ok(())
}

/// Hands the positions the listed statuses already hold to the new order, so other categories keep theirs.
/// One call is one history step; undo restores the old order.
///
/// # Errors
///
/// `Invalid` for an empty list, a duplicate id or ids from more than one category, `NotFound` for an unknown id.
pub async fn reorder(db: &Db, ids_in_order: &[StatusId], src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let mut slots = Vec::with_capacity(ids_in_order.len());
    let mut category = None;
    for id in ids_in_order {
        let s = fetch(&mut tx, *id).await?;
        if *category.get_or_insert(s.category) != s.category {
            return Err(Error::Invalid(
                "reorder lists one category at a time".into(),
            ));
        }
        slots.push(s.position);
    }
    let mut ids: Vec<i64> = ids_in_order.iter().map(|i| i.0).collect();
    ids.sort_unstable();
    ids.dedup();
    if ids.is_empty() || ids.len() != ids_in_order.len() {
        return Err(Error::Invalid("reorder needs distinct status ids".into()));
    }
    slots.sort_unstable();
    let before = rows::all(&mut tx, "statuses").await?;
    for (id, position) in ids_in_order.iter().zip(slots) {
        sqlx::query("UPDATE statuses SET position = ? WHERE id = ?")
            .bind(position)
            .bind(id.0)
            .execute(&mut *tx)
            .await?;
    }
    rows::changed(&mut tx, src, before).await?;
    tx.commit().await?;
    Ok(())
}
