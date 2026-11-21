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
    let snapshots = dir.path().join("snapshots");
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
