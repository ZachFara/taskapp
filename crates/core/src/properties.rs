use chrono::{DateTime, NaiveDate, Utc};
use serde_json::{Map, Value, json};
use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};

use crate::{
    ChangeSource, Db, Due, Error, FieldRef, Filter, Issue, IssueId, LabelId, MilestoneId,
    ProjectId, PropertyDef, PropertyKind, Result, SelectOption, StatusId, ViewDef, history,
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

async fn get_def(conn: &mut SqliteConnection, key: &str) -> Result<PropertyDef> {
    let row = sqlx::query("SELECT * FROM property_defs WHERE key = ?")
        .bind(key)
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| Error::NotFound(format!("property {key}")))?;
    def_from_row(&row)
}

/// Builds an issue from its row. Issues are read here, not through the issue module, so a value
/// change stays inside one transaction.
async fn issue_from_row(conn: &mut SqliteConnection, r: &SqliteRow) -> Result<Issue> {
    let id: i64 = r.get("id");
    let prefix: String =
        sqlx::query_scalar("SELECT value FROM settings WHERE key = 'issue_prefix'")
            .fetch_one(&mut *conn)
            .await
            .map(|v: String| dec(&v))??;
    let labels: Vec<i64> = sqlx::query_scalar(
        "SELECT label_id FROM issue_labels WHERE issue_id = ? ORDER BY label_id",
    )
    .bind(id)
    .fetch_all(conn)
    .await?;
    let due = match (
        r.get::<Option<DateTime<Utc>>, _>("due_time_utc"),
        r.get::<Option<String>, _>("due_tz"),
    ) {
        (Some(utc), Some(tz)) => Some(Due::DateTime { utc, tz }),
        _ => r.get::<Option<NaiveDate>, _>("due_date").map(Due::Date),
    };
    Ok(Issue {
        id: IssueId(id),
        key: format!("{prefix}-{}", r.get::<i64, _>("number")),
        title: r.get("title"),
        description: r.get("description"),
        status_id: StatusId(r.get("status_id")),
        priority: u8::try_from(r.get::<i64, _>("priority")).unwrap_or(0),
        project_id: r.get::<Option<i64>, _>("project_id").map(ProjectId),
        milestone_id: r.get::<Option<i64>, _>("milestone_id").map(MilestoneId),
        parent_id: r.get::<Option<i64>, _>("parent_id").map(IssueId),
        cycle_id: r.get::<Option<i64>, _>("cycle_id").map(crate::CycleId),
        label_ids: labels.into_iter().map(LabelId).collect(),
        start_date: r.get("start_date"),
        due,
        estimate_minutes: r
            .get::<Option<i64>, _>("estimate_minutes")
            .and_then(|m| u32::try_from(m).ok()),
        properties: dec(r.get("properties"))?,
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
        completed_at: r.get("completed_at"),
        archived_at: r.get("archived_at"),
    })
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
pub async fn create_def(db: &Db, def: PropertyDef, _src: ChangeSource) -> Result<PropertyDef> {
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
    _src: ChangeSource,
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

/// New `properties` JSON per issue that holds `key`; refuses, writing nothing, if any value cannot convert.
async fn convert_all(
    tx: &mut SqliteConnection,
    key: &str,
    old: &PropertyDef,
    new: &PropertyDef,
) -> Result<Vec<(i64, String)>> {
    let rows = sqlx::query("SELECT * FROM issues WHERE properties != '{}' ORDER BY id")
        .fetch_all(&mut *tx)
        .await?;
    let mut writes = Vec::new();
    let mut bad = Vec::new();
    for r in &rows {
        let mut props: Map<String, Value> = dec(r.get("properties"))?;
        let Some(v) = props.get(key) else { continue };
        match convert(old.kind, new.kind, v).and_then(|c| match c {
            Some(c) => validate(new, &c).map(|()| Some(c)),
            None => Ok(None),
        }) {
            Ok(Some(c)) => props.insert(key.to_owned(), c),
            Ok(None) => props.remove(key),
            Err(_) => {
                bad.push(issue_from_row(tx, r).await?.key);
                continue;
            }
        };
        writes.push((r.get::<i64, _>("id"), enc(&props)?));
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
    _src: ChangeSource,
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
    for (id, props) in writes {
        sqlx::query("UPDATE issues SET properties = ?, updated_at = ? WHERE id = ?")
            .bind(props)
            .bind(now)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("UPDATE property_defs SET kind = ?, options = ? WHERE key = ?")
        .bind(enc(&new.kind)?)
        .bind(enc(&new.options)?)
        .bind(key)
        .execute(&mut *tx)
        .await?;
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
pub async fn delete_def(db: &Db, key: &str, _src: ChangeSource) -> Result<()> {
    let mut tx = db.pool.begin().await?;
    get_def(&mut tx, key).await?;
    sqlx::query("UPDATE issues SET properties = json_remove(properties, '$.' || ?) WHERE json_type(properties, '$.' || ?) IS NOT NULL")
        .bind(format!("\"{key}\""))
        .bind(format!("\"{key}\""))
        .execute(&mut *tx)
        .await?;
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
    let row = sqlx::query("SELECT * FROM issues WHERE id = ?")
        .bind(issue.0)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| Error::NotFound(format!("issue {}", issue.0)))?;
    let mut found = issue_from_row(&mut tx, &row).await?;
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
