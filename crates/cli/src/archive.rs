//! `taskapp archive list|empty` and `taskapp issue delete`: removing issues for good, never without `--yes`.

use serde_json::{Value, json};
use taskapp_core::{ChangeSource, Db, Error, Result, ViewDef, delete, issues, views};

const SRC: ChangeSource = ChangeSource::Cli;

#[derive(clap::Subcommand)]
pub enum ArchiveCmd {
    /// Archived issues, newest first.
    List,
    /// Delete every archived issue for good (one undo step). Needs --yes.
    Empty {
        #[arg(long)]
        yes: bool,
    },
}

fn sure(yes: bool, what: &str) -> Result<()> {
    if yes {
        return Ok(());
    }
    Err(Error::Invalid(format!(
        "{what} for good; it can be undone only with `taskapp undo` right after. Rerun with --yes"
    )))
}

pub async fn run(db: &Db, cmd: ArchiveCmd) -> Result<Value> {
    match cmd {
        ArchiveCmd::List => {
            let def: ViewDef = serde_json::from_value(json!({
                "filter": {"Cond": {"field": {"Fixed": "Archived"}, "op": "Eq", "value": true}},
                "sorts": [], "group_by": null, "layout": "List", "columns": [], "include_archived": true,
            }))
            .map_err(|e| Error::Invalid(e.to_string()))?;
            let mut rows = views::query(db, &def).await?;
            rows.sort_by_key(|i| std::cmp::Reverse(i.archived_at));
            Ok(rows
                .iter()
                .map(|i| json!({"key": i.key, "title": i.title, "archived_at": i.archived_at}))
                .collect())
        }
        ArchiveCmd::Empty { yes } => {
            sure(yes, "archive empty deletes every archived issue")?;
            Ok(json!({"deleted": delete::empty_archive(db, SRC).await?}))
        }
    }
}

pub async fn delete_issue(db: &Db, key: &str, yes: bool) -> Result<Value> {
    sure(yes, &format!("issue delete removes {key}"))?;
    let id = issues::get_by_key(db, key).await?.id;
    delete::remove(db, &[id], SRC).await?;
    Ok(json!({"deleted": key}))
}
