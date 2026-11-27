use std::path::Path;
use std::process::Output;

use assert_cmd::Command;
use serde_json::{Value, json};

fn run(db: &Path, args: &[&str]) -> Output {
    Command::cargo_bin("taskapp")
        .expect("binary")
        .arg("--db")
        .arg(db)
        .arg("--json")
        .args(args)
        .output()
        .expect("run CLI")
}

fn call(db: &Path, name: &str, args: &Value) -> Value {
    let output = run(db, &["api", "call", name, "--args", &args.to_string()]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("JSON output")
}

#[test]
fn api_is_discoverable_and_edits_app_entities_with_cli_history() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("test.db");
    let listed = run(&db, &["api", "list"]);
    assert!(
        listed.status.success(),
        "{}",
        String::from_utf8_lossy(&listed.stderr)
    );
    let names: Value = serde_json::from_slice(&listed.stdout).expect("JSON");
    assert!(
        names
            .as_array()
            .expect("array")
            .iter()
            .any(|name| name == "labels_update")
    );
    let describe = run(&db, &["api", "describe", "labels_update"]);
    assert!(
        describe.status.success(),
        "{}",
        String::from_utf8_lossy(&describe.stderr)
    );
    assert!(String::from_utf8_lossy(&describe.stdout).contains("label"));

    let mut label = call(
        &db,
        "labels_create",
        &json!({"name":"Urgent","color":"red","icon":"icon:tag"}),
    );
    label["icon"] = json!("icon:star");
    label["name"] = json!("Renamed");
    let updated = call(&db, "labels_update", &json!({"label": label}));
    assert_eq!(updated["icon"], "icon:star");
    assert_eq!(updated["name"], "Renamed");
    let history = call(
        &db,
        "history_list_for",
        &json!({"entity":"label","entity_id": updated["id"].to_string()}),
    );
    assert!(
        history
            .as_array()
            .expect("history")
            .iter()
            .all(|row| row["source"] == "Cli")
    );
    let status = call(&db, "history_undo_status", &json!({}));
    assert_eq!(status["undo"], "api call labels_update");
}

#[test]
fn api_refuses_unknown_commands_and_invalid_arguments_as_json() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("test.db");
    for (name, args, kind) in [
        ("not_a_command", "{}", "NotFound"),
        ("labels_create", "{}", "Invalid"),
        ("labels_list", "[]", "Invalid"),
    ] {
        let out = run(&db, &["api", "call", name, "--args", args]);
        assert!(!out.status.success());
        let err: Value = serde_json::from_slice(&out.stderr).expect("JSON error");
        assert_eq!(err["kind"], kind);
    }
}

#[test]
fn generic_destructive_property_operations_still_require_a_snapshot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("test.db");
    let describe = run(&db, &["api", "describe", "properties_delete_def"]);
    assert!(
        describe.status.success(),
        "{}",
        String::from_utf8_lossy(&describe.stderr)
    );
    let created = run(&db, &["property", "create", "note", "Note", "text"]);
    assert!(created.status.success());
    std::fs::write(dir.path().join("backups"), "not a directory").expect("block snapshots");
    let out = run(
        &db,
        &[
            "api",
            "call",
            "properties_delete_def",
            "--args",
            r#"{"key":"note"}"#,
        ],
    );
    assert!(!out.status.success());
    let properties = run(&db, &["property", "list"]);
    let list: Value = serde_json::from_slice(&properties.stdout).expect("JSON");
    assert!(
        list.as_array()
            .expect("array")
            .iter()
            .any(|p| p["key"] == "note")
    );
}

#[test]
fn large_bulk_api_updates_require_a_snapshot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("test.db");
    let issue = call(
        &db,
        "issues_create",
        &json!({"input": {"title": "Keep me"}}),
    );
    std::fs::write(dir.path().join("backups"), "not a directory").expect("block snapshots");
    let args = json!({"ids": vec![issue["id"].clone(); 11], "patch": {"priority": 1}});
    let out = run(
        &db,
        &[
            "api",
            "call",
            "issues_bulk_update",
            "--args",
            &args.to_string(),
        ],
    );
    assert!(!out.status.success());
    let after = call(&db, "issues_get", &json!({"id": issue["id"]}));
    assert_eq!(after["priority"], issue["priority"]);
}

#[test]
fn api_manages_views_and_project_icons_with_undo() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("test.db");
    let mut project = call(
        &db,
        "projects_create",
        &json!({"slug":"example","name":"Example"}),
    );
    project["icon"] = json!("icon:star");
    let updated = call(&db, "projects_update", &json!({"project": project}));
    assert_eq!(updated["icon"], "icon:star");
    call(&db, "history_undo", &json!({}));
    let projects = call(&db, "projects_list", &json!({}));
    assert!(projects[0]["icon"].is_null());
    let view = call(
        &db,
        "views_save",
        &json!({
            "name":"My view", "def":taskapp_core::ViewDef::default(), "icon":"icon:star"
        }),
    );
    assert_eq!(view["icon"], "icon:star");
    let views = call(&db, "views_list", &json!({}));
    assert!(
        views
            .as_array()
            .expect("views")
            .iter()
            .any(|v| v["id"] == view["id"])
    );
}
