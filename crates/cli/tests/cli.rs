use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn fresh() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("taskapp.db");
    (dir, db)
}

fn run(db: &Path, args: &[&str]) -> Output {
    Command::cargo_bin("taskapp")
        .expect("binary")
        .arg("--db")
        .arg(db)
        .args(args)
        .output()
        .expect("run taskapp")
}

fn run_json(db: &Path, args: &[&str]) -> Value {
    let mut full = vec!["--json"];
    full.extend_from_slice(args);
    let out = run(db, &full);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("stdout is JSON")
}

#[test]
fn add_quickadd_prints_key() {
    let (_d, db) = fresh();
    let out = run(&db, &["add", "Buy milk tomorrow"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("ABC-1"), "stdout was: {stdout}");
}

#[test]
fn json_output_parses() {
    let (_d, db) = fresh();
    run(&db, &["add", "Write report"]);
    let issue = run_json(&db, &["issue", "get", "ABC-1"]);
    assert_eq!(issue["key"], "ABC-1");
}

#[test]
fn issue_update_by_key() {
    let (_d, db) = fresh();
    run(&db, &["add", "Write report"]);
    let out = run(&db, &["issue", "update", "ABC-1", "--priority", "2"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let issue = run_json(&db, &["issue", "get", "ABC-1"]);
    assert_eq!(issue["priority"], 2);
}

#[test]
fn undo_and_redo_revert_a_cli_edit() {
    let (_d, db) = fresh();
    run(&db, &["add", "Write report"]);
    run(&db, &["issue", "update", "ABC-1", "--priority", "2"]);
    let s = run_json(&db, &["undo"]);
    assert_eq!(s["redo"], "issue update ABC-1 --priority 2");
    assert_eq!(run_json(&db, &["issue", "get", "ABC-1"])["priority"], 0);
    run_json(&db, &["redo"]);
    assert_eq!(run_json(&db, &["issue", "get", "ABC-1"])["priority"], 2);
}

#[test]
fn issue_get_reports_checklist_progress() {
    let (_d, db) = fresh();
    run(&db, &["add", "Trip"]);
    let out = run(
        &db,
        &[
            "issue",
            "update",
            "ABC-1",
            "--description",
            "Packing\n- [x] pack\n- [ ] book",
        ],
    );
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let issue = run_json(&db, &["issue", "get", "ABC-1"]);
    assert_eq!(issue["checklist"]["done"], 1);
    assert_eq!(issue["checklist"]["total"], 2);
}

#[test]
fn unknown_key_exits_nonzero_with_message() {
    let (_d, db) = fresh();
    let out = run(&db, &["issue", "get", "ABC-999"]);
    assert_ne!(out.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("not found"), "stderr was: {stderr}");
}

#[test]
fn property_change_kind_snapshots_first() {
    let (dir, db) = fresh();
    let create = run(&db, &["property", "create", "note", "Note", "text"]);
    assert!(
        create.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&create.stderr)
    );
    let change = run(&db, &["property", "change-kind", "note", "number"]);
    assert!(
        change.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&change.stderr)
    );
    let snapshots = dir.path().join("backups/snapshots");
    let count = std::fs::read_dir(&snapshots).map_or(0, Iterator::count);
    assert!(count >= 1, "no snapshot in {}", snapshots.display());
}

#[test]
fn add_empty_text_exits_nonzero() {
    let (_d, db) = fresh();
    let out = run(&db, &["add", "   "]);
    assert_ne!(out.status.code(), Some(0));
}

#[test]
fn json_error_is_json_on_stderr() {
    let (_d, db) = fresh();
    let out = run(&db, &["--json", "issue", "get", "ABC-999"]);
    assert_ne!(out.status.code(), Some(0));
    let err: Value = serde_json::from_slice(&out.stderr).expect("stderr is JSON");
    assert_eq!(err["kind"], "NotFound");
    assert!(err["message"].is_string());
}

#[test]
fn unknown_subcommand_prints_usage_and_exits_2() {
    let (_d, db) = fresh();
    let out = run(&db, &["frobnicate"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("Usage"));
}

/// A snapshot of `db` made by a destructive command, as the app does before a property change.
fn make_snapshot(db: &Path) -> PathBuf {
    let create = run(db, &["property", "create", "note", "Note", "text"]);
    assert!(create.status.success());
    let change = run(db, &["property", "change-kind", "note", "number"]);
    assert!(change.status.success());
    let snaps = db.parent().expect("parent").join("backups/snapshots");
    std::fs::read_dir(&snaps)
        .expect("snapshots dir")
        .map(|e| e.expect("entry").path())
        .next()
        .expect("a snapshot file")
}

#[test]
fn backup_list_prints_snapshots() {
    let (dir, db) = fresh();
    make_snapshot(&db);
    let listed = run_json(
        &db,
        &[
            "backup",
            "list",
            "--dir",
            dir.path().join("backups").to_str().expect("utf8"),
        ],
    );
    let rows = listed.as_array().expect("array");
    assert_eq!(rows.len(), 1, "listed: {listed}");
    assert!(
        rows[0]["path"]
            .as_str()
            .expect("path")
            .contains("snapshots")
    );
}

#[test]
fn backup_restore_replaces_db() {
    let (_src, src_db) = fresh();
    run(&src_db, &["add", "One"]);
    run(&src_db, &["add", "Two"]);
    run(&src_db, &["add", "Three"]);
    let snapshot = make_snapshot(&src_db);

    let (_dst, dst_db) = fresh();
    run(&dst_db, &["add", "Only one"]);
    assert_ne!(
        run(&dst_db, &["issue", "get", "ABC-3"]).status.code(),
        Some(0)
    );

    let out = run(
        &dst_db,
        &[
            "backup",
            "restore",
            snapshot.to_str().expect("utf8"),
            "--yes",
        ],
    );
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let issue = run_json(&dst_db, &["issue", "get", "ABC-3"]);
    assert_eq!(issue["key"], "ABC-3");
}

#[test]
fn backup_restore_refuses_corrupt_file() {
    let (_d, db) = fresh();
    run(&db, &["add", "Keep me"]);
    let before = std::fs::read(&db).expect("db bytes");
    let corrupt = db.parent().expect("parent").join("corrupt.db");
    std::fs::write(&corrupt, b"not a database").expect("write");

    let out = run(
        &db,
        &[
            "backup",
            "restore",
            corrupt.to_str().expect("utf8"),
            "--yes",
        ],
    );
    assert_ne!(out.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("unrecognized"), "stderr was: {stderr}");
    assert_eq!(std::fs::read(&db).expect("db bytes"), before);
}

#[test]
fn backup_restore_without_yes_changes_nothing() {
    let (_src, src_db) = fresh();
    run(&src_db, &["add", "One"]);
    let snapshot = make_snapshot(&src_db);

    let (_dst, dst_db) = fresh();
    run(&dst_db, &["add", "Keep me"]);
    let before = std::fs::read(&dst_db).expect("db bytes");

    let out = run(
        &dst_db,
        &["backup", "restore", snapshot.to_str().expect("utf8")],
    );
    assert_ne!(out.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(&dst_db.display().to_string()),
        "stderr was: {stderr}"
    );
    assert_eq!(std::fs::read(&dst_db).expect("db bytes"), before);
}

fn history_sources(db: &Path, key: &str) -> Vec<Value> {
    let h = run_json(db, &["history", key]);
    h.as_array()
        .expect("history array")
        .iter()
        .map(|c| c["source"].clone())
        .collect()
}

#[test]
fn cli_writes_record_cli_source() {
    let (_d, db) = fresh();
    run(&db, &["add", "Write report"]);
    let sources = history_sources(&db, "ABC-1");
    assert_eq!(sources[0], "Cli");
    assert!(sources.iter().all(|s| s == "Cli"), "{sources:?}");
}

#[test]
fn property_delete_snapshots_first() {
    let (dir, db) = fresh();
    run(&db, &["property", "create", "note", "Note", "text"]);
    let del = run(&db, &["property", "delete", "note"]);
    assert!(del.status.success());
    let snaps = dir.path().join("backups/snapshots");
    let names: Vec<_> = std::fs::read_dir(&snaps)
        .expect("snapshots dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        names.len() == 1 && names[0].contains("delete-property"),
        "{names:?}"
    );
}

#[test]
fn failed_snapshot_aborts_property_delete() {
    let (dir, db) = fresh();
    run(&db, &["property", "create", "note", "Note", "text"]);
    std::fs::write(dir.path().join("backups"), "not a dir").expect("block backups");
    let del = run(&db, &["property", "delete", "note"]);
    assert_ne!(del.status.code(), Some(0));
    let props = run_json(&db, &["property", "list"]);
    assert!(
        props
            .as_array()
            .expect("array")
            .iter()
            .any(|p| p["key"] == "note"),
        "property was deleted despite the failed snapshot"
    );
}

#[test]
fn issue_create_accepts_migration_fields() {
    let (_d, db) = fresh();
    run(&db, &["add", "Parent"]);
    let label = run_json(&db, &["label", "create", "bug", "red"]);
    let cycle = run_json(
        &db,
        &[
            "cycle",
            "create",
            "--start",
            "2024-01-01",
            "--end",
            "2024-01-14",
        ],
    );
    run(
        &db,
        &["property", "create", "linear_id", "Linear ID", "text"],
    );
    let (label_id, cycle_id) = (label["id"].to_string(), cycle["id"].to_string());
    let made = run_json(
        &db,
        &[
            "issue",
            "create",
            "Old",
            "--created-at",
            "2024-01-02T03:04:05Z",
            "--completed-at",
            "2024-01-09T00:00:00Z",
            "--property",
            "linear_id=ENG-7",
            "--parent",
            "ABC-1",
            "--cycle",
            &cycle_id,
            "--label",
            &label_id,
            "--migration",
        ],
    );
    assert_eq!(made["created_at"], "2024-01-02T03:04:05Z");
    assert_eq!(made["completed_at"], "2024-01-09T00:00:00Z");
    assert_eq!(made["properties"]["linear_id"], "ENG-7");
    assert_eq!(made["cycle_id"], cycle["id"]);
    assert!(made["parent_id"].is_number(), "{made}");
    assert!(
        history_sources(&db, "ABC-2")
            .iter()
            .all(|s| s == "Migration")
    );
}

#[test]
fn comment_add_accepts_created_at_and_migration() {
    let (_d, db) = fresh();
    run(&db, &["add", "Parent"]);
    let c = run_json(
        &db,
        &[
            "comment",
            "add",
            "ABC-1",
            "hi",
            "--created-at",
            "2023-05-06T07:08:09Z",
            "--migration",
        ],
    );
    assert_eq!(c["created_at"], "2023-05-06T07:08:09Z");
}

#[test]
fn import_csv_shows_plan_then_applies() {
    let (dir, db) = fresh();
    let file = dir.path().join("in.csv");
    std::fs::write(&file, "Title,Tags\nOne,a\nTwo,b\n").expect("write csv");
    let path = file.to_str().expect("utf8 path");
    let plan = run_json(&db, &["import", "csv", path]);
    assert_eq!(plan["issues"].as_array().map(Vec::len), Some(2));
    assert!(!run(&db, &["issue", "get", "ABC-1"]).status.success());
    let done = run_json(&db, &["import", "csv", path, "--apply"]);
    assert_eq!(done["applied"]["created"], 2);
    assert_eq!(run_json(&db, &["issue", "get", "ABC-2"])["title"], "Two");
}

#[test]
fn setup_sets_the_key_for_new_issues() {
    let (_d, db) = fresh();
    let out = run_json(&db, &["setup", "Cairn", "XY", "--tz", "Europe/Berlin"]);
    assert_eq!(out["setup_done"], true);
    assert_eq!(out["key"], "XY");
    run(&db, &["add", "Buy milk"]);
    assert_eq!(run_json(&db, &["issue", "get", "XY-1"])["key"], "XY-1");
}

#[test]
fn a_project_view_is_saved_from_the_command_line() {
    let (_d, db) = fresh();
    let def = r#"{"filter":null,"sorts":[],"group_by":{"Fixed":"Status"},"layout":"Board","columns":[],"include_archived":false}"#;
    let v = run_json(
        &db,
        &["view", "save", "Mine", "--def", def, "--entity", "project"],
    );
    assert_eq!(v["entity"], "project");
    assert_eq!(
        run_json(&db, &["view", "save", "Plain", "--def", def])["entity"],
        "issue"
    );
}

#[test]
fn view_templates_from_the_command_line() {
    let (_d, db) = fresh();
    let t = run_json(&db, &["view", "save-as-template", "1", "--name", "ABC t"]);
    assert_eq!(t["name"], "ABC t");
    let v = run_json(&db, &["view", "from-template", "1"]);
    assert_eq!(v["template_id"], 1);
    run_json(&db, &["view", "reset", &v["id"].to_string()]);
    assert_eq!(
        run_json(&db, &["view", "templates"])
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    run_json(&db, &["view", "delete-template", "1"]);
    assert_eq!(
        run_json(&db, &["view", "templates"])
            .as_array()
            .map(Vec::len),
        Some(0)
    );
}

#[test]
fn points_flags_and_insights() {
    let (_d, db) = fresh();
    run_json(&db, &["project", "create", "p", "P"]);
    let made = run_json(
        &db,
        &[
            "issue",
            "create",
            "ABC a",
            "--project",
            "1",
            "--points",
            "3",
        ],
    );
    assert_eq!(made["points"], 3.0);
    let set = run_json(&db, &["issue", "update", "ABC-1", "--points", "0.5"]);
    assert_eq!(set["points"], 0.5);
    assert!(
        !run(&db, &["issue", "update", "ABC-1", "--points", "0.25"])
            .status
            .success()
    );
    let cleared = run_json(&db, &["issue", "update", "ABC-1", "--clear-points"]);
    assert_eq!(cleared["points"], Value::Null);
    let p = run_json(
        &db,
        &[
            "insights",
            "progress",
            "--project",
            "1",
            "--measure",
            "count",
            "--from",
            "2026-01-01",
            "--to",
            "2026-01-02",
        ],
    );
    assert_eq!(p["days"].as_array().map(Vec::len), Some(2));
    let v = run_json(
        &db,
        &["insights", "velocity", "--project", "1", "--periods", "3"],
    );
    assert_eq!(v.as_array().map(Vec::len), Some(3));
    assert!(
        !run(&db, &["insights", "progress", "--measure", "count"])
            .status
            .success()
    );
}

#[test]
fn events_are_created_linked_and_listed() {
    let (_d, db) = fresh();
    run(&db, &["add", "Parent"]);
    let made = run_json(
        &db,
        &[
            "event",
            "create",
            "Demo",
            "--start",
            "2026-12-10T09:30",
            "--place",
            "Room 4",
        ],
    );
    assert_eq!(made["starts_at"], "2026-12-10T09:30:00");
    let id = made["id"].to_string();
    run(&db, &["event", "link", &id, "ABC-1"]);
    let day = run_json(
        &db,
        &[
            "event",
            "list",
            "--from",
            "2026-12-10",
            "--to",
            "2026-12-10",
        ],
    );
    assert_eq!(day[0]["issue_ids"].as_array().map(Vec::len), Some(1));
    let later = run_json(&db, &["event", "list", "--from", "2026-12-11"]);
    assert_eq!(later.as_array().map(Vec::len), Some(0));
}

#[test]
fn restore_builds_a_new_database_from_an_export_and_refuses_an_existing_one() {
    let (_src, src_db) = fresh();
    run(&src_db, &["add", "Keep me"]);
    run(
        &src_db,
        &["event", "create", "Demo", "--start", "2026-12-10T09:30"],
    );
    let export = tempfile::tempdir().expect("tempdir");
    tokio::runtime::Runtime::new()
        .expect("runtime")
        .block_on(async {
            let db = taskapp_core::Db::open(&src_db).await.expect("open");
            taskapp_export::write::write_snapshot(&db, export.path())
                .await
                .expect("export");
        });
    let from = export.path().to_str().expect("utf8");

    let (_dst, dst_db) = fresh();
    let out = run(&dst_db, &["restore", "--from", from]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        run_json(&dst_db, &["issue", "get", "ABC-1"])["title"],
        "Keep me"
    );
    assert_eq!(run_json(&dst_db, &["event", "list"])[0]["title"], "Demo");

    let again = run(&dst_db, &["restore", "--from", from]);
    assert!(!again.status.success());
    assert!(String::from_utf8_lossy(&again.stderr).contains("already exists"));
}

#[test]
fn a_status_then_a_view_filtered_on_it_can_be_created() {
    let (_d, db) = fresh();
    let status = run_json(&db, &["status", "create", "Inbox", "--category", "Backlog"]);
    let id = status["id"].as_i64().expect("status id");
    let def = format!(
        r#"{{"filter":{{"Cond":{{"field":{{"Fixed":"Status"}},"op":"In","value":[{id}]}}}},"sorts":[],"group_by":null,"layout":"List","columns":[],"include_archived":false}}"#
    );
    let view = run_json(&db, &["view", "save", "Inbox", "--def", &def]);
    assert_eq!(view["name"], "Inbox");
    let views = run_json(&db, &["view", "list"]);
    assert!(
        views
            .as_array()
            .expect("views")
            .iter()
            .any(|v| v["name"] == "Inbox")
    );
    let project = run_json(
        &db,
        &[
            "project",
            "status-create",
            "Backlog",
            "--category",
            "Planned",
        ],
    );
    assert_eq!(project["name"], "Backlog");
    assert!(
        !run(&db, &["status", "create", "X", "--category", "nope"])
            .status
            .success()
    );
}

#[test]
fn replacing_a_view_keeps_its_icon() {
    let (_d, db) = fresh();
    let def = r#"{"filter":null,"sorts":[],"group_by":null,"layout":"List","columns":[],"include_archived":false}"#;
    let made = run_json(&db, &["view", "save", "Mine", "--def", def]);
    let id = made["id"].as_i64().expect("id");
    let conn = rusqlite_free_icon(&db, id);
    assert!(conn);
    let again = run_json(
        &db,
        &[
            "view",
            "save",
            "Mine",
            "--def",
            def,
            "--id",
            &id.to_string(),
        ],
    );
    assert_eq!(again["icon"], "icon:moon");
}

/// Sets an icon on a view straight in the database file (the CLI has no icon command).
fn rusqlite_free_icon(db: &Path, id: i64) -> bool {
    std::process::Command::new("sqlite3")
        .arg(db)
        .arg(format!(
            "UPDATE views SET icon = 'icon:moon' WHERE id = {id}"
        ))
        .status()
        .is_ok_and(|s| s.success())
}

#[test]
fn a_label_is_put_on_an_issue_by_its_name() {
    let (_d, db) = fresh();
    run_json(&db, &["add", "Write the docs"]);
    let label = run_json(&db, &["label", "create", "By AI", "purple"]);
    let issue = run_json(&db, &["label", "add", "ABC-1", "by ai"]);
    assert_eq!(issue["label_ids"], serde_json::json!([label["id"]]));
    assert!(
        !run(&db, &["label", "add", "ABC-1", "nope"])
            .status
            .success()
    );
}

#[test]
fn update_sets_the_start_date_and_estimate() {
    let (_d, db) = fresh();
    run_json(&db, &["add", "Plan it"]);
    let issue = run_json(
        &db,
        &[
            "issue",
            "update",
            "ABC-1",
            "--start",
            "2026-12-01",
            "--estimate",
            "960",
        ],
    );
    assert_eq!(issue["start_date"], "2026-12-01");
    assert_eq!(issue["estimate_minutes"], 960);
}

#[test]
fn issue_list_filters_by_names_and_prints_names() {
    let (_d, db) = fresh();
    run_json(&db, &["project", "create", "cairn", "Cairn"]);
    run_json(&db, &["label", "create", "By AI", "blue"]);
    let new = |title: &str, extra: &[&str]| {
        let mut args = vec!["issue", "create", title];
        args.extend_from_slice(extra);
        run_json(&db, &args)
    };
    new(
        "Urgent todo",
        &["--project", "Cairn", "--priority", "1", "--status", "Todo"],
    );
    new(
        "Done one",
        &["--project", "cairn", "--priority", "2", "--status", "Done"],
    );
    new(
        "Low backlog",
        &["--project", "Cairn", "--priority", "4", "--label", "By AI"],
    );
    new("Elsewhere", &["--priority", "1"]);

    let rows = run_json(
        &db,
        &[
            "issue",
            "list",
            "--project",
            "Cairn",
            "--not-status",
            "Done,Canceled",
        ],
    );
    let titles: Vec<&str> = rows
        .as_array()
        .expect("array")
        .iter()
        .map(|r| r["title"].as_str().expect("title"))
        .collect();
    assert_eq!(
        titles,
        ["Urgent todo", "Low backlog"],
        "sorted by priority, Done left out"
    );
    assert_eq!(rows[0]["status"], "Todo");
    assert_eq!(rows[0]["priority"], "Urgent");
    assert_eq!(rows[0]["project"], "Cairn");
    assert_eq!(rows[1]["labels"][0], "By AI");

    let ai = run_json(&db, &["issue", "list", "--label", "by ai"]);
    assert_eq!(ai.as_array().expect("array").len(), 1);
    let urgent = run_json(&db, &["issue", "list", "--priority", "urgent"]);
    assert_eq!(urgent.as_array().expect("array").len(), 2);

    let out = run(&db, &["issue", "list", "--status", "Nope"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("status Nope"));
}

#[test]
fn issue_get_prints_names_and_keeps_ids() {
    let (_d, db) = fresh();
    run_json(&db, &["project", "create", "cairn", "Cairn"]);
    run_json(&db, &["label", "create", "By AI", "blue"]);
    run_json(
        &db,
        &[
            "issue",
            "create",
            "Named",
            "--project",
            "Cairn",
            "--priority",
            "1",
            "--status",
            "Todo",
            "--label",
            "By AI",
        ],
    );

    let issue = run_json(&db, &["issue", "get", "ABC-1"]);
    assert_eq!(issue["key"], "ABC-1");
    assert!(issue["status_id"].is_number(), "ids stay in JSON");
    assert!(issue["project_id"].is_number());
    assert_eq!(issue["priority"], 1, "numeric priority stays in JSON");
    assert_eq!(issue["status"], "Todo");
    assert_eq!(issue["project"], "Cairn");
    assert_eq!(issue["labels"][0], "By AI");
    assert_eq!(issue["priority_name"], "Urgent");

    let table = run(&db, &["issue", "get", "ABC-1"]);
    let stdout = String::from_utf8_lossy(&table.stdout);
    assert!(stdout.contains("status: Todo"), "stdout was: {stdout}");
    assert!(
        stdout.contains("priority_name: Urgent"),
        "stdout was: {stdout}"
    );
}

#[test]
fn table_prints_list_values_as_plain_text() {
    let (_d, db) = fresh();
    run_json(&db, &["project", "create", "cairn", "Cairn"]);
    run_json(&db, &["label", "create", "By AI", "blue"]);
    run_json(&db, &["label", "create", "For AI", "green"]);
    run_json(
        &db,
        &[
            "issue", "create", "Tagged", "--label", "By AI", "--label", "For AI",
        ],
    );

    let table = run(&db, &["issue", "list"]);
    let stdout = String::from_utf8_lossy(&table.stdout);
    assert!(stdout.contains("By AI, For AI"), "stdout was: {stdout}");
    assert!(!stdout.contains("[\""), "stdout was: {stdout}");

    let json = run_json(&db, &["issue", "list"]);
    assert_eq!(json[0]["labels"][1], "For AI", "--json keeps the array");
}

/// A PNG signature is enough for the attachment check; the bytes are stored as given.
fn add_attachment(db: &Path) {
    let png = "[137,80,78,71,13,10,26,10]";
    let args = format!(r#"{{"bytes":{png},"mime":"image/png"}}"#);
    run_json(db, &["api", "call", "attachments_add", "--args", &args]);
}

#[test]
fn attachment_path_and_list_for_an_issue() {
    let (_d, db) = fresh();
    add_attachment(&db);
    run_json(
        &db,
        &[
            "issue",
            "create",
            "Shot",
            "--description",
            "See (attachment:1) here",
        ],
    );
    run_json(&db, &["issue", "create", "No picture"]);

    let path = run_json(&db, &["attachment", "path", "1"]);
    let path = PathBuf::from(path.as_str().expect("path"));
    assert!(path.is_absolute(), "path was {path:?}");
    assert_eq!(path.extension().and_then(|e| e.to_str()), Some("png"));
    assert!(path.exists(), "file is at {path:?}");
    assert_eq!(
        path.parent().expect("parent").file_name().expect("name"),
        "attachments"
    );

    let rows = run_json(&db, &["attachment", "list", "ABC-1"]);
    let rows = rows.as_array().expect("array");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"], 1);
    assert_eq!(rows[0]["mime"], "image/png");
    assert_eq!(rows[0]["bytes"], 8);
    assert_eq!(rows[0]["path"], path.display().to_string());

    let none = run_json(&db, &["attachment", "list", "ABC-2"]);
    assert_eq!(none.as_array().expect("array").len(), 0);

    let missing = run(&db, &["attachment", "path", "99"]);
    assert!(!missing.status.success());
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_now_reports_missing_reminders_access_from_the_helper() {
    use std::os::unix::fs::PermissionsExt;
    let (d, db) = fresh();
    {
        let conn = taskapp_core::Db::open(&db).await.expect("open");
        taskapp_core::settings::set(&conn, "reminders_list_id", &"L1")
            .await
            .expect("set");
    }
    let helper = d.path().join("helper");
    std::fs::write(
        &helper,
        "#!/bin/sh\necho '{\"error\":\"Reminders access is off: press Authorize in Settings > Reminders\"}'\nexit 1\n",
    )
    .expect("script");
    std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let out = run(
        &db,
        &["sync", "now", "--helper", helper.to_str().expect("utf8")],
    );
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("Reminders access is off: press Authorize"),
        "{err}"
    );
    assert!(!err.contains("reminders helper:"), "{err}");
}

#[test]
fn sync_now_with_nothing_configured_does_nothing() {
    let (_d, db) = fresh();
    let v = run_json(&db, &["sync", "now"]);
    assert_eq!(v["reminders"]["imported"], 0);
    assert_eq!(v["gcal"]["created"], 0);
}

fn raise(db: &Path) -> Value {
    run(db, &["add", "Pick a database"]);
    let args = [
        "decision",
        "create",
        "Which database?",
        "--option",
        "a=Postgres: heavier",
        "--option",
        "b=SQLite: simpler",
        "--recommend",
        "b",
        "--mode",
        "blocking",
        "--issue",
        "ABC-1",
        "--source",
        "agent:test",
        "--external-id",
        "x1",
    ];
    run_json(db, &args)
}

#[test]
fn decision_create_keeps_every_field() {
    let (_d, db) = fresh();
    let made = raise(&db);
    assert_eq!(made["id"], 1);
    assert_eq!(made["options"][1]["tradeoff"], "simpler");
    assert_eq!(made["mode"], "blocking");
    assert_eq!(made["source"], "agent:test");
    let dup = [
        "decision",
        "create",
        "Again",
        "--source",
        "agent:test",
        "--external-id",
        "x1",
    ];
    assert!(!run(&db, &dup).status.success());
}

#[test]
fn decision_answer_moves_it_out_of_the_open_list() {
    let (_d, db) = fresh();
    raise(&db);
    assert!(
        !run(&db, &["decision", "answer", "1", "--option", "zzz"])
            .status
            .success()
    );
    let done = run_json(
        &db,
        &["decision", "answer", "1", "--option", "b", "--note", "ok"],
    );
    assert_eq!(
        (
            done["status"].as_str(),
            done["answer"].as_str(),
            done["note"].as_str()
        ),
        (Some("answered"), Some("b"), Some("ok"))
    );
    assert_eq!(
        run_json(&db, &["decision", "list"])
            .as_array()
            .expect("array")
            .len(),
        0
    );
    assert_eq!(
        run_json(&db, &["decision", "list", "--status", "all"])
            .as_array()
            .expect("array")
            .len(),
        1
    );
    assert_eq!(
        run_json(&db, &["decision", "get", "1"])["issue_ids"]
            .as_array()
            .expect("array")
            .len(),
        1
    );
}

#[test]
fn decision_withdraw() {
    let (_d, db) = fresh();
    run_json(&db, &["decision", "create", "Skip it?"]);
    assert_eq!(
        run_json(&db, &["decision", "withdraw", "1"])["withdrawn"],
        1
    );
}

#[test]
fn decision_list_table_shows_options_as_labels() {
    let (_d, db) = fresh();
    raise(&db);
    let out = run(&db, &["decision", "list", "--status", "all"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let header = stdout.lines().next().expect("header");
    for col in [
        "id",
        "mode",
        "status",
        "question",
        "options",
        "recommendation",
    ] {
        assert!(header.contains(col), "header was: {header}");
    }
    assert!(
        stdout.contains("a) Postgres; b) SQLite"),
        "stdout was: {stdout}"
    );
    assert!(
        !stdout.contains('{'),
        "options must not be raw JSON: {stdout}"
    );
    assert!(
        !stdout.contains("heavier"),
        "trade-offs stay in get: {stdout}"
    );
}

#[test]
fn decision_list_json_keeps_full_detail() {
    let (_d, db) = fresh();
    raise(&db);
    let listed = run_json(&db, &["decision", "list", "--status", "all"]);
    assert_eq!(listed[0]["options"][0]["tradeoff"], "heavier");
}

#[test]
fn project_set_changes_look_name_and_dates() {
    let (_d, db) = fresh();
    run_json(&db, &["project", "create", "web", "Website"]);
    let p = run_json(
        &db,
        &[
            "project",
            "set",
            "web",
            "--icon",
            "folder",
            "--color",
            "teal",
            "--name",
            "Web",
            "--start",
            "2026-12-01",
            "--target",
            "2026-12-31",
            "--description",
            "The site",
        ],
    );
    assert_eq!(p["icon"], "icon:folder");
    assert_eq!(p["color"], "teal");
    assert_eq!(p["name"], "Web");
    assert_eq!(p["start_date"], "2026-12-01");
    assert_eq!(p["target_date"], "2026-12-31");
    assert_eq!(p["description"], "The site");
}

#[test]
fn project_set_rejects_unknown_icon_and_color_with_choices() {
    let (_d, db) = fresh();
    run_json(&db, &["project", "create", "web", "Website"]);
    let bad_icon = run(&db, &["project", "set", "web", "--icon", "nope"]);
    assert!(!bad_icon.status.success());
    let err = String::from_utf8_lossy(&bad_icon.stderr).to_string();
    assert!(err.contains("folder-open"), "stderr was: {err}");
    let bad_color = run(&db, &["project", "set", "web", "--color", "teal2"]);
    assert!(!bad_color.status.success());
    let err = String::from_utf8_lossy(&bad_color.stderr).to_string();
    assert!(err.contains("pink"), "stderr was: {err}");
    let bad_date = run(&db, &["project", "set", "web", "--start", "1 Dec"]);
    assert!(!bad_date.status.success());
}

/// Icon ids the app offers, read from the UI's own list so the CLI cannot drift from it.
fn app_icon_ids() -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui/src/ui/icons.ts");
    let text = std::fs::read_to_string(&path).expect("icons.ts");
    text.lines()
        .filter(|l| l.starts_with("  ") && !l.starts_with("  //") && l.contains(": `"))
        .map(|l| {
            l.trim()
                .split(':')
                .next()
                .unwrap_or_default()
                .trim_matches('\'')
                .to_string()
        })
        .collect()
}

#[test]
fn icons_lists_the_app_icon_ids() {
    let (_d, db) = fresh();
    let out = run(&db, &["icons"]);
    assert!(out.status.success());
    let listed: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect();
    assert_eq!(listed, app_icon_ids());
}

#[test]
fn colors_lists_the_palette_keys() {
    let (_d, db) = fresh();
    let out = run(&db, &["colors"]);
    assert!(out.status.success());
    let listed: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect();
    let expected = [
        "gray", "red", "orange", "amber", "yellow", "green", "teal", "cyan", "blue", "indigo",
        "purple", "pink",
    ];
    assert_eq!(listed, expected);
}

#[test]
fn status_update_sets_definition_kind_and_nudge() {
    let (_d, db) = fresh();
    run_json(
        &db,
        &["status", "create", "Blocked", "--category", "Started"],
    );
    let s = run_json(
        &db,
        &[
            "status",
            "update",
            "blocked",
            "--definition",
            "Needs someone else",
            "--kind",
            "waiting",
            "--nudge-days",
            "4",
        ],
    );
    assert_eq!(s["definition"], "Needs someone else");
    assert_eq!(
        (s["flow_kind"].as_str(), s["nudge_days"].as_i64()),
        (Some("Waiting"), Some(4))
    );
    let listed = run_json(&db, &["status", "list"]);
    assert!(
        listed
            .as_array()
            .expect("array")
            .iter()
            .any(|s| s["name"] == "Blocked" && s["flow_kind"] == "Waiting")
    );
    let bad = run(&db, &["status", "update", "Blocked", "--kind", "sleepy"]);
    assert_ne!(bad.status.code(), Some(0));
}
