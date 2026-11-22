//! View templates: saved view definitions that new views start from. A view made from one remembers it
//! (`template_id`) so it can be reset to it.

use serde_json::json;
use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};

use crate::{
    ChangeSource, Db, Error, FieldRef, Filter, FixedField, Layout, Op, Result, ViewDef, ViewId,
    ViewTemplate, ViewTemplateId,
    history::{self, Edit},
    palette::{self, Look},
    rows, settings, views,
};

const COLS: &str = "id, name, definition, position, icon, color";

fn from_row(r: &SqliteRow) -> Result<ViewTemplate> {
    let def: String = r.try_get("definition")?;
    Ok(ViewTemplate {
        id: ViewTemplateId(r.try_get("id")?),
        name: r.try_get("name")?,
        icon: r.try_get("icon")?,
        color: r.try_get("color")?,
        def: serde_json::from_str(&def)
            .map_err(|e| Error::Invalid(format!("view template: {e}")))?,
        position: r.try_get("position")?,
    })
}

async fn get(tx: &mut SqliteConnection, id: ViewTemplateId) -> Result<ViewTemplate> {
    let row = sqlx::query(&format!("SELECT {COLS} FROM view_templates WHERE id = ?"))
        .bind(id.0)
        .fetch_optional(tx)
        .await?
        .ok_or_else(|| Error::NotFound(format!("view template {}", id.0)))?;
    from_row(&row)
}

/// # Errors
///
/// Fails on a database error or a stored definition that no longer parses.
pub async fn list(db: &Db) -> Result<Vec<ViewTemplate>> {
    sqlx::query(&format!(
        "SELECT {COLS} FROM view_templates ORDER BY position, id"
    ))
    .fetch_all(&db.pool)
    .await?
    .iter()
    .map(from_row)
    .collect()
}

fn look(icon: Option<String>, color: Option<&str>) -> Result<Look> {
    if let Some(i) = &icon {
        palette::check_icon(i)?;
    }
    let color = color.map(palette::key_for).transpose()?.map(str::to_owned);
    Ok(Look { icon, color })
}

fn json_of(def: &ViewDef) -> Result<String> {
    serde_json::to_string(def).map_err(|e| Error::Invalid(e.to_string()))
}

async fn insert(
    tx: &mut SqliteConnection,
    src: ChangeSource,
    name: &str,
    def: &ViewDef,
    look: &Look,
) -> Result<ViewTemplate> {
    let id = sqlx::query(
        "INSERT INTO view_templates (name, definition, position, icon, color)
         VALUES (?, ?, (SELECT COALESCE(MAX(position) + 1, 0) FROM view_templates), ?, ?)",
    )
    .bind(name)
    .bind(json_of(def)?)
    .bind(look.icon.as_deref())
    .bind(look.color.as_deref())
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();
    rows::created(tx, src, "view_templates", id).await?;
    let created = json!(name);
    let edit = Edit {
        field: "created",
        old: None,
        new: Some(&created),
    };
    history::record(tx, src, "view_template", &id.to_string(), &edit).await?;
    get(tx, ViewTemplateId(id)).await
}

/// # Errors
///
/// Fails with `Invalid` for a bad icon or color, or on a database error.
pub async fn create(
    db: &Db,
    name: &str,
    def: ViewDef,
    icon: Option<String>,
    color: Option<String>,
    src: ChangeSource,
) -> Result<ViewTemplate> {
    let look = look(icon, color.as_deref())?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let t = insert(&mut tx, src, name, &def, &look).await?;
    tx.commit().await?;
    Ok(t)
}

/// Replaces name, icon, color and definition. Position is kept.
///
/// # Errors
///
/// Fails with `NotFound` for an unknown id, `Invalid` for a bad icon or color, or on a database error.
pub async fn update(db: &Db, t: ViewTemplate, src: ChangeSource) -> Result<ViewTemplate> {
    let look = look(t.icon, t.color.as_deref())?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    get(&mut tx, t.id).await?;
    let before = rows::by_key(&mut tx, "view_templates", t.id.0).await?;
    sqlx::query(
        "UPDATE view_templates SET name = ?, definition = ?, icon = ?, color = ? WHERE id = ?",
    )
    .bind(&t.name)
    .bind(json_of(&t.def)?)
    .bind(look.icon.as_deref())
    .bind(look.color.as_deref())
    .bind(t.id.0)
    .execute(&mut *tx)
    .await?;
    rows::changed(&mut tx, src, before).await?;
    let out = get(&mut tx, t.id).await?;
    tx.commit().await?;
    Ok(out)
}

/// Deletes the template, unlinks the views made from it and clears it as the default.
///
/// # Errors
///
/// Fails with `NotFound` for an unknown id, or on a database error.
pub async fn delete(db: &Db, id: ViewTemplateId, src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    get(&mut tx, id).await?;
    let template = rows::by_key(&mut tx, "view_templates", id.0).await?;
    let linked = rows::snap(&mut tx, "views", "template_id = ?", id.0).await?;
    sqlx::query("UPDATE views SET template_id = NULL WHERE template_id = ?")
        .bind(id.0)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM view_templates WHERE id = ?")
        .bind(id.0)
        .execute(&mut *tx)
        .await?;
    let default = rows::by_key(&mut tx, "settings", "default_view_template").await?;
    sqlx::query("DELETE FROM settings WHERE key = 'default_view_template' AND value = ?")
        .bind(id.0.to_string())
        .execute(&mut *tx)
        .await?;
    // Children before parents, so undo restores the template before the views that point at it.
    rows::changed(&mut tx, src, default).await?;
    rows::changed(&mut tx, src, linked).await?;
    rows::changed(&mut tx, src, template).await?;
    tx.commit().await?;
    Ok(())
}

/// Tags a view with its template, as part of the step that made or changed it.
async fn link(
    db: &Db,
    view: ViewId,
    template: Option<ViewTemplateId>,
    src: ChangeSource,
) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let before = rows::by_key(&mut tx, "views", view.0).await?;
    sqlx::query("UPDATE views SET template_id = ? WHERE id = ?")
        .bind(template.map(|t| t.0))
        .bind(view.0)
        .execute(&mut *tx)
        .await?;
    rows::changed(&mut tx, src, before).await?;
    tx.commit().await?;
    Ok(())
}

async fn view(db: &Db, id: ViewId) -> Result<crate::SavedView> {
    views::list(db)
        .await?
        .into_iter()
        .find(|v| v.id == id)
        .ok_or_else(|| Error::NotFound(format!("view {}", id.0)))
}

/// A new view with the template's whole definition, icon and color, named `name` or after the template.
///
/// # Errors
///
/// Fails with `NotFound` for an unknown template, or on a database error.
pub async fn create_view(
    db: &Db,
    template: ViewTemplateId,
    name: Option<&str>,
    src: ChangeSource,
) -> Result<crate::SavedView> {
    let t = get(&mut *db.pool.acquire().await?, template).await?;
    let look = Look {
        icon: t.icon,
        color: t.color,
    };
    let v = views::save(db, None, name.unwrap_or(&t.name), t.def, look, src).await?;
    link(db, v.id, Some(template), src).await?;
    view(db, v.id).await
}

/// Puts a view's definition back to its template's, keeping its name, icon and color.
///
/// # Errors
///
/// Fails with `Invalid` when the view has no template, `NotFound` for an unknown view or template.
pub async fn reset_view(db: &Db, id: ViewId, src: ChangeSource) -> Result<crate::SavedView> {
    let v = view(db, id).await?;
    let template = v
        .template_id
        .ok_or_else(|| Error::Invalid("this view was not made from a template".into()))?;
    let t = get(&mut *db.pool.acquire().await?, template).await?;
    views::save(db, Some(id), &v.name, t.def, Look::default(), src).await
}

/// A template with the view's definition, icon and color, named `name` or after the view.
///
/// # Errors
///
/// Fails with `NotFound` for an unknown view, or on a database error.
pub async fn from_view(
    db: &Db,
    id: ViewId,
    name: Option<&str>,
    src: ChangeSource,
) -> Result<ViewTemplate> {
    let v = view(db, id).await?;
    create(db, name.unwrap_or(&v.name), v.def, v.icon, v.color, src).await
}

/// The three starting templates; `property_keys` are the custom properties the planning table shows.
///
/// shortcut: the "This week" cutoff is fixed when the template is made, like the seeded view; upgrade when
/// saved filters get relative dates.
fn starters(property_keys: &[String]) -> [(&'static str, &'static str, ViewDef); 3] {
    let fixed = FieldRef::Fixed;
    let mut table = vec![
        fixed(FixedField::Priority),
        fixed(FixedField::Status),
        fixed(FixedField::Project),
        fixed(FixedField::Estimate),
        fixed(FixedField::DueDate),
    ];
    table.extend(property_keys.iter().cloned().map(FieldRef::Property));
    let week = chrono::Utc::now().date_naive() + chrono::Days::new(7);
    [
        (
            "Status board",
            "icon:board",
            ViewDef {
                layout: Layout::Board,
                group_by: Some(fixed(FixedField::Status)),
                card_fields: vec![
                    fixed(FixedField::Priority),
                    fixed(FixedField::Label),
                    fixed(FixedField::DueDate),
                ],
                ..ViewDef::default()
            },
        ),
        (
            "Planning table",
            "icon:list",
            ViewDef {
                layout: Layout::Table,
                columns: table,
                ..ViewDef::default()
            },
        ),
        (
            "This week",
            "icon:calendar",
            ViewDef {
                filter: Some(Filter::Cond {
                    field: fixed(FixedField::DueDate),
                    op: Op::Lte,
                    value: json!(week.to_string()),
                }),
                group_by: Some(fixed(FixedField::Project)),
                ..ViewDef::default()
            },
        ),
    ]
}

/// Adds the starting templates to a workspace that has none and has never run setup.
pub(crate) async fn seed(db: &Db, property_keys: &[String]) -> Result<()> {
    if settings::get::<bool>(db, "setup_done")
        .await?
        .unwrap_or(false)
        || !list(db).await?.is_empty()
    {
        return Ok(());
    }
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    for (name, icon, def) in starters(property_keys) {
        sqlx::query(
            "INSERT INTO view_templates (name, definition, position, icon, color)
             VALUES (?, ?, (SELECT COALESCE(MAX(position) + 1, 0) FROM view_templates), ?, NULL)",
        )
        .bind(name)
        .bind(json_of(&def)?)
        .bind(icon)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}
