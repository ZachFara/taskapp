//! Linear import over its GraphQL API. The HTTP call sits behind [`LinearApi`] so tests run on
//! fixtures. The token is held in a [`Token`] that never prints itself and is never stored.

use std::process::Stdio;

use chrono::NaiveDate;
use serde_json::{Value, json};
use tokio::{io::AsyncWriteExt, process::Command};

use super::plan::{ImportPlan, PlanCycle, PlanIssue, PlanRelation, PlanStatus, Skipped, finish};
use crate::{Db, Error, RelationKind, Result, StatusCategory};

const QUERY: &str = "query($after: String) { issues(first: 100, after: $after, includeArchived: false) { \
  nodes { identifier title description priority estimate dueDate \
    state { name type } project { name } labels { nodes { name } } \
    cycle { number name startsAt endsAt } parent { identifier } \
    relations { nodes { type relatedIssue { identifier } } } } \
  pageInfo { hasNextPage endCursor } } }";

/// An API token. Debug prints a placeholder; there is no Display or Serialize.
pub struct Token(String);

impl Token {
    pub fn new(token: impl Into<String>) -> Token {
        Token(token.into())
    }

    /// Reads `LINEAR_API_KEY` now.
    ///
    /// # Errors
    /// `Invalid` when it is unset or empty.
    pub fn from_env() -> Result<Token> {
        std::env::var("LINEAR_API_KEY")
            .ok()
            .filter(|t| !t.is_empty())
            .map(Token)
            .ok_or_else(|| Error::Invalid("LINEAR_API_KEY is not set".into()))
    }

    fn redact(&self, msg: &str) -> String {
        msg.replace(&self.0, "[token]")
    }
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Token(redacted)")
    }
}

/// One GraphQL call; returns the response's `data` object.
pub trait LinearApi {
    fn query(&self, query: &str, vars: Value) -> impl Future<Output = Result<Value>> + Send;
}

/// Unwraps a GraphQL response body: `data` on success, an error built from `errors` otherwise.
///
/// # Errors
/// `Invalid` with the API's messages, token removed, when the body has `errors` or no `data`.
pub fn response_data(body: &Value, token: &Token) -> Result<Value> {
    if let Some(errors) = body.get("errors") {
        let msgs: Vec<&str> = errors
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|e| e["message"].as_str())
            .collect();
        return Err(Error::Invalid(
            token.redact(&format!("Linear: {}", msgs.join("; "))),
        ));
    }
    body.get("data")
        .cloned()
        .ok_or_else(|| Error::Invalid("Linear: response has no data".into()))
}

/// Calls the real API through `curl`; the token and body travel on its stdin, never in its arguments.
///
/// shortcut: shells out because the workspace has no HTTP client for core; swap for `reqwest` when
/// the manifest allows. Never run against the live API by the tests.
#[derive(Debug)]
pub struct CurlLinear {
    token: Token,
}

impl CurlLinear {
    #[must_use]
    pub fn new(token: Token) -> CurlLinear {
        CurlLinear { token }
    }
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

impl LinearApi for CurlLinear {
    async fn query(&self, query: &str, vars: Value) -> Result<Value> {
        let body = json!({"query": query, "variables": vars}).to_string();
        let config = format!(
            "url = \"https://api.linear.app/graphql\"\nheader = {}\nheader = \"Content-Type: application/json\"\ndata = {}\n",
            quote(&format!("Authorization: {}", self.token.0)),
            quote(&body)
        );
        let mut child = Command::new("curl")
            .args([
                "--silent",
                "--show-error",
                "--fail-with-body",
                "--config",
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::Invalid("curl has no stdin".into()))?;
        stdin.write_all(config.as_bytes()).await?;
        drop(stdin);
        let out = child.wait_with_output().await?;
        let parsed = serde_json::from_slice::<Value>(&out.stdout);
        match parsed {
            Ok(v) if v.get("data").is_some() || v.get("errors").is_some() => {
                response_data(&v, &self.token)
            }
            _ => Err(Error::Invalid(self.token.redact(&format!(
                "Linear request failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )))),
        }
    }
}

fn category(t: &str) -> Option<StatusCategory> {
    Some(match t {
        "triage" | "backlog" => StatusCategory::Backlog,
        "unstarted" => StatusCategory::Unstarted,
        "started" => StatusCategory::Started,
        "completed" => StatusCategory::Completed,
        "canceled" => StatusCategory::Canceled,
        _ => return None,
    })
}

fn day(v: &Value) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(v.as_str()?.get(..10)?, "%Y-%m-%d").ok()
}

fn names(v: &Value) -> Vec<String> {
    v["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|n| n["name"].as_str().map(str::to_owned))
        .collect()
}

fn add_node(plan: &mut ImportPlan, n: &Value, as_points: bool) -> std::result::Result<(), String> {
    let text = |k: &str| n[k].as_str().map(str::to_owned);
    let id = text("identifier").ok_or("no identifier")?;
    let title = text("title")
        .filter(|t| !t.trim().is_empty())
        .ok_or("title is empty")?;
    let state = text_of(&n["state"], "name").ok_or("no workflow state")?;
    let cat = n["state"]["type"]
        .as_str()
        .and_then(category)
        .unwrap_or_else(|| {
            plan.warnings.push(format!(
                "{id}: unknown state type for `{state}`; treated as backlog"
            ));
            StatusCategory::Backlog
        });
    if !plan.statuses.iter().any(|s| s.name == state) {
        plan.statuses.push(PlanStatus {
            name: state.clone(),
            category: cat,
        });
    }
    let cycle = day(&n["cycle"]["startsAt"]).zip(day(&n["cycle"]["endsAt"]));
    if let Some((start_date, end_date)) =
        cycle.filter(|(s, _)| !plan.cycles.iter().any(|c| c.start_date == *s))
    {
        plan.cycles.push(PlanCycle {
            name: text_of(&n["cycle"], "name"),
            start_date,
            end_date,
        });
    }
    add_relations(plan, &id, n);
    plan.issues.push(PlanIssue {
        external_id: id,
        title,
        description: text("description").unwrap_or_default(),
        status: Some(state),
        priority: n["priority"]
            .as_u64()
            .and_then(|p| u8::try_from(p).ok())
            .filter(|p| *p <= 4)
            .unwrap_or(0),
        project: text_of(&n["project"], "name"),
        labels: names(&n["labels"]),
        start_date: None,
        due_date: day(&n["dueDate"]),
        // Where the workspace counts points the estimate is points; otherwise one point is one hour.
        estimate_minutes: n["estimate"]
            .as_f64()
            .filter(|_| !as_points)
            .and_then(|e| format!("{:.0}", e * 60.0).parse().ok())
            .filter(|m| *m > 0),
        points: n["estimate"]
            .as_f64()
            .filter(|e| as_points && *e > 0.0 && crate::issues::valid_points(*e)),
        properties: serde_json::Map::new(),
        parent: text_of(&n["parent"], "identifier"),
        cycle: cycle.map(|(s, _)| s),
    });
    Ok(())
}

fn add_relations(plan: &mut ImportPlan, id: &str, n: &Value) {
    for r in n["relations"]["nodes"].as_array().into_iter().flatten() {
        let kind = match r["type"].as_str() {
            Some(t @ ("blocks" | "duplicate" | "related")) => RelationKind::new(t),
            other => {
                plan.warnings
                    .push(format!("{id}: relation type {other:?} is not supported"));
                continue;
            }
        };
        if let Some(to) = text_of(&r["relatedIssue"], "identifier") {
            plan.relations.push(PlanRelation {
                from: id.to_owned(),
                to,
                kind,
            });
        }
    }
}

fn text_of(v: &Value, k: &str) -> Option<String> {
    v[k].as_str().map(str::to_owned)
}

/// Pages through every issue and builds the plan. Writes nothing.
///
/// # Errors
/// Whatever `api` returns for a page.
pub async fn plan(db: &Db, api: &impl LinearApi) -> Result<ImportPlan> {
    let mut plan = ImportPlan {
        source: "linear".into(),
        ..ImportPlan::default()
    };
    let as_points = !matches!(
        crate::settings::get::<String>(db, "estimation")
            .await?
            .as_deref(),
        Some("time" | "none")
    );
    let (mut after, mut row, mut skipped) = (Value::Null, 0u32, vec![]);
    loop {
        let data = api.query(QUERY, json!({ "after": after })).await?;
        let page = &data["issues"];
        for n in page["nodes"].as_array().into_iter().flatten() {
            row += 1;
            if let Err(reason) = add_node(&mut plan, n, as_points) {
                skipped.push(Skipped { row, reason });
            }
        }
        after = page["pageInfo"]["endCursor"].clone();
        if page["pageInfo"]["hasNextPage"] != true || after.is_null() {
            break;
        }
    }
    finish(db, &mut plan).await?;
    plan.skipped.splice(0..0, skipped);
    Ok(plan)
}
