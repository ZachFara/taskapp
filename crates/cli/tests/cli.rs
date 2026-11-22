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
    assert!(stdout.contains("ZF-1"), "stdout was: {stdout}");
}

#[test]
fn json_output_parses() {
    let (_d, db) = fresh();
    run(&db, &["add", "Write report"]);
    let issue = run_json(&db, &["issue", "get", "ZF-1"]);
    assert_eq!(issue["key"], "ZF-1");
}

#[test]
fn issue_update_by_key() {
    let (_d, db) = fresh();
    run(&db, &["add", "Write report"]);
    let out = run(&db, &["issue", "update", "ZF-1", "--priority", "2"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let issue = run_json(&db, &["issue", "get", "ZF-1"]);
    assert_eq!(issue["priority"], 2);
}

#[test]
fn undo_and_redo_revert_a_cli_edit() {
    let (_d, db) = fresh();
    run(&db, &["add", "Write report"]);
    run(&db, &["issue", "update", "ZF-1", "--priority", "2"]);
    let s = run_json(&db, &["undo"]);
    assert_eq!(s["redo"], "issue update ZF-1 --priority 2");
    assert_eq!(run_json(&db, &["issue", "get", "ZF-1"])["priority"], 0);
    run_json(&db, &["redo"]);
    assert_eq!(run_json(&db, &["issue", "get", "ZF-1"])["priority"], 2);
}

#[test]
fn unknown_key_exits_nonzero_with_message() {
    let (_d, db) = fresh();
    let out = run(&db, &["issue", "get", "ZF-999"]);
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
    let out = run(&db, &["--json", "issue", "get", "ZF-999"]);
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
        run(&dst_db, &["issue", "get", "ZF-3"]).status.code(),
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
    let issue = run_json(&dst_db, &["issue", "get", "ZF-3"]);
    assert_eq!(issue["key"], "ZF-3");
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
    let sources = history_sources(&db, "ZF-1");
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
            "ZF-1",
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
        history_sources(&db, "ZF-2")
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
            "ZF-1",
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
    assert!(!run(&db, &["issue", "get", "ZF-1"]).status.success());
    let done = run_json(&db, &["import", "csv", path, "--apply"]);
    assert_eq!(done["applied"]["created"], 2);
    assert_eq!(run_json(&db, &["issue", "get", "ZF-2"])["title"], "Two");
}
