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
    Points,
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
            "points" => Field::Points,
            p if p
                .strip_prefix("property:")
                .is_some_and(|n| !n.trim().is_empty()) =>
            {
                Field::Property(p["property:".len()..].trim().to_owned())
            }
            other => {
                return Err(Error::Invalid(format!(
                    "unknown field `{other}`; use title, description, status, priority, project, labels, start_date, due_date, estimate, points or property:<name>"
                )));
            }
        })
    }
}

/// Largest file the import reads. A plan is returned whole to the caller, so the size is bounded.
const MAX_BYTES: usize = 5 * 1024 * 1024;

/// A file line number and the cells of the row that starts on it.
type Row = (u32, Vec<String>);

/// Parses RFC 4180 CSV: quoted fields, `""` escapes, newlines inside quotes (LF, CRLF or CR). Blank lines
/// are dropped. Each row comes with the file line it starts on (the header is line 1).
fn parse(text: &str) -> Result<Vec<Row>> {
    if text.len() > MAX_BYTES {
        return Err(Error::Invalid(format!(
            "the file is larger than {} MB; split it into smaller files",
            MAX_BYTES / (1024 * 1024)
        )));
    }
    let (mut rows, mut row, mut cell) = (vec![], vec![], String::new());
    let (mut quoted, mut was_quoted) = (false, false);
    let (mut line, mut row_line) = (1u32, 1u32);
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
            '\r' | '\n' if !quoted => {
                if c == '\r' && chars.peek() == Some(&'\n') {
                    continue;
                }
                row.push(std::mem::take(&mut cell));
                was_quoted = false;
                if row.iter().any(|c| !c.is_empty()) {
                    rows.push((row_line, std::mem::take(&mut row)));
                }
                row.clear();
                line += 1;
                row_line = line;
            }
            '\n' => {
                line += 1;
                cell.push(c);
            }
            _ => cell.push(c),
        }
    }
    if quoted {
        return Err(Error::Invalid("the file ends inside a quoted field".into()));
    }
    row.push(cell);
    if row.iter().any(|c| !c.is_empty()) {
        rows.push((row_line, row));
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
        "estimate" => Some("estimate"),
        "points" => Some("points"),
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
                color: super::plan::COLOR.into(),
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
        applies_to: crate::PropertyAppliesTo::Issues,
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

fn points(cell: &str) -> std::result::Result<f64, String> {
    cell.parse()
        .ok()
        .filter(|p: &f64| crate::issues::valid_points(*p))
        .ok_or_else(|| format!("`{cell}` is not points (0 to 1000, one decimal)"))
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
        points: None,
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
            Field::Points => i.points = Some(points(cell)?),
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
/// `status`, `priority`, `project`, `labels`, `start_date`, `due_date`, `estimate` in minutes, `points`, or
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
    let headers = &rows
        .first()
        .ok_or_else(|| Error::Invalid("the file is empty".into()))?
        .1;
    let mapping = mapping.cloned().unwrap_or_else(|| guess_mapping(headers));
    let cols = map_columns(headers, &mapping)?;
    let body = &rows[1..];
    let data: Vec<Vec<String>> = body.iter().map(|(_, c)| c.clone()).collect();
    let defs = property_defs(db, &cols, &data).await?;
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
        unmapped: headers
            .iter()
            .filter(|h| !mapping.contains_key(*h))
            .cloned()
            .collect(),
        ..ImportPlan::default()
    };
    let mut seen = std::collections::HashSet::new();
    let mut plan_rows: Vec<u32> = vec![];
    for (row, cells) in body {
        let row = *row;
        let mut warnings = vec![];
        match row_issue(&ctx, cells, &mut warnings) {
            Ok(i) if !seen.insert(i.external_id.clone()) => plan.skipped.push(Skipped {
                row,
                reason: "duplicate of an earlier row".into(),
            }),
            Ok(i) => {
                plan_rows.push(row);
                plan.issues.push(i);
            }
            Err(reason) => plan.skipped.push(Skipped { row, reason }),
        }
        for w in warnings {
            if !plan.warnings.contains(&w) {
                plan.warnings.push(w);
            }
        }
    }
    finish_with_rows(db, &mut plan, plan_rows).await?;
    Ok(plan)
}

/// Column index and field for each mapped header. Refuses duplicate headers, unknown mapped columns and no title.
fn map_columns(
    headers: &[String],
    mapping: &BTreeMap<String, String>,
) -> Result<Vec<(usize, Field)>> {
    if let Some(dup) = headers
        .iter()
        .enumerate()
        .find(|(i, h)| headers[..*i].contains(h))
    {
        return Err(Error::Invalid(format!(
            "column `{}` appears more than once",
            dup.1
        )));
    }
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
    Ok(cols)
}

/// Runs `finish`, then gives the rows it skips as already imported their file row, not their position.
async fn finish_with_rows(db: &Db, plan: &mut ImportPlan, plan_rows: Vec<u32>) -> Result<()> {
    let before = plan.skipped.len();
    let file_rows: Vec<(String, u32)> = plan
        .issues
        .iter()
        .map(|i| i.external_id.clone())
        .zip(plan_rows)
        .collect();
    finish(db, plan).await?;
    let kept: std::collections::HashSet<&str> =
        plan.issues.iter().map(|i| i.external_id.as_str()).collect();
    let mut gone = file_rows
        .iter()
        .filter(|(id, _)| !kept.contains(id.as_str()))
        .map(|(_, r)| *r);
    for s in plan.skipped.iter_mut().skip(before) {
        if let Some(r) = gone.next() {
            s.row = r;
        }
    }
    Ok(())
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
