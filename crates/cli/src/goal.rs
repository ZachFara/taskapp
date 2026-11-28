//! `taskapp goal create|list|set|link|unlink`: quarterly goals that projects work towards.

use clap::Subcommand;
use serde_json::{Value, json};
use taskapp_core::{ChangeSource, Db, Error, GoalId, GoalStatus, Result, goals};

const SRC: ChangeSource = ChangeSource::Cli;

#[derive(Subcommand)]
pub enum GoalCmd {
    /// Creates a goal for a quarter.
    Create {
        title: String,
        /// The quarter, like 2027-Q1.
        #[arg(long)]
        quarter: String,
        #[arg(long)]
        description: Option<String>,
    },
    /// Lists goals by quarter, with the linked project ids and progress as [done, total] issues.
    List,
    /// Edits a goal; the status is active, achieved or dropped.
    Set {
        id: i64,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        quarter: Option<String>,
        #[arg(long)]
        description: Option<String>,
    },
    /// Links a project (slug or name) to a goal (id).
    Link { project: String, goal: i64 },
    /// Removes the link between a project and a goal.
    Unlink { project: String, goal: i64 },
}

fn val<T: serde::Serialize>(v: T) -> Result<Value> {
    serde_json::to_value(v).map_err(|e| Error::Invalid(e.to_string()))
}

pub async fn run(db: &Db, cmd: GoalCmd) -> Result<Value> {
    match cmd {
        GoalCmd::Create {
            title,
            quarter,
            description,
        } => val(goals::create(db, &title, &quarter, description, SRC).await?),
        GoalCmd::List => {
            let mut out = Vec::new();
            for g in goals::list(db).await? {
                let mut row = val(&g)?;
                row["progress"] = json!(goals::progress(db, g.id).await?);
                out.push(row);
            }
            Ok(Value::Array(out))
        }
        GoalCmd::Set {
            id,
            status,
            title,
            quarter,
            description,
        } => {
            let mut g = goals::get(db, GoalId(id)).await?;
            if let Some(s) = status {
                g.status = match s.as_str() {
                    "active" => GoalStatus::Active,
                    "achieved" => GoalStatus::Achieved,
                    "dropped" => GoalStatus::Dropped,
                    other => {
                        return Err(Error::Invalid(format!(
                            "status is active, achieved or dropped, not {other}"
                        )));
                    }
                };
            }
            g.title = title.unwrap_or(g.title);
            g.quarter = quarter.unwrap_or(g.quarter);
            g.description = description.unwrap_or(g.description);
            val(goals::update(db, g, SRC).await?)
        }
        GoalCmd::Link { project, goal } => {
            let p = project_id(db, &project).await?;
            goals::link(db, GoalId(goal), p, SRC).await?;
            val(goals::get(db, GoalId(goal)).await?)
        }
        GoalCmd::Unlink { project, goal } => {
            let p = project_id(db, &project).await?;
            goals::unlink(db, GoalId(goal), p, SRC).await?;
            val(goals::get(db, GoalId(goal)).await?)
        }
    }
}

async fn project_id(db: &Db, name: &str) -> Result<taskapp_core::ProjectId> {
    crate::references::project(db, Some(name))
        .await?
        .ok_or_else(|| Error::NotFound(format!("project {name}")))
}
