use chrono::{NaiveDate, Utc};
use serde_json::{Map, Value, json};
use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};

use crate::{
    ChangeSource, Db, Error, Issue, IssueId, ProjectId, PropertyAppliesTo, PropertyDef,
    PropertyKind, Result, SelectOption, ViewDef,
    history::{self, Edit},
    issues, rows, views,
};

fn enc<T: serde::Serialize>(v: &T) -> Result<String> {
    serde_json::to_string(v).map_err(|e| Error::Invalid(e.to_string()))
}

fn dec<T: serde::de::DeserializeOwned>(s: &str) -> Result<T> {
    serde_json::from_str(s).map_err(|e| Error::Invalid(e.to_string()))
}

fn valid_key(key: &str) -> bool {
    let b = key.as_bytes();
    (1..=40).contains(&b.len())
        && b[0].is_ascii_lowercase()
        && b[1..]
            .iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'_')
}

fn def_from_row(r: &SqliteRow) -> Result<PropertyDef> {
    Ok(PropertyDef {
        key: r.get("key"),
        name: r.get("name"),
        kind: dec(r.get("kind"))?,
        options: dec(r.get("options"))?,
        project_id: r.get::<Option<i64>, _>("project_id").map(ProjectId),
        position: r.get("position"),
        applies_to: match r.get::<String, _>("applies_to").as_str() {
            "projects" => PropertyAppliesTo::Projects,
            "both" => PropertyAppliesTo::Both,
            _ => PropertyAppliesTo::Issues,
        },
    })
}

fn applies_str(a: PropertyAppliesTo) -> &'static str {
    match a {
        PropertyAppliesTo::Issues => "issues",
        PropertyAppliesTo::Projects => "projects",
        PropertyAppliesTo::Both => "both",
    }
}

fn check_scope(applies_to: PropertyAppliesTo, project: Option<ProjectId>) -> Result<()> {
    if applies_to == PropertyAppliesTo::Projects && project.is_some() {
        return Err(Error::Invalid(
            "a projects-only property cannot be limited to one project".into(),
        ));
    }
    Ok(())
}

async fn record_def(
    tx: &mut SqliteConnection,
    src: ChangeSource,
    key: &str,
    field: &str,
    old: Option<&Value>,
    new: Option<&Value>,
) -> Result<()> {
    history::record(tx, src, "property_def", key, &Edit { field, old, new }).await
}

pub(crate) async fn get_def(conn: &mut SqliteConnection, key: &str) -> Result<PropertyDef> {
    let row = sqlx::query("SELECT * FROM property_defs WHERE key = ?")
        .bind(key)
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| Error::NotFound(format!("property {key}")))?;
    def_from_row(&row)
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn list_defs(db: &Db) -> Result<Vec<PropertyDef>> {
    sqlx::query("SELECT * FROM property_defs ORDER BY position, key")
        .fetch_all(&db.pool)
        .await?
        .iter()
        .map(def_from_row)
        .collect()
}

/// The stored position is the next free one; `def.position` is ignored.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn create_def(db: &Db, def: PropertyDef, src: ChangeSource) -> Result<PropertyDef> {
    if !valid_key(&def.key) {
        return Err(Error::Invalid(format!(
            "property key {:?} must match ^[a-z][a-z0-9_]{{0,39}}$",
            def.key
        )));
    }
    if def.name.trim().is_empty() {
        return Err(Error::Invalid("property name is empty".into()));
    }
    check_options(def.kind, &def.options)?;
    check_scope(def.applies_to, def.project_id)?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    if get_def(&mut tx, &def.key).await.is_ok() {
        return Err(Error::Conflict(format!("property {} exists", def.key)));
    }
    if let Some(p) = def.project_id {
        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM projects WHERE id = ?")
            .bind(p.0)
            .fetch_one(&mut *tx)
            .await?;
        if n == 0 {
            return Err(Error::NotFound(format!("project {}", p.0)));
        }
    }
    let position: i32 =
        sqlx::query_scalar("SELECT coalesce(max(position) + 1, 0) FROM property_defs")
            .fetch_one(&mut *tx)
            .await?;
    sqlx::query(
        "INSERT INTO property_defs (key, name, kind, options, project_id, position, applies_to) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&def.key)
    .bind(&def.name)
    .bind(enc(&def.kind)?)
    .bind(enc(&def.options)?)
    .bind(def.project_id.map(|p| p.0))
    .bind(position)
    .bind(applies_str(def.applies_to))
    .execute(&mut *tx)
    .await?;
    rows::created(&mut tx, src, "property_defs", &def.key).await?;
    record_def(
        &mut tx,
        src,
        &def.key,
        "created",
        None,
        Some(&json!(def.name)),
    )
    .await?;
    tx.commit().await?;
    Ok(PropertyDef { position, ..def })
}

fn check_options(kind: PropertyKind, options: &[SelectOption]) -> Result<()> {
    let selectish = matches!(kind, PropertyKind::Select | PropertyKind::MultiSelect);
    if !selectish && !options.is_empty() {
        return Err(Error::Invalid("only select kinds take options".into()));
    }
    for (i, o) in options.iter().enumerate() {
        if options[..i].iter().any(|p| p.value == o.value) {
            return Err(Error::Invalid(format!("duplicate option {:?}", o.value)));
        }
    }
    Ok(())
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn rename_def(
    db: &Db,
    key: &str,
    new_name: &str,
    src: ChangeSource,
) -> Result<PropertyDef> {
    if new_name.trim().is_empty() {
        return Err(Error::Invalid("property name is empty".into()));
    }
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let def = get_def(&mut tx, key).await?;
    let before_row = rows::by_key(&mut tx, "property_defs", key).await?;
    sqlx::query("UPDATE property_defs SET name = ? WHERE key = ?")
        .bind(new_name)
        .bind(key)
        .execute(&mut *tx)
        .await?;
    rows::changed(&mut tx, src, before_row).await?;
    let (before, after) = (json!(def.name), json!(new_name));
    record_def(&mut tx, src, key, "name", Some(&before), Some(&after)).await?;
    tx.commit().await?;
    Ok(PropertyDef {
        name: new_name.to_owned(),
        ..def
    })
}

fn stringify(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(stringify).collect::<Vec<_>>().join(", "),
        other => other.to_string(),
    }
}

/// `Ok(None)` drops the value (an empty multi-select has nothing to keep as a single select).
fn convert(from: PropertyKind, to: PropertyKind, v: &Value) -> Result<Option<Value>> {
    use PropertyKind::{MultiSelect, Number, Select, Text};
    Ok(Some(match (from, to) {
        (_, Text) => Value::String(stringify(v)),
        (Text, Number) => {
            let s = v.as_str().unwrap_or_default().trim();
            match (s.parse::<i64>(), s.parse::<f64>()) {
                (Ok(i), _) => json!(i),
                (_, Ok(f)) if f.is_finite() => json!(f),
                _ => return Err(Error::Invalid(format!("{s:?} is not a number"))),
            }
        }
        (Select, MultiSelect) => Value::Array(vec![v.clone()]),
        (MultiSelect, Select) => match v.as_array().map(Vec::as_slice) {
            Some([]) => return Ok(None),
            Some([one]) => one.clone(),
            _ => return Err(Error::Invalid("more than one value".into())),
        },
        _ => v.clone(),
    }))
}

/// One issue's converted `properties` JSON and the value before and after.
struct Conversion {
    id: i64,
    props: String,
    old: Value,
    new: Option<Value>,
}

/// New `properties` JSON per issue that holds `key`; refuses, writing nothing, if any value cannot convert.
async fn convert_all(
    tx: &mut SqliteConnection,
    key: &str,
    old: &PropertyDef,
    new: &PropertyDef,
) -> Result<Vec<Conversion>> {
    let sql = format!(
        "SELECT {} FROM issues WHERE properties != '{{}}' ORDER BY id",
        issues::ISSUE_COLS
    );
    let rows = sqlx::query(&sql).fetch_all(&mut *tx).await?;
    let prefix = issues::prefix(tx).await?;
    let mut writes = Vec::new();
    let mut bad = Vec::new();
    for r in &rows {
        let mut props: Map<String, Value> = dec(r.get("properties"))?;
        let Some(v) = props.get(key).cloned() else {
            continue;
        };
        let Ok(converted) = convert(old.kind, new.kind, &v).and_then(|c| match c {
            Some(c) => validate(new, &c).map(|()| Some(c)),
            None => Ok(None),
        }) else {
            bad.push(issues::issue_from_row(r, &prefix)?.key);
            continue;
        };
        match &converted {
            Some(c) => props.insert(key.to_owned(), c.clone()),
            None => props.remove(key),
        };
        writes.push(Conversion {
            id: r.get("id"),
            props: enc(&props)?,
            old: v,
            new: converted,
        });
    }
    if !bad.is_empty() {
        let more = bad.len().saturating_sub(20);
        bad.truncate(20);
        return Err(Error::Conflict(format!(
            "cannot convert {key} to {:?} on {}{}",
            new.kind,
            bad.join(", "),
            if more > 0 {
                format!(" and {more} more")
            } else {
                String::new()
            }
        )));
    }
    Ok(writes)
}

/// Sets every property's position to its index in `keys`.
///
/// # Errors
///
/// Fails with `Invalid` unless `keys` lists every property exactly once.
pub async fn reorder(db: &Db, keys: &[String]) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let mut have: Vec<String> = sqlx::query_scalar("SELECT key FROM property_defs")
        .fetch_all(&mut *tx)
        .await?;
    let mut want = keys.to_vec();
    have.sort_unstable();
    want.sort_unstable();
    if want != have {
        return Err(Error::Invalid(
            "reorder must list every property exactly once".into(),
        ));
    }
    let before = rows::all(&mut tx, "property_defs").await?;
    for (position, key) in (0_i64..).zip(keys) {
        sqlx::query("UPDATE property_defs SET position = ? WHERE key = ?")
            .bind(position)
            .bind(key)
            .execute(&mut *tx)
            .await?;
    }
    rows::changed(&mut tx, ChangeSource::Ui, before).await?;
    tx.commit().await?;
    Ok(())
}

/// Limits the property to one project, or to all projects when `project` is `None`. Refused with
/// `Conflict`, changing nothing, while an issue outside the new project holds a value.
///
/// # Errors
///
/// Fails with `NotFound` for an unknown key, `Conflict` as above, or on a database error.
pub async fn set_scope(
    db: &Db,
    key: &str,
    project: Option<ProjectId>,
    src: ChangeSource,
) -> Result<PropertyDef> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let old = get_def(&mut tx, key).await?;
    check_scope(old.applies_to, project)?;
    if let Some(p) = project {
        let sql = format!(
            "SELECT {} FROM issues WHERE json_type(properties, ?1) IS NOT NULL
             AND project_id IS NOT ?2 ORDER BY id",
            issues::ISSUE_COLS
        );
        let rows = sqlx::query(&sql)
            .bind(format!("$.{key}"))
            .bind(p.0)
            .fetch_all(&mut *tx)
            .await?;
        if !rows.is_empty() {
            let prefix = issues::prefix(&mut tx).await?;
            let mut keys = rows
                .iter()
                .map(|r| issues::issue_from_row(r, &prefix).map(|i| i.key))
                .collect::<Result<Vec<_>>>()?;
            let more = keys.len().saturating_sub(20);
            keys.truncate(20);
            return Err(Error::Conflict(format!(
                "{key} has values on issues outside the project: {}{}",
                keys.join(", "),
                if more > 0 {
                    format!(" and {more} more")
                } else {
                    String::new()
                }
            )));
        }
    }
    let before_row = rows::by_key(&mut tx, "property_defs", key).await?;
    sqlx::query("UPDATE property_defs SET project_id = ? WHERE key = ?")
        .bind(project.map(|p| p.0))
        .bind(key)
        .execute(&mut *tx)
        .await?;
    rows::changed(&mut tx, src, before_row).await?;
    let (before, after) = (json!(old.project_id), json!(project));
    record_def(&mut tx, src, key, "project_id", Some(&before), Some(&after)).await?;
    tx.commit().await?;
    Ok(PropertyDef {
        project_id: project,
        ..old
    })
}

/// Chooses whether the property applies to issues, projects or both. Refused with `Conflict`, changing
/// nothing, while an entity the property would stop applying to holds a value. A projects-only property
/// loses any single-project limit.
///
/// # Errors
///
/// Fails with `NotFound` for an unknown key, `Conflict` as above, or on a database error.
pub async fn set_applies_to(
    db: &Db,
    key: &str,
    applies_to: PropertyAppliesTo,
    src: ChangeSource,
) -> Result<PropertyDef> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let old = get_def(&mut tx, key).await?;
    let path = format!("$.\"{key}\"");
    for (table, noun, drops) in [
        (
            "issues",
            "issues",
            applies_to == PropertyAppliesTo::Projects,
        ),
        (
            "projects",
            "projects",
            applies_to == PropertyAppliesTo::Issues,
        ),
    ] {
        if !drops {
            continue;
        }
        let held: i64 = sqlx::query_scalar(&format!(
            "SELECT count(*) FROM {table} WHERE json_type(properties, ?) IS NOT NULL"
        ))
        .bind(&path)
        .fetch_one(&mut *tx)
        .await?;
        if held > 0 {
            return Err(Error::Conflict(format!(
                "{key} has values on {held} {noun}"
            )));
        }
    }
    let project = if applies_to == PropertyAppliesTo::Projects {
        None
    } else {
        old.project_id
    };
    let before_row = rows::by_key(&mut tx, "property_defs", key).await?;
    sqlx::query("UPDATE property_defs SET applies_to = ?, project_id = ? WHERE key = ?")
        .bind(applies_str(applies_to))
        .bind(project.map(|p| p.0))
        .bind(key)
        .execute(&mut *tx)
        .await?;
    rows::changed(&mut tx, src, before_row).await?;
    let (before, after) = (json!(old.applies_to), json!(applies_to));
    record_def(&mut tx, src, key, "applies_to", Some(&before), Some(&after)).await?;
    tx.commit().await?;
    Ok(PropertyDef {
        applies_to,
        project_id: project,
        ..old
    })
}

/// Rewrites the property's value on every project to the new kind; refuses, writing nothing, if one cannot convert.
async fn convert_projects(
    tx: &mut SqliteConnection,
    src: ChangeSource,
    key: &str,
    old: &PropertyDef,
    new: &PropertyDef,
) -> Result<()> {
    let path = format!("$.\"{key}\"");
    let before = rows::snap(
        tx,
        "projects",
        "json_type(properties, ?) IS NOT NULL",
        &path,
    )
    .await?;
    let held: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT id, slug, properties FROM projects WHERE json_type(properties, ?) IS NOT NULL ORDER BY id",
    )
    .bind(&path)
    .fetch_all(&mut *tx)
    .await?;
    let mut bad = Vec::new();
    let mut writes = Vec::new();
    for (id, slug, props) in held {
        let mut props: Map<String, Value> = dec(&props)?;
        let v = props.get(key).cloned().unwrap_or(Value::Null);
        let converted = convert(old.kind, new.kind, &v).and_then(|c| match c {
            Some(c) => validate(new, &c).map(|()| Some(c)),
            None => Ok(None),
        });
        match converted {
            Ok(Some(c)) => props.insert(key.to_owned(), c),
            Ok(None) => props.remove(key),
            Err(_) => {
                bad.push(slug);
                continue;
            }
        };
        writes.push((id, enc(&props)?));
    }
    if !bad.is_empty() {
        return Err(Error::Conflict(format!(
            "cannot convert {key} to {:?} on projects {}",
            new.kind,
            bad.join(", ")
        )));
    }
    for (id, props) in writes {
        sqlx::query("UPDATE projects SET properties = ? WHERE id = ?")
            .bind(props)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    rows::changed(tx, src, before).await
}

/// Converts every stored value to the new kind. If any value cannot convert, nothing changes and the
/// error lists up to 20 issue keys.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn change_kind(
    db: &Db,
    key: &str,
    kind: PropertyKind,
    options: Vec<SelectOption>,
    src: ChangeSource,
) -> Result<PropertyDef> {
    check_options(kind, &options)?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let old = get_def(&mut tx, key).await?;
    let new = PropertyDef {
        kind,
        options,
        ..old.clone()
    };
    let writes = convert_all(&mut tx, key, &old, &new).await?;
    let now = Utc::now();
    let field = format!("properties.{key}");
    for w in writes {
        sqlx::query("UPDATE issues SET properties = ?, updated_at = ? WHERE id = ?")
            .bind(w.props)
            .bind(now)
            .bind(w.id)
            .execute(&mut *tx)
            .await?;
        let edit = Edit {
            field: &field,
            old: Some(&w.old),
            new: w.new.as_ref(),
        };
        history::record(&mut tx, src, "issue", &w.id.to_string(), &edit).await?;
    }
    convert_projects(&mut tx, src, key, &old, &new).await?;
    let before_row = rows::by_key(&mut tx, "property_defs", key).await?;
    sqlx::query("UPDATE property_defs SET kind = ?, options = ? WHERE key = ?")
        .bind(enc(&new.kind)?)
        .bind(enc(&new.options)?)
        .bind(key)
        .execute(&mut *tx)
        .await?;
    rows::changed(&mut tx, src, before_row).await?;
    let (before, after) = (json!(old.kind), json!(new.kind));
    record_def(&mut tx, src, key, "kind", Some(&before), Some(&after)).await?;
    if old.options != new.options {
        let (before, after) = (json!(old.options), json!(new.options));
        record_def(&mut tx, src, key, "options", Some(&before), Some(&after)).await?;
    }
    tx.commit().await?;
    Ok(new)
}

/// Removes the property from every saved view and view template: filter, sorts, columns, card fields, grouping.
/// Returns both tables as they were.
async fn strip_from_views(
    tx: &mut SqliteConnection,
    key: &str,
) -> Result<(rows::Snap, rows::Snap)> {
    let before = (
        rows::all(tx, "views").await?,
        rows::all(tx, "view_templates").await?,
    );
    for (table, col) in [("views", "definition"), ("view_templates", "definition")] {
        let all = sqlx::query(&format!("SELECT id, {col} FROM {table}"))
            .fetch_all(&mut *tx)
            .await?;
        for r in all {
            let mut def: ViewDef = dec(r.get(col))?;
            let old = def.clone();
            views::scrub(&mut def, &|k| k == key);
            if def != old {
                sqlx::query(&format!("UPDATE {table} SET {col} = ? WHERE id = ?"))
                    .bind(enc(&def)?)
                    .bind(r.get::<i64, _>("id"))
                    .execute(&mut *tx)
                    .await?;
            }
        }
    }
    Ok(before)
}

/// Drops the property from every issue that holds a value, recording each.
async fn strip_from_issues(tx: &mut SqliteConnection, src: ChangeSource, key: &str) -> Result<()> {
    let path = format!("$.\"{key}\"");
    let lost: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, properties -> ?1 FROM issues WHERE json_type(properties, ?1) IS NOT NULL ORDER BY id",
    )
    .bind(&path)
    .fetch_all(&mut *tx)
    .await?;
    sqlx::query("UPDATE issues SET properties = json_remove(properties, ?1) WHERE json_type(properties, ?1) IS NOT NULL")
        .bind(&path)
        .execute(&mut *tx)
        .await?;
    let field = format!("properties.{key}");
    for (id, v) in lost {
        let old: Value = dec(&v)?;
        let edit = Edit {
            field: &field,
            old: Some(&old),
            new: None,
        };
        history::record(tx, src, "issue", &id.to_string(), &edit).await?;
    }
    Ok(())
}

/// Drops the property from every project that holds a value.
async fn strip_from_projects(
    tx: &mut SqliteConnection,
    src: ChangeSource,
    key: &str,
) -> Result<()> {
    let path = format!("$.\"{key}\"");
    let path = path.as_str();
    let filter = "json_type(properties, ?) IS NOT NULL";
    let before = rows::snap(tx, "projects", filter, path).await?;
    sqlx::query("UPDATE projects SET properties = json_remove(properties, ?1) WHERE json_type(properties, ?1) IS NOT NULL")
        .bind(path)
        .execute(&mut *tx)
        .await?;
    rows::changed(tx, src, before).await
}

/// Removes the definition, every issue's value, and every saved-view reference to it.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn delete_def(db: &Db, key: &str, src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let def = get_def(&mut tx, key).await?;
    strip_from_issues(&mut tx, src, key).await?;
    strip_from_projects(&mut tx, src, key).await?;
    let (views_before, templates_before) = strip_from_views(&mut tx, key).await?;
    rows::changed(&mut tx, src, views_before).await?;
    rows::changed(&mut tx, src, templates_before).await?;
    let def_before = rows::by_key(&mut tx, "property_defs", key).await?;
    sqlx::query("DELETE FROM property_defs WHERE key = ?")
        .bind(key)
        .execute(&mut *tx)
        .await?;
    rows::changed(&mut tx, src, def_before).await?;
    record_def(&mut tx, src, key, "deleted", Some(&json!(def.name)), None).await?;
    tx.commit().await?;
    Ok(())
}

/// `None` clears the value.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn set_value(
    db: &Db,
    issue: IssueId,
    key: &str,
    value: Option<Value>,
    src: ChangeSource,
) -> Result<Issue> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let def = get_def(&mut tx, key).await?;
    let sql = format!("SELECT {} FROM issues WHERE id = ?", issues::ISSUE_COLS);
    let row = sqlx::query(&sql)
        .bind(issue.0)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| Error::NotFound(format!("issue {}", issue.0)))?;
    let mut found = issues::issues_from_rows(&mut tx, &[row]).await?.remove(0);
    if def.applies_to == PropertyAppliesTo::Projects {
        return Err(Error::Invalid(format!("{key} applies to projects only")));
    }
    if def.project_id.is_some() && def.project_id != found.project_id {
        return Err(Error::Invalid(format!(
            "{key} is scoped to another project than {}",
            found.key
        )));
    }
    if let Some(v) = &value {
        validate(&def, v)?;
    }
    let old = found.properties.get(key).cloned();
    if old != value {
        match &value {
            Some(v) => found.properties.insert(key.to_owned(), v.clone()),
            None => found.properties.remove(key),
        };
        found.updated_at = Utc::now();
        sqlx::query("UPDATE issues SET properties = ?, updated_at = ? WHERE id = ?")
            .bind(enc(&found.properties)?)
            .bind(found.updated_at)
            .bind(issue.0)
            .execute(&mut *tx)
            .await?;
        history::record(
            &mut tx,
            src,
            "issue",
            &issue.0.to_string(),
            &history::Edit {
                field: &format!("properties.{key}"),
                old: old.as_ref(),
                new: value.as_ref(),
            },
        )
        .await?;
    }
    tx.commit().await?;
    Ok(found)
}

/// # Errors
///
/// `Error::Invalid` when `value` does not fit the kind.
pub fn validate(def: &PropertyDef, value: &Value) -> Result<()> {
    let has = |s: &Value| {
        s.as_str()
            .is_some_and(|s| def.options.iter().any(|o| o.value == s))
    };
    let ok = match def.kind {
        PropertyKind::Text => value.is_string(),
        PropertyKind::Number => value.is_number(),
        PropertyKind::Date => value.as_str().is_some_and(|s| {
            NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok_and(|d| d.to_string() == s)
        }),
        PropertyKind::Select => has(value),
        PropertyKind::MultiSelect => value.as_array().is_some_and(|a| {
            a.iter().all(has) && a.iter().enumerate().all(|(i, v)| !a[..i].contains(v))
        }),
        PropertyKind::Checkbox => value.is_boolean(),
        PropertyKind::Url => value.as_str().is_some_and(|s| {
            ["http://", "https://"].iter().any(|p| {
                s.get(..p.len()).is_some_and(|h| h.eq_ignore_ascii_case(p))
                    && s.len() > p.len()
                    && !s.contains(char::is_whitespace)
            })
        }),
        PropertyKind::NoteLink => value.as_str().is_some_and(|s| {
            s.starts_with("obsidian://")
                || std::path::Path::new(s)
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("md"))
        }),
    };
    if ok {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "{value} is not a valid {:?} for {}",
            def.kind, def.key
        )))
    }
}
