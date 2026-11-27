//! `taskapp issue list`: issues filtered by names, shown with names, most urgent first.

use std::collections::HashMap;

use serde_json::{Value, json};
use taskapp_core::{
    Db, Error, Issue, LabelId, ProjectId, Result, StatusId, ViewDef, issues, labels, projects,
    statuses, views,
};

use crate::references;

const PRIORITIES: [&str; 5] = ["None", "Urgent", "High", "Medium", "Low"];

#[derive(clap::Args)]
pub struct ListArgs {
    /// Project id, slug or name; repeat or separate with commas.
    #[arg(long, value_delimiter = ',')]
    project: Vec<String>,
    /// Only these statuses (id or name); repeat or separate with commas.
    #[arg(long, value_delimiter = ',')]
    status: Vec<String>,
    /// Leave out these statuses, such as Done,Canceled,Someday.
    #[arg(long, value_delimiter = ',')]
    not_status: Vec<String>,
    /// urgent, high, medium, low, none or 0-4; repeat or separate with commas.
    #[arg(long, value_delimiter = ',')]
    priority: Vec<String>,
    /// Label name; issues with any of the given labels.
    #[arg(long, value_delimiter = ',')]
    label: Vec<String>,
}

fn cond(field: &str, op: &str, ids: &[i64]) -> Value {
    json!({ "Cond": { "field": { "Fixed": field }, "op": op, "value": ids } })
}

fn priority(input: &str) -> Result<i64> {
    PRIORITIES
        .iter()
        .position(|p| p.eq_ignore_ascii_case(input))
        .or_else(|| {
            input
                .parse::<usize>()
                .ok()
                .filter(|n| *n < PRIORITIES.len())
        })
        .and_then(|n| i64::try_from(n).ok())
        .ok_or_else(|| {
            Error::Invalid(format!(
                "priority {input}: use urgent, high, medium, low, none or 0-4"
            ))
        })
}

async fn status_ids(db: &Db, inputs: &[String]) -> Result<Vec<i64>> {
    let mut ids = Vec::new();
    for input in inputs {
        ids.extend(references::status(db, Some(input)).await?.map(|s| s.0));
    }
    Ok(ids)
}

async fn filter(db: &Db, args: &ListArgs) -> Result<Value> {
    let mut parts = Vec::new();
    if !args.project.is_empty() {
        let mut ids = Vec::new();
        for input in &args.project {
            ids.extend(references::project(db, Some(input)).await?.map(|p| p.0));
        }
        parts.push(cond("Project", "In", &ids));
    }
    if !args.status.is_empty() {
        parts.push(cond("Status", "In", &status_ids(db, &args.status).await?));
    }
    if !args.not_status.is_empty() {
        parts.push(cond(
            "Status",
            "NotIn",
            &status_ids(db, &args.not_status).await?,
        ));
    }
    if !args.priority.is_empty() {
        let ids = args
            .priority
            .iter()
            .map(|p| priority(p))
            .collect::<Result<Vec<_>>>()?;
        parts.push(cond("Priority", "In", &ids));
    }
    if !args.label.is_empty() {
        let ids = references::labels(db, &args.label).await?;
        parts.push(cond(
            "Label",
            "In",
            &ids.iter().map(|l| l.0).collect::<Vec<_>>(),
        ));
    }
    Ok(if parts.is_empty() {
        Value::Null
    } else {
        json!({ "And": parts })
    })
}

pub async fn run(db: &Db, args: &ListArgs) -> Result<Value> {
    let def: ViewDef = serde_json::from_value(json!({
        "filter": filter(db, args).await?, "sorts": [], "group_by": null,
        "layout": "List", "columns": [], "include_archived": false,
    }))
    .map_err(|e| Error::Invalid(e.to_string()))?;
    let mut rows = views::query(db, &def).await?;
    let number = |key: &str| key.rsplit('-').next().and_then(|n| n.parse::<u32>().ok());
    rows.sort_by_key(|i| (i.priority == 0, i.priority, number(&i.key)));
    let names = Names::load(db).await?;
    Ok(rows
        .iter()
        .map(|i| {
            json!({
                "key": i.key,
                "priority": priority_name(i.priority),
                "status": names.status(i),
                "project": names.project(i),
                "labels": names.labels(i),
                "title": i.title,
            })
        })
        .collect())
}

/// Display names for the ids on an issue, so readers need no id lookups.
pub struct Names {
    status: HashMap<StatusId, String>,
    project: HashMap<ProjectId, String>,
    label: HashMap<LabelId, String>,
}

impl Names {
    pub async fn load(db: &Db) -> Result<Self> {
        Ok(Self {
            status: statuses::list(db)
                .await?
                .into_iter()
                .map(|s| (s.id, s.name))
                .collect(),
            project: projects::list(db)
                .await?
                .into_iter()
                .map(|p| (p.id, p.name))
                .collect(),
            label: labels::list(db)
                .await?
                .into_iter()
                .map(|l| (l.id, l.name))
                .collect(),
        })
    }

    pub fn status(&self, i: &Issue) -> Option<&str> {
        self.status.get(&i.status_id).map(String::as_str)
    }

    pub fn project(&self, i: &Issue) -> Option<&str> {
        i.project_id
            .and_then(|p| self.project.get(&p))
            .map(String::as_str)
    }

    pub fn labels(&self, i: &Issue) -> Vec<&str> {
        i.label_ids
            .iter()
            .filter_map(|l| self.label.get(l).map(String::as_str))
            .collect()
    }
}

/// `issue get`: the raw issue plus names; ids and numeric priority stay.
pub async fn get(db: &Db, key: &str) -> Result<Value> {
    let issue = issues::get_by_key(db, key).await?;
    let names = Names::load(db).await?;
    let mut out = serde_json::to_value(&issue).map_err(|e| Error::Invalid(e.to_string()))?;
    out["status"] = json!(names.status(&issue));
    out["project"] = json!(names.project(&issue));
    out["labels"] = json!(names.labels(&issue));
    out["priority_name"] = json!(priority_name(issue.priority));
    Ok(out)
}

pub fn priority_name(priority: u8) -> &'static str {
    PRIORITIES[usize::from(priority).min(4)]
}
