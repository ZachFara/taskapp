//! Command-line adapter over `taskapp-core`. Every write uses `ChangeSource::Cli`.

mod api;
mod commands;
mod list;
mod references;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use taskapp_core::{Db, Error, Result, backup, history};

#[derive(Parser)]
#[command(name = "taskapp", about = "Manage Cairn issues from the terminal")]
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
    /// Explore and call the application's core operations using JSON.
    #[command(subcommand)]
    Api(ApiCmd),
    /// Show a practical guide to the CLI.
    Guide,
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
    Status(StatusCmd),
    #[command(subcommand)]
    Property(PropertyCmd),
    #[command(subcommand)]
    Project(ProjectCmd),
    #[command(subcommand)]
    Cycle(CycleCmd),
    #[command(subcommand)]
    Event(EventCmd),
    #[command(subcommand)]
    Label(LabelCmd),
    #[command(subcommand)]
    Comment(CommentCmd),
    #[command(subcommand)]
    Relation(RelationCmd),
    #[command(subcommand)]
    Pin(PinCmd),
    #[command(subcommand)]
    Attachment(AttachmentCmd),
    #[command(subcommand)]
    Template(TemplateCmd),
    /// Progress and velocity over time, rebuilt from history.
    #[command(subcommand)]
    Insights(InsightsCmd),
    /// Change history of an issue.
    History { key: String },
    /// Undo the newest step, whatever made it (UI, agent or CLI).
    Undo,
    /// Redo the step most recently undone.
    Redo,
    #[command(subcommand)]
    Backup(BackupCmd),
    /// Builds a new database from an export folder. Needs --db, which must not exist yet.
    Restore {
        #[arg(long, value_name = "DIR")]
        from: PathBuf,
    },
    #[command(subcommand)]
    Import(ImportCmd),
    /// First-run setup: workspace name, issue key prefix (2 to 5 capital letters) and time zone.
    Setup {
        name: String,
        key: String,
        /// IANA time zone. Defaults to the zone already stored, then the system zone.
        #[arg(long)]
        tz: Option<String>,
    },
}

#[derive(Subcommand)]
enum ApiCmd {
    /// List available operations.
    List,
    /// Show an operation's argument names and types.
    Describe { name: String },
    /// Call an operation with a JSON object of named arguments.
    Call {
        name: String,
        #[arg(long, default_value = "{}")]
        args: String,
    },
}

/// Both sources print the plan and write nothing unless `--apply` is given.
#[derive(Subcommand)]
enum ImportCmd {
    /// Import issues from a CSV file with a header row.
    Csv {
        file: PathBuf,
        /// Column to field, repeatable: title, description, status, priority, project, labels,
        /// `start_date`, `due_date`, `estimate` (minutes) or `property:<name>`. Guessed from headers when absent.
        #[arg(long = "map", value_name = "COLUMN=FIELD")]
        map: Vec<String>,
        /// Write the plan instead of only showing it.
        #[arg(long)]
        apply: bool,
    },
    /// Import issues from Linear, with the token in `LINEAR_API_KEY`.
    Linear {
        #[arg(long)]
        apply: bool,
    },
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
        /// Status id or exact name (case-insensitive).
        #[arg(long)]
        status: Option<String>,
        /// Project id, slug or exact name (case-insensitive).
        #[arg(long)]
        project: Option<String>,
        /// Due date, YYYY-MM-DD.
        #[arg(long)]
        due: Option<String>,
        /// Estimate in minutes.
        #[arg(long)]
        estimate: Option<u32>,
        /// Points: 0 or more, one decimal.
        #[arg(long)]
        points: Option<f64>,
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
        /// Cycle id or exact name (case-insensitive).
        #[arg(long)]
        cycle: Option<String>,
        /// Label id or exact name (case-insensitive); repeat for several.
        #[arg(long = "label")]
        labels: Vec<String>,
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
        /// Status id or exact name (case-insensitive).
        #[arg(long)]
        status: Option<String>,
        /// Due date, YYYY-MM-DD.
        #[arg(long, conflicts_with = "clear_due")]
        due: Option<String>,
        /// Remove the due date.
        #[arg(long)]
        clear_due: bool,
        /// Points: 0 or more, one decimal.
        #[arg(long, conflicts_with = "clear_points")]
        points: Option<f64>,
        /// Remove the points.
        #[arg(long)]
        clear_points: bool,
        /// Start date, YYYY-MM-DD.
        #[arg(long)]
        start: Option<String>,
        /// Estimate in minutes.
        #[arg(long)]
        estimate: Option<u32>,
        /// Project id, slug or exact name (case-insensitive).
        #[arg(long)]
        project: Option<String>,
    },
    /// Issues filtered by project, status, priority and label names, most urgent first.
    List(list::ListArgs),
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

/// What a chart covers: exactly one of the three ids.
#[derive(clap::Args)]
struct ScopeArgs {
    #[arg(long, group = "scope")]
    project: Option<i64>,
    #[arg(long, group = "scope")]
    cycle: Option<i64>,
    #[arg(long, group = "scope")]
    view: Option<i64>,
    /// points, count or time (minutes).
    #[arg(long, default_value = "points")]
    measure: String,
}

#[derive(Subcommand)]
enum InsightsCmd {
    /// Per day scope, started and completed; the range defaults to the scope's own dates.
    Progress {
        #[command(flatten)]
        scope: ScopeArgs,
        /// First day, YYYY-MM-DD.
        #[arg(long)]
        from: Option<String>,
        /// Last day, YYYY-MM-DD.
        #[arg(long)]
        to: Option<String>,
    },
    /// Completed per cycle, or per week when there are no cycles.
    Velocity {
        #[command(flatten)]
        scope: ScopeArgs,
        #[arg(long, default_value_t = 6)]
        periods: u32,
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
    /// List view templates.
    Templates,
    /// Make a template from a saved view.
    SaveAsTemplate {
        view_id: i64,
        #[arg(long)]
        name: Option<String>,
    },
    /// Make a saved view from a template.
    FromTemplate {
        template_id: i64,
        #[arg(long)]
        name: Option<String>,
    },
    /// Put a view made from a template back to the template's definition.
    Reset {
        view_id: i64,
    },
    DeleteTemplate {
        id: i64,
    },
    /// List saved views with their definitions.
    List,
    /// Create a saved view from a JSON definition, or replace the one with `--id`.
    Save {
        name: String,
        #[arg(long)]
        def: String,
        #[arg(long)]
        id: Option<i64>,
        /// What a new view lists: `issue` (default) or `project`. A saved view keeps its own.
        #[arg(long, default_value = "issue")]
        entity: String,
    },
}

#[derive(Subcommand)]
enum StatusCmd {
    List,
    /// Create an issue status. Category: Backlog, Unstarted, Started, Completed or Canceled.
    Create {
        name: String,
        #[arg(long)]
        category: String,
        #[arg(long, default_value = "gray")]
        color: String,
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
        /// issues (default), projects or both.
        #[arg(long, default_value = "issues")]
        applies_to: String,
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
    /// Project statuses.
    Statuses,
    /// Create a project status. Category: Planned, Started, Paused, Completed or Cancelled.
    StatusCreate {
        name: String,
        #[arg(long)]
        category: String,
        #[arg(long, default_value = "gray")]
        color: String,
    },
    /// Change a project's status, priority, labels or a custom property value.
    Set {
        slug: String,
        /// Project status name.
        #[arg(long)]
        status: Option<String>,
        /// 0 none, 1 urgent, 2 high, 3 medium, 4 low.
        #[arg(long)]
        priority: Option<u8>,
        /// Label name; repeat for several. Replaces the project's labels.
        #[arg(long = "label")]
        labels: Vec<String>,
        /// Property key and JSON value, e.g. `--property owner '"sam"'`; `null` clears.
        #[arg(long, num_args = 2, value_names = ["KEY", "JSON"])]
        property: Vec<String>,
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
enum EventCmd {
    /// Events overlapping a date range.
    List {
        /// YYYY-MM-DD.
        #[arg(long)]
        from: Option<String>,
        /// YYYY-MM-DD.
        #[arg(long)]
        to: Option<String>,
    },
    Create {
        title: String,
        /// Local time, YYYY-MM-DDTHH:MM or YYYY-MM-DD.
        #[arg(long)]
        start: String,
        /// Local time, same form as --start.
        #[arg(long)]
        end: Option<String>,
        #[arg(long)]
        all_day: bool,
        #[arg(long)]
        place: Option<String>,
        #[arg(long)]
        notes: Option<String>,
    },
    /// Link an event to an issue.
    Link {
        /// Event id.
        id: i64,
        /// Issue key.
        key: String,
    },
}

#[derive(Subcommand)]
enum TemplateCmd {
    List,
    Create {
        #[command(flatten)]
        t: TemplateArgs,
    },
    Update {
        id: i64,
        #[command(flatten)]
        t: TemplateArgs,
    },
    Delete {
        id: i64,
    },
    /// Creates the issue and its sub-issues. TITLE is added to the template's title prefix.
    Apply {
        id: i64,
        title: Option<String>,
    },
}

#[derive(clap::Args)]
struct TemplateArgs {
    name: String,
    /// Used as /slug in quick-add.
    slug: String,
    #[arg(long)]
    icon: Option<String>,
    #[arg(long)]
    color: Option<String>,
    /// JSON: `{"title_prefix", "fields": {...}, "sub_issues": [{"title", "fields"}]}`.
    #[arg(long, default_value = "{}")]
    body: String,
}

impl TemplateArgs {
    fn input(self) -> Result<taskapp_core::TemplateInput> {
        let body =
            serde_json::from_str(&self.body).map_err(|e| Error::Invalid(format!("--body: {e}")))?;
        Ok(taskapp_core::TemplateInput {
            name: self.name,
            slug: self.slug,
            icon: self.icon,
            color: self.color,
            body,
        })
    }
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
    /// Put a label on an issue, by the label's name (any case).
    Add {
        key: String,
        label: String,
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
        /// A relation kind id or name: blocks, related, duplicate, follows, `caused_by`, `part_of`, or a custom one.
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
enum AttachmentCmd {
    /// Absolute path of an attachment's image file.
    Path { id: i64 },
    /// Attachments an issue's description points at with `(attachment:N)`.
    List { key: String },
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
    match &cli.cmd {
        Cmd::Guide => {
            print_value(&api::guide(), cli.json);
            return Ok(());
        }
        Cmd::Api(ApiCmd::List) => {
            print_value(&api::names(), cli.json);
            return Ok(());
        }
        Cmd::Api(ApiCmd::Describe { name }) => {
            print_value(&api::describe(name)?, cli.json);
            return Ok(());
        }
        _ => {}
    }
    // Backup commands run before the database opens: restore must not hold a pool on its target.
    if let Cmd::Backup(cmd) = &cli.cmd {
        let value = backup_cmd(cmd, cli.db.as_deref()).await?;
        print_value(&value, cli.json);
        return Ok(());
    }
    if let Cmd::Restore { from } = &cli.cmd {
        let to = cli
            .db
            .as_deref()
            .ok_or_else(|| Error::Invalid("restore needs --db <new database path>".into()))?;
        taskapp_export::rebuild::rebuild(from, to).await?;
        let (from, to) = (from.display().to_string(), to.display().to_string());
        print_value(&json!({"restored": to, "from": from}), cli.json);
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
    let label = match &cli.cmd {
        Cmd::Api(ApiCmd::Call { name, .. }) => format!("api call {name}"),
        _ => words.join(" "),
    };
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
    if !rows.is_empty() && rows.iter().all(|row| !row.is_object()) {
        return rows.iter().map(cell).collect::<Vec<_>>().join("\n") + "\n";
    }
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
        Value::Array(items) if items.iter().all(|v| !v.is_object() && !v.is_array()) => {
            items.iter().map(cell).collect::<Vec<_>>().join(", ")
        }
        other => other.to_string(),
    }
}
