//! Command-line adapter over `taskapp-core`. Every write uses `ChangeSource::Cli`.

mod commands;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use taskapp_core::{Db, Error, Result, backup, history};

#[derive(Parser)]
#[command(name = "taskapp", about = "Manage taskapp issues from the terminal")]
struct Cli {
    /// Database file. Defaults to the app's database.
    #[arg(long, global = true, value_name = "PATH")]
    db: Option<PathBuf>,
    /// Print JSON instead of a table.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create an issue from quick-add text, such as "Pay rent friday p1".
    Add {
        text: String,
        /// IANA time zone for relative dates. Defaults to $TZ, then UTC.
        #[arg(long)]
        tz: Option<String>,
    },
    #[command(subcommand)]
    Issue(IssueCmd),
    #[command(subcommand)]
    View(ViewCmd),
    #[command(subcommand)]
    Property(PropertyCmd),
    #[command(subcommand)]
    Project(ProjectCmd),
    #[command(subcommand)]
    Cycle(CycleCmd),
    #[command(subcommand)]
    Label(LabelCmd),
    #[command(subcommand)]
    Comment(CommentCmd),
    #[command(subcommand)]
    Relation(RelationCmd),
    #[command(subcommand)]
    Pin(PinCmd),
    /// Change history of an issue.
    History { key: String },
    /// Undo the newest step, whatever made it (UI, agent or CLI).
    Undo,
    /// Redo the step most recently undone.
    Redo,
    #[command(subcommand)]
    Backup(BackupCmd),
}

#[derive(Subcommand)]
enum BackupCmd {
    /// Snapshots and nightly backups under a backup directory.
    List {
        #[arg(long, value_name = "DIR")]
        dir: PathBuf,
    },
    /// Replaces the database with a backup. Quit the app first. Needs --yes.
    Restore {
        file: PathBuf,
        /// Confirms the replacement of the target database.
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum IssueCmd {
    Create {
        title: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        priority: Option<u8>,
        /// Status id.
        #[arg(long)]
        status: Option<i64>,
        /// Project id.
        #[arg(long)]
        project: Option<i64>,
        /// Due date, YYYY-MM-DD.
        #[arg(long)]
        due: Option<String>,
        /// Estimate in minutes.
        #[arg(long)]
        estimate: Option<u32>,
        /// Original creation time, RFC 3339.
        #[arg(long)]
        created_at: Option<String>,
        /// Original completion time, RFC 3339.
        #[arg(long)]
        completed_at: Option<String>,
        /// Custom property as key=value (value is JSON, else text); repeat for several.
        #[arg(long = "property")]
        properties: Vec<String>,
        /// Parent issue key.
        #[arg(long)]
        parent: Option<String>,
        /// Cycle id.
        #[arg(long)]
        cycle: Option<i64>,
        /// Label id; repeat for several.
        #[arg(long = "label")]
        labels: Vec<i64>,
        /// Record the Migration source instead of Cli.
        #[arg(long)]
        migration: bool,
    },
    Update {
        key: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        priority: Option<u8>,
        /// Status id.
        #[arg(long)]
        status: Option<i64>,
        /// Due date, YYYY-MM-DD.
        #[arg(long, conflicts_with = "clear_due")]
        due: Option<String>,
        /// Remove the due date.
        #[arg(long)]
        clear_due: bool,
    },
    Get {
        key: String,
    },
    Archive {
        key: String,
    },
    Search {
        text: String,
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
}

#[derive(Subcommand)]
enum ViewCmd {
    /// Run a view: an ad hoc JSON definition, or a saved view by id.
    Query {
        #[arg(long, conflicts_with = "id")]
        def: Option<String>,
        #[arg(long)]
        id: Option<i64>,
    },
}

#[derive(Subcommand)]
enum PropertyCmd {
    List,
    Create {
        key: String,
        name: String,
        /// text, number, date, select, multiselect, checkbox, url, notelink.
        kind: String,
        /// Select option value; repeat for several.
        #[arg(long = "option")]
        options: Vec<String>,
    },
    Rename {
        key: String,
        name: String,
    },
    /// Snapshots the database first.
    ChangeKind {
        key: String,
        kind: String,
        #[arg(long = "option")]
        options: Vec<String>,
    },
    /// Snapshots the database first.
    Delete {
        key: String,
    },
    /// Set a value on an issue; the value is JSON, `null` clears it.
    Set {
        issue: String,
        key: String,
        value: String,
    },
}

#[derive(Subcommand)]
enum ProjectCmd {
    List,
    Create {
        slug: String,
        name: String,
        #[arg(long)]
        icon: Option<String>,
        #[arg(long)]
        color: Option<String>,
    },
}

#[derive(Subcommand)]
enum CycleCmd {
    List,
    Create {
        /// YYYY-MM-DD.
        #[arg(long)]
        start: String,
        /// YYYY-MM-DD.
        #[arg(long)]
        end: String,
        #[arg(long)]
        name: Option<String>,
        /// Palette key or #rrggbb.
        #[arg(long)]
        color: Option<String>,
        /// icon:<id> or emoji:<emoji>.
        #[arg(long)]
        icon: Option<String>,
    },
}

#[derive(Subcommand)]
enum LabelCmd {
    List,
    Create {
        name: String,
        color: String,
        #[arg(long)]
        icon: Option<String>,
    },
}

#[derive(Subcommand)]
enum CommentCmd {
    Add {
        key: String,
        body: String,
        /// Original creation time, RFC 3339.
        #[arg(long)]
        created_at: Option<String>,
        /// Record the Migration source instead of Cli.
        #[arg(long)]
        migration: bool,
    },
}

#[derive(Subcommand)]
enum RelationCmd {
    Add {
        from: String,
        /// blocks, related, duplicate.
        kind: String,
        to: String,
    },
    Remove {
        from: String,
        kind: String,
        to: String,
    },
}

#[derive(Subcommand)]
enum PinCmd {
    List,
    /// kind: issue, project, view, cycle, note, url.
    Add {
        kind: String,
        target: String,
        #[arg(long)]
        title: Option<String>,
    },
    Remove {
        id: i64,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let json = cli.json;
    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            report(&err, json);
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<()> {
    // Backup commands run before the database opens: restore must not hold a pool on its target.
    if let Cmd::Backup(cmd) = &cli.cmd {
        let value = backup_cmd(cmd, cli.db.as_deref()).await?;
        print_value(&value, cli.json);
        return Ok(());
    }
    let path = match cli.db {
        Some(path) => path,
        None => default_db_path()?,
    };
    let dir = path
        .parent()
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    std::fs::create_dir_all(&dir)?;
    let db = Db::open(&path).await?;
    // Same folder the MCP server and the app list and prune.
    // One invocation is one undoable step, labelled with the command line minus the global flags.
    let mut args = std::env::args().skip(1);
    let mut words = Vec::new();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--db" => drop(args.next()),
            "--json" => {}
            _ => words.push(a),
        }
    }
    let label = words.join(" ");
    let value = history::step(&label, commands::run(&db, &dir.join("backups"), cli.cmd)).await?;
    print_value(&value, cli.json);
    Ok(())
}

async fn backup_cmd(cmd: &BackupCmd, db: Option<&Path>) -> Result<Value> {
    match cmd {
        BackupCmd::List { dir } => {
            let rows = backup::list(dir)?
                .into_iter()
                .map(|p| json!({"path": p.display().to_string()}))
                .collect();
            Ok(Value::Array(rows))
        }
        BackupCmd::Restore { file, yes } => {
            let target = match db {
                Some(path) => path.to_path_buf(),
                None => default_db_path()?,
            };
            if !yes {
                return Err(Error::Invalid(format!(
                    "restore replaces {}; quit the app, then rerun with --yes",
                    target.display()
                )));
            }
            eprintln!("Quit the app before restoring; it must not be running.");
            backup::restore(file, &target).await?;
            Ok(
                json!({"restored": target.display().to_string(), "from": file.display().to_string()}),
            )
        }
    }
}

fn default_db_path() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| Error::Invalid("HOME is not set; pass --db".into()))?;
    Ok(PathBuf::from(home).join("Library/Application Support/taskapp/taskapp.db"))
}

fn report(err: &Error, json: bool) {
    if json {
        eprintln!(
            "{}",
            serde_json::json!({"kind": err.kind(), "message": err.to_string()})
        );
    } else {
        eprintln!("error: {err}");
    }
}

fn print_value(value: &Value, json: bool) {
    if json {
        println!("{value}");
    } else {
        print!("{}", render(value));
    }
}

/// Arrays of objects become a table, objects become `key: value` lines.
fn render(value: &Value) -> String {
    match value {
        Value::Array(rows) => render_table(rows),
        Value::Object(map) => map
            .iter()
            .map(|(k, v)| format!("{k}: {}\n", cell(v)))
            .collect::<Vec<_>>()
            .concat(),
        other => format!("{}\n", cell(other)),
    }
}

fn render_table(rows: &[Value]) -> String {
    let Some(first) = rows.first().and_then(Value::as_object) else {
        return "(none)\n".to_string();
    };
    let headers: Vec<String> = first.keys().cloned().collect();
    let mut table = vec![headers.clone()];
    for row in rows {
        table.push(
            headers
                .iter()
                .map(|h| row.get(h).map_or_else(String::new, cell))
                .collect(),
        );
    }
    let widths: Vec<usize> = (0..headers.len())
        .map(|i| {
            table
                .iter()
                .filter_map(|r| r.get(i))
                .map(|c| c.chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();
    table
        .iter()
        .map(|row| {
            let line: Vec<String> = row
                .iter()
                .zip(&widths)
                .map(|(c, &w)| format!("{c:<w$}"))
                .collect();
            format!("{}\n", line.join("  ").trim_end())
        })
        .collect::<Vec<_>>()
        .concat()
}

fn cell(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.replace('\n', " "),
        other => other.to_string(),
    }
}
