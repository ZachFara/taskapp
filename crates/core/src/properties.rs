use chrono::{NaiveDate, Utc};
use serde_json::{Map, Value, json};
use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};

use crate::{
    ChangeSource, Db, Error, FieldRef, Filter, Issue, IssueId, ProjectId, PropertyDef,
    PropertyKind, Result, SelectOption, ViewDef,
    history::{self, Edit},
    issues,
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
    })
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

async fn get_def(conn: &mut SqliteConnection, key: &str) -> Result<PropertyDef> {
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
    let mut tx = db.pool.begin().await?;
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
        "INSERT INTO property_defs (key, name, kind, options, project_id, position) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&def.key)
    .bind(&def.name)
    .bind(enc(&def.kind)?)
    .bind(enc(&def.options)?)
    .bind(def.project_id.map(|p| p.0))
    .bind(position)
    .execute(&mut *tx)
    .await?;
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
    let mut tx = db.pool.begin().await?;
    let def = get_def(&mut tx, key).await?;
    sqlx::query("UPDATE property_defs SET name = ? WHERE key = ?")
        .bind(new_name)
        .bind(key)
        .execute(&mut *tx)
        .await?;
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
    let mut tx = db.pool.begin().await?;
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
    sqlx::query("UPDATE property_defs SET kind = ?, options = ? WHERE key = ?")
        .bind(enc(&new.kind)?)
        .bind(enc(&new.options)?)
        .bind(key)
        .execute(&mut *tx)
        .await?;
    let (before, after) = (json!(old.kind), json!(new.kind));
    record_def(&mut tx, src, key, "kind", Some(&before), Some(&after)).await?;
    if old.options != new.options {
        let (before, after) = (json!(old.options), json!(new.options));
        record_def(&mut tx, src, key, "options", Some(&before), Some(&after)).await?;
    }
    tx.commit().await?;
    Ok(new)
}

fn strip_filter(f: Filter, key: &str) -> Option<Filter> {
    match f {
        Filter::Cond {
            field: FieldRef::Property(ref k),
            ..
        } if k == key => None,
        Filter::And(v) => Some(
            v.into_iter()
                .filter_map(|f| strip_filter(f, key))
                .collect::<Vec<_>>(),
        )
        .filter(|v| !v.is_empty())
        .map(Filter::And),
        Filter::Or(v) => Some(
            v.into_iter()
                .filter_map(|f| strip_filter(f, key))
                .collect::<Vec<_>>(),
        )
        .filter(|v| !v.is_empty())
        .map(Filter::Or),
        Filter::Not(b) => strip_filter(*b, key).map(|f| Filter::Not(Box::new(f))),
        other @ Filter::Cond { .. } => Some(other),
    }
}

fn is_key(f: &FieldRef, key: &str) -> bool {
    matches!(f, FieldRef::Property(k) if k == key)
}

/// Removes the definition, every issue's value, and every saved-view reference to it.
///
/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn delete_def(db: &Db, key: &str, src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin().await?;
    let def = get_def(&mut tx, key).await?;
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
        history::record(&mut tx, src, "issue", &id.to_string(), &edit).await?;
    }
    let views = sqlx::query("SELECT id, definition FROM views")
        .fetch_all(&mut *tx)
        .await?;
    for r in views {
        let mut def: ViewDef = dec(r.get("definition"))?;
        let before = def.clone();
        def.filter = def.filter.and_then(|f| strip_filter(f, key));
        def.sorts.retain(|s| !is_key(&s.field, key));
        def.columns.retain(|c| !is_key(c, key));
        if def.group_by.as_ref().is_some_and(|g| is_key(g, key)) {
            def.group_by = None;
        }
        if def != before {
            sqlx::query("UPDATE views SET definition = ? WHERE id = ?")
                .bind(enc(&def)?)
                .bind(r.get::<i64, _>("id"))
                .execute(&mut *tx)
                .await?;
        }
    }
    sqlx::query("DELETE FROM property_defs WHERE key = ?")
        .bind(key)
        .execute(&mut *tx)
        .await?;
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
    let mut tx = db.pool.begin().await?;
    let def = get_def(&mut tx, key).await?;
    let sql = format!("SELECT {} FROM issues WHERE id = ?", issues::ISSUE_COLS);
    let row = sqlx::query(&sql)
        .bind(issue.0)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| Error::NotFound(format!("issue {}", issue.0)))?;
    let mut found = issues::issues_from_rows(&mut tx, &[row]).await?.remove(0);
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
