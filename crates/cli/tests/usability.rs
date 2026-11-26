use std::path::Path;

use assert_cmd::Command;
use serde_json::Value;

fn run(db: &Path, args: &[&str]) -> Value {
    let output = Command::cargo_bin("taskapp")
        .expect("binary")
        .args(["--db", db.to_str().expect("path"), "--json"])
        .args(args)
        .output()
        .expect("run CLI");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("JSON")
}

#[test]
fn issues_accept_names_for_status_project_cycle_and_labels() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("test.db");
    let status = run(
        &db,
        &["status", "create", "Needs review", "--category", "Started"],
    );
    let project = run(&db, &["project", "create", "test-project", "Test project"]);
    let label = run(&db, &["label", "create", "Review", "red"]);
    let cycle = run(
        &db,
        &[
            "cycle",
            "create",
            "--start",
            "2026-01-01",
            "--end",
            "2026-01-14",
            "--name",
            "Winter",
        ],
    );
    let issue = run(
        &db,
        &[
            "issue",
            "create",
            "Named references",
            "--status",
            "needs review",
            "--project",
            "Test project",
            "--cycle",
            "winter",
            "--label",
            "review",
        ],
    );
    assert_eq!(issue["status_id"], status["id"]);
    assert_eq!(issue["project_id"], project["id"]);
    assert_eq!(issue["cycle_id"], cycle["id"]);
    assert_eq!(issue["label_ids"], serde_json::json!([label["id"]]));
    let updated = run(
        &db,
        &[
            "issue",
            "update",
            issue["key"].as_str().expect("key"),
            "--status",
            "Needs review",
            "--project",
            "test-project",
        ],
    );
    assert_eq!(updated["project_id"], project["id"]);
}

#[test]
fn guide_is_available_without_opening_a_database() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("missing/never-created.db");
    let guide = run(&db, &["guide"]);
    assert!(guide.to_string().contains("api"));
    assert!(!db.exists());
}

#[test]
fn ambiguous_project_name_requires_an_id() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("test.db");
    let first = run(&db, &["project", "create", "shared", "First"]);
    run(&db, &["project", "create", "other", "Shared"]);
    let output = Command::cargo_bin("taskapp")
        .expect("binary")
        .args([
            "--db",
            db.to_str().expect("path"),
            "--json",
            "issue",
            "create",
            "Test",
            "--project",
            "shared",
        ])
        .output()
        .expect("run CLI");
    assert!(!output.status.success());
    let error: Value = serde_json::from_slice(&output.stderr).expect("JSON error");
    assert_eq!(error["kind"], "Conflict");
    let issue = run(
        &db,
        &[
            "issue",
            "create",
            "Test",
            "--project",
            &first["id"].to_string(),
        ],
    );
    assert_eq!(issue["project_id"], first["id"]);
}
