//! `taskapp decision ...`: raise, list, read, answer and withdraw decisions (the owner usually answers in the app).

use clap::Subcommand;
use serde_json::{Value, json};
use taskapp_core::{
    ChangeSource, Db, DecisionId, DecisionInput, DecisionMode, DecisionOption, DecisionStatus,
    Error, Result, decisions, issues,
};

#[derive(Subcommand)]
pub enum DecisionCmd {
    /// Raises a decision for the owner.
    Create {
        question: String,
        /// An option as KEY="label: trade-off"; repeat for each option.
        #[arg(long = "option", value_name = "KEY=TEXT")]
        options: Vec<String>,
        /// The key of the option you recommend.
        #[arg(long)]
        recommend: Option<String>,
        /// blocking (you wait for the answer) or assume (you carry on with your recommendation).
        #[arg(long, default_value = "assume")]
        mode: String,
        /// Background for the owner, markdown.
        #[arg(long)]
        context: Option<String>,
        /// Who raises it. Defaults to cli.
        #[arg(long)]
        source: Option<String>,
        /// Your own id for it, unique per source; raising the same one twice is refused.
        #[arg(long)]
        external_id: Option<String>,
        /// An issue key this is about; repeat for several.
        #[arg(long = "issue", value_name = "KEY")]
        issues: Vec<String>,
    },
    /// Lists decisions, newest first, blocking ones first.
    List {
        /// open, answered, withdrawn or all.
        #[arg(long, default_value = "open")]
        status: String,
    },
    Get {
        id: i64,
    },
    /// Answers an open decision with an option key or free text.
    Answer {
        id: i64,
        #[arg(long)]
        option: Option<String>,
        #[arg(long)]
        text: Option<String>,
        #[arg(long)]
        note: Option<String>,
    },
    /// Closes an open decision that no longer applies.
    Withdraw {
        id: i64,
    },
}

fn option(spec: &str) -> Result<DecisionOption> {
    let (key, text) = spec
        .split_once('=')
        .filter(|(k, _)| !k.trim().is_empty())
        .ok_or_else(|| {
            Error::Invalid(format!(
                "an option looks like KEY=\"label: trade-off\", got {spec}"
            ))
        })?;
    let (label, tradeoff) = text.split_once(':').unwrap_or((text, ""));
    Ok(DecisionOption {
        key: key.trim().into(),
        label: label.trim().into(),
        tradeoff: tradeoff.trim().into(),
    })
}

fn val<T: serde::Serialize>(v: T) -> Result<Value> {
    serde_json::to_value(v).map_err(|e| Error::Invalid(e.to_string()))
}

async fn raise(db: &Db, cmd: DecisionCmd) -> Result<Value> {
    let DecisionCmd::Create {
        question,
        options,
        recommend,
        mode,
        context,
        source,
        external_id,
        issues: keys,
    } = cmd
    else {
        return Err(Error::Invalid("not a create".into()));
    };
    let mode = match mode.as_str() {
        "blocking" => DecisionMode::Blocking,
        "assume" => DecisionMode::Assume,
        other => {
            return Err(Error::Invalid(format!(
                "mode is blocking or assume, not {other}"
            )));
        }
    };
    let mut issue_ids = Vec::new();
    for k in &keys {
        issue_ids.push(issues::get_by_key(db, k).await?.id);
    }
    let input = DecisionInput {
        question,
        context: context.unwrap_or_default(),
        options: options.iter().map(|o| option(o)).collect::<Result<_>>()?,
        recommendation: recommend,
        mode,
        source,
        external_id,
        issue_ids,
    };
    val(decisions::create(db, input, ChangeSource::Cli).await?)
}

pub async fn run(db: &Db, cmd: DecisionCmd) -> Result<Value> {
    const SRC: ChangeSource = ChangeSource::Cli;
    match cmd {
        create @ DecisionCmd::Create { .. } => raise(db, create).await,
        DecisionCmd::List { status } => {
            let status = match status.as_str() {
                "all" => None,
                "open" => Some(DecisionStatus::Open),
                "answered" => Some(DecisionStatus::Answered),
                "withdrawn" => Some(DecisionStatus::Withdrawn),
                other => {
                    return Err(Error::Invalid(format!(
                        "status is open, answered, withdrawn or all, not {other}"
                    )));
                }
            };
            val(decisions::list(db, status).await?)
        }
        DecisionCmd::Get { id } => val(decisions::get(db, DecisionId(id)).await?),
        DecisionCmd::Answer {
            id,
            option,
            text,
            note,
        } => val(decisions::answer(
            db,
            DecisionId(id),
            option,
            text,
            note.unwrap_or_default(),
            SRC,
        )
        .await?),
        DecisionCmd::Withdraw { id } => {
            decisions::withdraw(db, DecisionId(id), SRC).await?;
            Ok(json!({ "withdrawn": id }))
        }
    }
}

/// The table view of `decision list`: options as `a) Label; b) Label`. Trade-offs stay in `decision get`.
pub fn table(list: Value) -> Value {
    let Value::Array(rows) = &list else {
        return list;
    };
    rows.iter()
        .map(|d| {
            let options = d["options"]
                .as_array()
                .map(|opts| {
                    opts.iter()
                        .map(|o| {
                            format!(
                                "{}) {}",
                                o["key"].as_str().unwrap_or(""),
                                o["label"].as_str().unwrap_or("")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("; ")
                })
                .unwrap_or_default();
            json!({
                "id": d["id"],
                "mode": d["mode"],
                "status": d["status"],
                "question": d["question"],
                "options": options,
                "recommendation": d["recommendation"],
            })
        })
        .collect::<Vec<_>>()
        .into()
}
