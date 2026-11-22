//! CSV import: a column-to-field mapping turns rows into an [`ImportPlan`].

use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde_json::{Map, Number, Value};

use super::plan::{ImportPlan, PlanIssue, Skipped, finish, property_key};
use crate::{Db, Error, PropertyDef, PropertyKind, Result, SelectOption, properties};

/// What a column maps to.
#[derive(Debug, Clone, PartialEq)]
enum Field {
    Title,
    Description,
    Status,
    Priority,
    Project,
    Labels,
    Start,
    Due,
    Estimate,
    Property(String),
}

impl Field {
    fn parse(s: &str) -> Result<Field> {
        Ok(match s.trim() {
            "title" => Field::Title,
            "description" => Field::Description,
            "status" => Field::Status,
            "priority" => Field::Priority,
            "project" => Field::Project,
            "labels" => Field::Labels,
            "start_date" => Field::Start,
            "due_date" => Field::Due,
            "estimate" => Field::Estimate,
            p if p
                .strip_prefix("property:")
                .is_some_and(|n| !n.trim().is_empty()) =>
            {
                Field::Property(p["property:".len()..].trim().to_owned())
            }
            other => {
                return Err(Error::Invalid(format!(
                    "unknown field `{other}`; use title, description, status, priority, project, labels, start_date, due_date, estimate or property:<name>"
                )));
            }
        })
    }
}

/// Parses RFC 4180 CSV: quoted fields, `""` escapes, newlines inside quotes. Blank lines are dropped.
fn parse(text: &str) -> Result<Vec<Vec<String>>> {
    let (mut rows, mut row, mut cell) = (vec![], vec![], String::new());
    let (mut quoted, mut was_quoted) = (false, false);
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cell.push('"');
                chars.next();
            }
            '"' if quoted => quoted = false,
            '"' if cell.is_empty() && !was_quoted => (quoted, was_quoted) = (true, true),
            ',' if !quoted => {
                row.push(std::mem::take(&mut cell));
                was_quoted = false;
            }
            '\r' if !quoted => {}
            '\n' if !quoted => {
                row.push(std::mem::take(&mut cell));
                was_quoted = false;
                if row.iter().any(|c| !c.is_empty()) {
                    rows.push(std::mem::take(&mut row));
                }
                row.clear();
            }
            _ => cell.push(c),
        }
    }
    if quoted {
        return Err(Error::Invalid("the file ends inside a quoted field".into()));
    }
    row.push(cell);
    if row.iter().any(|c| !c.is_empty()) {
        rows.push(row);
    }
    Ok(rows)
}

/// A mapping guessed from header names; headers it does not recognise are left out.
#[must_use]
pub fn guess_mapping(headers: &[String]) -> BTreeMap<String, String> {
    let field = |h: &str| match h.trim().to_lowercase().replace(['_', '-'], " ").as_str() {
        "title" | "name" | "summary" => Some("title"),
        "description" | "body" | "details" | "notes" => Some("description"),
        "status" | "state" => Some("status"),
        "priority" => Some("priority"),
        "project" => Some("project"),
        "labels" | "label" | "tags" => Some("labels"),
        "start" | "start date" => Some("start_date"),
        "due" | "due date" | "deadline" => Some("due_date"),
        "estimate" | "points" => Some("estimate"),
        _ => None,
    };
    headers
        .iter()
        .filter_map(|h| field(h).map(|f| (h.clone(), f.to_owned())))
        .collect()
}

fn date(s: &str) -> std::result::Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .ok()
        .filter(|d| d.to_string() == s)
        .ok_or_else(|| format!("`{s}` is not a YYYY-MM-DD date"))
}

fn priority(s: &str) -> std::result::Result<u8, String> {
    match s.to_lowercase().as_str() {
        "" | "none" => Ok(0),
        "urgent" => Ok(1),
        "high" => Ok(2),
        "medium" => Ok(3),
        "low" => Ok(4),
        n => {
            n.parse().ok().filter(|p| *p <= 4).ok_or_else(|| {
                format!("`{s}` is not a priority (0 to 4, urgent, high, medium, low)")
            })
        }
    }
}

/// Columns of one property, as the definition the import will use and whether it is new.
fn infer(name: &str, values: &[&str]) -> PropertyDef {
    let filled: Vec<&str> = values.iter().copied().filter(|v| !v.is_empty()).collect();
    let mut uniq: Vec<&str> = vec![];
    for v in &filled {
        if !uniq.contains(v) {
            uniq.push(v);
        }
    }
    let (kind, options) = if filled.is_empty() {
        (PropertyKind::Text, vec![])
    } else if filled
        .iter()
        .all(|v| v.parse::<f64>().is_ok_and(f64::is_finite))
    {
        (PropertyKind::Number, vec![])
    } else if filled.iter().all(|v| date(v).is_ok()) {
        (PropertyKind::Date, vec![])
    } else if uniq.len() <= 6 && filled.len() >= 3 * uniq.len() {
        let options = uniq
            .iter()
            .map(|v| SelectOption {
                value: (*v).to_owned(),
                color: "#95a2b3".into(),
            })
            .collect();
        (PropertyKind::Select, options)
    } else {
        (PropertyKind::Text, vec![])
    };
    PropertyDef {
        key: property_key(name),
        name: name.to_owned(),
        kind,
        options,
        project_id: None,
        position: 0,
    }
}

fn prop_value(def: &PropertyDef, cell: &str) -> std::result::Result<Value, String> {
    let v = if def.kind == PropertyKind::Number {
        cell.parse::<f64>()
            .ok()
            .and_then(Number::from_f64)
            .map(Value::Number)
            .ok_or_else(|| format!("`{cell}` is not a number for {}", def.name))?
    } else {
        Value::String(cell.to_owned())
    };
    properties::validate(def, &v).map_err(|e| e.to_string())?;
    Ok(v)
}

struct Ctx<'a> {
    cols: &'a [(usize, Field)],
    defs: &'a BTreeMap<String, PropertyDef>,
    statuses: &'a [String],
}

fn row_issue(
    ctx: &Ctx<'_>,
    cells: &[String],
    warnings: &mut Vec<String>,
) -> std::result::Result<PlanIssue, String> {
    let mut i = PlanIssue {
        external_id: format!("csv:{}", cells.join("\u{1f}")),
        title: String::new(),
        description: String::new(),
        status: None,
        priority: 0,
        project: None,
        labels: vec![],
        start_date: None,
        due_date: None,
        estimate_minutes: None,
        properties: Map::new(),
        parent: None,
        cycle: None,
    };
    for (idx, field) in ctx.cols {
        let cell = cells.get(*idx).map_or("", |c| c.trim());
        if cell.is_empty() {
            continue;
        }
        match field {
            Field::Title => cell.clone_into(&mut i.title),
            Field::Description => cells[*idx].clone_into(&mut i.description),
            Field::Status => match ctx.statuses.iter().find(|s| s.eq_ignore_ascii_case(cell)) {
                Some(s) => i.status = Some(s.clone()),
                None => warnings.push(format!(
                    "unknown status `{cell}`; those issues get the default status"
                )),
            },
            Field::Priority => i.priority = priority(cell)?,
            Field::Project => i.project = Some(cell.to_owned()),
            Field::Labels => {
                i.labels = cell
                    .split([',', ';'])
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(str::to_owned)
                    .collect();
            }
            Field::Start => i.start_date = Some(date(cell)?),
            Field::Due => i.due_date = Some(date(cell)?),
            Field::Estimate => {
                i.estimate_minutes = Some(
                    cell.parse()
                        .map_err(|_| format!("`{cell}` is not a whole number of minutes"))?,
                );
            }
            Field::Property(name) => {
                let def = &ctx.defs[name];
                i.properties.insert(def.key.clone(), prop_value(def, cell)?);
            }
        }
    }
    if i.title.is_empty() {
        return Err("title is empty".into());
    }
    Ok(i)
}

/// Builds the plan for `text`. `mapping` goes from column name to field (`title`, `description`,
/// `status`, `priority`, `project`, `labels`, `start_date`, `due_date`, `estimate` in minutes, or
/// `property:<name>`); `None` guesses it from the headers. Writes nothing.
///
/// # Errors
/// `Invalid` for an unterminated quote, an unknown field, an unknown column in the mapping, or no
/// column mapped to `title`.
pub async fn plan(
    db: &Db,
    text: &str,
    mapping: Option<&BTreeMap<String, String>>,
) -> Result<ImportPlan> {
    let rows = parse(text)?;
    let headers = rows
        .first()
        .ok_or_else(|| Error::Invalid("the file is empty".into()))?;
    let mapping = mapping.cloned().unwrap_or_else(|| guess_mapping(headers));
    if let Some(bad) = mapping.keys().find(|k| !headers.contains(k)) {
        return Err(Error::Invalid(format!("no column named `{bad}`")));
    }
    let mut cols = vec![];
    for (idx, h) in headers.iter().enumerate() {
        if let Some(f) = mapping.get(h) {
            cols.push((idx, Field::parse(f)?));
        }
    }
    if !cols.iter().any(|(_, f)| *f == Field::Title) {
        return Err(Error::Invalid("no column is mapped to title".into()));
    }
    let defs = property_defs(db, &cols, &rows[1..]).await?;
    let statuses = crate::statuses::list(db)
        .await?
        .into_iter()
        .map(|s| s.name)
        .collect::<Vec<_>>();
    let ctx = Ctx {
        cols: &cols,
        defs: &defs.0,
        statuses: &statuses,
    };
    let mut plan = ImportPlan {
        source: "csv".into(),
        property_defs: defs.1,
        ..ImportPlan::default()
    };
    plan.unmapped = headers
        .iter()
        .filter(|h| !mapping.contains_key(*h))
        .cloned()
        .collect();
    let mut seen = std::collections::HashSet::new();
    for (n, cells) in rows.iter().enumerate().skip(1) {
        let row = u32::try_from(n + 1).map_err(|e| Error::Invalid(e.to_string()))?;
        let mut warnings = vec![];
        match row_issue(&ctx, cells, &mut warnings) {
            Ok(i) if !seen.insert(i.external_id.clone()) => plan.skipped.push(Skipped {
                row,
                reason: "duplicate of an earlier row".into(),
            }),
            Ok(i) => plan.issues.push(i),
            Err(reason) => plan.skipped.push(Skipped { row, reason }),
        }
        for w in warnings {
            if !plan.warnings.contains(&w) {
                plan.warnings.push(w);
            }
        }
    }
    finish(db, &mut plan).await?;
    Ok(plan)
}

/// The definition per mapped property name (existing ones win, new ones are inferred), and the new ones.
async fn property_defs(
    db: &Db,
    cols: &[(usize, Field)],
    data: &[Vec<String>],
) -> Result<(BTreeMap<String, PropertyDef>, Vec<PropertyDef>)> {
    let (mut all, mut new) = (BTreeMap::new(), vec![]);
    let existing = properties::list_defs(db).await?;
    for (idx, f) in cols {
        let Field::Property(name) = f else { continue };
        let key = property_key(name);
        if !(1..=40).contains(&key.len()) {
            return Err(Error::Invalid(format!(
                "`{name}` cannot be turned into a property key"
            )));
        }
        if let Some(d) = existing.iter().find(|d| d.key == key) {
            all.insert(name.clone(), d.clone());
        } else {
            let values: Vec<&str> = data
                .iter()
                .map(|r| r.get(*idx).map_or("", |c| c.trim()))
                .collect();
            let d = infer(name, &values);
            all.insert(name.clone(), d.clone());
            new.push(d);
        }
    }
    Ok((all, new))
}
