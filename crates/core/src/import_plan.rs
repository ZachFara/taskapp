use std::collections::HashMap;

use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sqlx::SqliteConnection;

use crate::{
    ChangeSource, Db, Due, Error, NewIssue, PropertyDef, RelationKind, Result, StatusCategory,
    StatusId, history, issues, relations, rows, statuses::category_str,
};

const COLOR: &str = "#95a2b3";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct PlanStatus {
    pub name: String,
    pub category: StatusCategory,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct PlanCycle {
    pub name: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

/// Names refer to statuses, labels and projects; a cycle is its start date (cycles never overlap);
/// `parent` and relation ends are source identifiers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct PlanIssue {
    pub external_id: String,
    pub title: String,
    pub description: String,
    pub status: Option<String>,
    pub priority: u8,
    pub project: Option<String>,
    pub labels: Vec<String>,
    pub start_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub estimate_minutes: Option<u32>,
    #[serde(default)]
    pub points: Option<f64>,
    pub properties: Map<String, Value>,
    pub parent: Option<String>,
    pub cycle: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct PlanRelation {
    pub from: String,
    pub to: String,
    pub kind: RelationKind,
}

/// A source row that will not be imported. `row` counts from 1 (CSV: the header is row 1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Skipped {
    pub row: u32,
    pub reason: String,
}

/// Everything an import would create. The lists of statuses, labels, projects, cycles and property
/// definitions hold only what does not exist yet.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct ImportPlan {
    pub source: String,
    pub statuses: Vec<PlanStatus>,
    pub labels: Vec<String>,
    pub projects: Vec<String>,
    pub cycles: Vec<PlanCycle>,
    pub property_defs: Vec<PropertyDef>,
    pub issues: Vec<PlanIssue>,
    pub relations: Vec<PlanRelation>,
    pub warnings: Vec<String>,
    pub skipped: Vec<Skipped>,
    /// Source columns no field was mapped to.
    pub unmapped: Vec<String>,
}

#[derive(Debug, Serialize, specta::Type)]
pub struct Applied {
    pub created: u32,
    pub warnings: Vec<String>,
}

async fn find(conn: &mut SqliteConnection, sql: &str, v: impl ToString) -> Result<Option<i64>> {
    Ok(sqlx::query_scalar(sql)
        .bind(v.to_string())
        .fetch_optional(conn)
        .await?)
}

fn invalid(e: impl std::fmt::Display) -> Error {
    Error::Invalid(e.to_string())
}

fn distinct<'a>(names: impl Iterator<Item = &'a String>) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for n in names {
        if !out.iter().any(|o| o.eq_ignore_ascii_case(n)) {
            out.push(n.clone());
        }
    }
    out
}

async fn keep_missing<T>(
    c: &mut SqliteConnection,
    items: Vec<T>,
    sql: &str,
    key: impl Fn(&T) -> String,
) -> Result<Vec<T>> {
    let mut out = vec![];
    for it in items {
        if find(c, sql, key(&it)).await?.is_none() {
            out.push(it);
        }
    }
    Ok(out)
}

const STATUS_BY_NAME: &str = "SELECT id FROM statuses WHERE name = ? COLLATE NOCASE";
const LABEL_BY_NAME: &str = "SELECT id FROM labels WHERE name = ? COLLATE NOCASE";
const PROJECT_BY_NAME: &str = "SELECT id FROM projects WHERE name = ? COLLATE NOCASE";
const CYCLE_BY_START: &str = "SELECT id FROM cycles WHERE start_date = ?";
const DEF_BY_KEY: &str = "SELECT 1 FROM property_defs WHERE key = ?";

/// Narrows a source's raw plan to what is new: drops what exists, lists the labels and projects the
/// issues need, and skips issues already in `import_log`.
pub(crate) async fn finish(db: &Db, plan: &mut ImportPlan) -> Result<()> {
    let mut c = db.pool.acquire().await?;
    let c = &mut *c;
    plan.statuses = keep_missing(c, std::mem::take(&mut plan.statuses), STATUS_BY_NAME, |s| {
        s.name.clone()
    })
    .await?;
    let labels = distinct(plan.issues.iter().flat_map(|i| &i.labels));
    plan.labels = keep_missing(c, labels, LABEL_BY_NAME, Clone::clone).await?;
    let projects = distinct(plan.issues.iter().filter_map(|i| i.project.as_ref()));
    plan.projects = keep_missing(c, projects, PROJECT_BY_NAME, Clone::clone).await?;
    plan.property_defs = keep_missing(
        c,
        std::mem::take(&mut plan.property_defs),
        DEF_BY_KEY,
        |d| d.key.clone(),
    )
    .await?;
    plan_cycles(c, plan).await?;
    skip_imported(c, plan).await
}

/// A cycle that shares dates with an existing one is reused; one that overlaps an existing cycle
/// differently is dropped, because cycles may not overlap.
async fn plan_cycles(c: &mut SqliteConnection, plan: &mut ImportPlan) -> Result<()> {
    let mut kept = vec![];
    for cy in std::mem::take(&mut plan.cycles) {
        if find(c, CYCLE_BY_START, cy.start_date).await?.is_some() {
            continue;
        }
        let overlap = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM cycles WHERE start_date <= ?2 AND end_date >= ?1",
        )
        .bind(cy.start_date)
        .bind(cy.end_date)
        .fetch_one(&mut *c)
        .await?;
        if overlap > 0 {
            plan.warnings.push(format!(
                "cycle starting {} overlaps an existing cycle and was dropped",
                cy.start_date
            ));
            for i in plan
                .issues
                .iter_mut()
                .filter(|i| i.cycle == Some(cy.start_date))
            {
                i.cycle = None;
            }
        } else {
            kept.push(cy);
        }
    }
    plan.cycles = kept;
    Ok(())
}

async fn skip_imported(c: &mut SqliteConnection, plan: &mut ImportPlan) -> Result<()> {
    let mut fresh = vec![];
    for (n, i) in std::mem::take(&mut plan.issues).into_iter().enumerate() {
        let row = u32::try_from(n + 1).map_err(invalid)?;
        if find(
            c,
            "SELECT issue_id FROM import_log WHERE external_id = ?",
            &i.external_id,
        )
        .await?
        .is_some()
        {
            plan.skipped.push(Skipped {
                row,
                reason: "already imported".into(),
            });
        } else {
            fresh.push(i);
        }
    }
    plan.issues = fresh;
    Ok(())
}

fn slugify(name: &str, sep: char) -> String {
    let mut out = String::new();
    for ch in name.chars().map(|c| c.to_ascii_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if !out.is_empty() && !out.ends_with(sep) {
            out.push(sep);
        }
    }
    out.truncate(36);
    out.trim_end_matches(sep).to_owned()
}

/// A property key (`^[a-z][a-z0-9_]{0,39}$`) from a column name.
pub(crate) fn property_key(name: &str) -> String {
    let k = slugify(name, '_');
    if k.starts_with(|c: char| c.is_ascii_lowercase()) {
        k
    } else {
        format!("p_{k}")
    }
}

async fn created(
    tx: &mut SqliteConnection,
    entity: &str,
    id: impl ToString,
    what: &str,
) -> Result<()> {
    let new = json!(what);
    let edit = history::Edit {
        field: "created",
        old: None,
        new: Some(&new),
    };
    history::record(tx, ChangeSource::Migration, entity, &id.to_string(), &edit).await?;
    let table = match entity {
        "label" => "labels",
        "cycle" => "cycles",
        "project" => "projects",
        "property_def" => "property_defs",
        _ => return Ok(()),
    };
    rows::created(tx, ChangeSource::Migration, table, id).await
}

async fn ensure_refs(tx: &mut SqliteConnection, plan: &ImportPlan) -> Result<()> {
    for s in &plan.statuses {
        if find(tx, STATUS_BY_NAME, &s.name).await?.is_none() {
            sqlx::query("INSERT INTO statuses (name, category, position, color) VALUES (?, ?, (SELECT COALESCE(MAX(position), -1) + 1 FROM statuses), ?)")
                .bind(&s.name).bind(category_str(s.category)).bind(COLOR).execute(&mut *tx).await?;
        }
    }
    for l in &plan.labels {
        if find(tx, LABEL_BY_NAME, l).await?.is_none() {
            let id = sqlx::query("INSERT INTO labels (name, color) VALUES (?, ?)")
                .bind(l)
                .bind(COLOR)
                .execute(&mut *tx)
                .await?
                .last_insert_rowid();
            created(tx, "label", id, l).await?;
        }
    }
    for p in &plan.projects {
        ensure_project(tx, p).await?;
    }
    ensure_cycles(tx, plan).await?;
    ensure_defs(tx, plan).await
}

async fn ensure_cycles(tx: &mut SqliteConnection, plan: &ImportPlan) -> Result<()> {
    for c in &plan.cycles {
        if find(tx, CYCLE_BY_START, c.start_date).await?.is_none() {
            let id = sqlx::query("INSERT INTO cycles (number, name, start_date, end_date) VALUES ((SELECT coalesce(max(number), 0) + 1 FROM cycles), ?, ?, ?)")
                .bind(&c.name).bind(c.start_date).bind(c.end_date).execute(&mut *tx).await?.last_insert_rowid();
            created(
                tx,
                "cycle",
                id,
                &format!("{} to {}", c.start_date, c.end_date),
            )
            .await?;
        }
    }
    Ok(())
}

async fn ensure_defs(tx: &mut SqliteConnection, plan: &ImportPlan) -> Result<()> {
    for d in &plan.property_defs {
        if find(tx, DEF_BY_KEY, &d.key).await?.is_none() {
            sqlx::query("INSERT INTO property_defs (key, name, kind, options, project_id, position) VALUES (?, ?, ?, ?, NULL, (SELECT coalesce(max(position) + 1, 0) FROM property_defs))")
                .bind(&d.key).bind(&d.name)
                .bind(serde_json::to_string(&d.kind).map_err(invalid)?)
                .bind(serde_json::to_string(&d.options).map_err(invalid)?)
                .execute(&mut *tx).await?;
            created(tx, "property_def", &d.key, &d.name).await?;
        }
    }
    Ok(())
}

async fn ensure_project(tx: &mut SqliteConnection, name: &str) -> Result<()> {
    if find(tx, PROJECT_BY_NAME, name).await?.is_some() {
        return Ok(());
    }
    let base = match slugify(name, '-') {
        s if s.is_empty() => "project".to_owned(),
        s => s,
    };
    let mut slug = base.clone();
    let mut n = 1;
    while find(tx, "SELECT id FROM projects WHERE slug = ?", &slug)
        .await?
        .is_some()
    {
        n += 1;
        slug = format!("{base}-{n}");
    }
    let id = sqlx::query("INSERT INTO projects (slug, name, created_at) VALUES (?, ?, ?)")
        .bind(&slug)
        .bind(name)
        .bind(Utc::now())
        .execute(&mut *tx)
        .await?
        .last_insert_rowid();
    created(tx, "project", id, &slug).await
}

async fn new_issue(tx: &mut SqliteConnection, i: &PlanIssue) -> Result<NewIssue> {
    let mut ids = vec![];
    for l in &i.labels {
        if let Some(id) = find(tx, LABEL_BY_NAME, l).await? {
            ids.push(crate::LabelId(id));
        }
    }
    let status = match &i.status {
        Some(s) => find(tx, STATUS_BY_NAME, s).await?,
        None => None,
    };
    let project = match &i.project {
        Some(p) => find(tx, PROJECT_BY_NAME, p).await?,
        None => None,
    };
    let cycle = match i.cycle {
        Some(d) => find(tx, CYCLE_BY_START, d).await?,
        None => None,
    };
    Ok(NewIssue {
        title: i.title.clone(),
        description: Some(i.description.clone()),
        status_id: status.map(StatusId),
        priority: Some(i.priority),
        project_id: project.map(crate::ProjectId),
        cycle_id: cycle.map(crate::CycleId),
        label_ids: Some(ids),
        start_date: i.start_date,
        due: i.due_date.map(Due::Date),
        estimate_minutes: i.estimate_minutes,
        points: i.points,
        properties: Some(i.properties.clone()),
        ..NewIssue::default()
    })
}

async fn resolve(
    tx: &mut SqliteConnection,
    ids: &HashMap<String, i64>,
    ext: &str,
) -> Result<Option<i64>> {
    match ids.get(ext) {
        Some(id) => Ok(Some(*id)),
        None => {
            find(
                tx,
                "SELECT issue_id FROM import_log WHERE external_id = ?",
                ext,
            )
            .await
        }
    }
}

async fn link(
    tx: &mut SqliteConnection,
    plan: &ImportPlan,
    ids: &HashMap<String, i64>,
    warnings: &mut Vec<String>,
) -> Result<()> {
    for i in plan
        .issues
        .iter()
        .filter(|i| ids.contains_key(&i.external_id))
    {
        let Some(parent) = &i.parent else { continue };
        match resolve(tx, ids, parent).await? {
            Some(p) => {
                let me = ids[&i.external_id];
                sqlx::query("UPDATE issues SET parent_id = ? WHERE id = ?")
                    .bind(p)
                    .bind(me)
                    .execute(&mut *tx)
                    .await?;
                let new = json!(p);
                let edit = history::Edit {
                    field: "parent_id",
                    old: None,
                    new: Some(&new),
                };
                history::record(tx, ChangeSource::Migration, "issue", &me.to_string(), &edit)
                    .await?;
            }
            None => warnings.push(format!(
                "{}: parent {parent} was not imported",
                i.external_id
            )),
        }
    }
    for r in &plan.relations {
        let ends = (
            resolve(tx, ids, &r.from).await?,
            resolve(tx, ids, &r.to).await?,
        );
        let (Some(from), Some(to)) = ends else {
            warnings.push(format!(
                "relation {} -> {} skipped: an issue was not imported",
                r.from, r.to
            ));
            continue;
        };
        let rel = crate::Relation {
            from: crate::IssueId(from),
            to: crate::IssueId(to),
            kind: r.kind,
        };
        if let Err(e) = relations::add_in(tx, rel, ChangeSource::Migration).await {
            warnings.push(format!("relation {} -> {} skipped: {e}", r.from, r.to));
        }
    }
    Ok(())
}

/// Writes `plan` in one transaction and one undoable step labelled "Import N issues". An issue
/// whose identifier is already in `import_log` is skipped, so applying a plan twice creates nothing
/// the second time. Undo works for an import that only creates issues; one that also creates
/// labels, projects, statuses, cycles, property definitions or relations is refused by undo, which
/// does not cover those.
///
/// # Errors
/// `Invalid` when an issue fails core validation (nothing is written), `Db` for other failures.
pub async fn apply(db: &Db, plan: &ImportPlan) -> Result<Applied> {
    let mut fresh = 0;
    let mut c = db.pool.acquire().await?;
    for i in &plan.issues {
        if find(
            &mut c,
            "SELECT issue_id FROM import_log WHERE external_id = ?",
            &i.external_id,
        )
        .await?
        .is_none()
        {
            fresh += 1;
        }
    }
    drop(c);
    history::step(&format!("Import {fresh} issues"), write(db, plan)).await
}

async fn write(db: &Db, plan: &ImportPlan) -> Result<Applied> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    ensure_refs(&mut tx, plan).await?;
    let (mut ids, mut warnings) = (HashMap::new(), vec![]);
    for i in &plan.issues {
        if resolve(&mut tx, &ids, &i.external_id).await?.is_some() {
            warnings.push(format!("{} was already imported", i.external_id));
            continue;
        }
        let new = new_issue(&mut tx, i).await?;
        let issue = issues::create_in(&mut tx, new, ChangeSource::Migration).await?;
        sqlx::query("INSERT INTO import_log (external_id, source, issue_id, imported_at) VALUES (?, ?, ?, ?)")
            .bind(&i.external_id).bind(&plan.source).bind(issue.id.0).bind(Utc::now()).execute(&mut *tx).await?;
        ids.insert(i.external_id.clone(), issue.id.0);
    }
    link(&mut tx, plan, &ids, &mut warnings).await?;
    tx.commit().await?;
    Ok(Applied {
        created: u32::try_from(ids.len()).map_err(invalid)?,
        warnings,
    })
}
