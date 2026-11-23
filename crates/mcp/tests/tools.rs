//! Drives the real binary over stdio against a temp database.

use std::{
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Command, Stdio},
};

use serde_json::{Value, json};

const TOOL_NAMES: &[&str] = &[
    "search_issues",
    "query_view",
    "get_issue",
    "create_issue",
    "update_issue",
    "archive_issue",
    "add_comment",
    "add_relation",
    "remove_relation",
    "list_properties",
    "create_property",
    "rename_property",
    "change_property_kind",
    "delete_property",
    "set_property",
    "list_statuses",
    "create_status",
    "list_projects",
    "create_project",
    "create_milestone",
    "list_cycles",
    "create_cycle",
    "list_labels",
    "create_label",
    "list_pins",
    "pin",
    "unpin",
    "update_project",
    "bulk_update",
    "get_history",
    "list_templates",
    "create_template",
    "update_template",
    "delete_template",
    "apply_template",
    "import_csv_plan",
    "import_linear_plan",
    "import_apply",
    "undo",
    "redo",
    "undo_status",
    "workspace_status",
    "workspace_setup",
    "list_view_templates",
    "save_view_as_template",
    "create_view_from_template",
    "reset_view_to_template",
    "delete_view_template",
    "set_points",
    "insights_progress",
    "insights_velocity",
];

/// Runs one server session, sending each call only after the previous reply (the server handles
/// requests concurrently). Returns `Ok` for a successful tool result and `Err` for a tool error.
fn session(db: &Path, calls: &[(&str, Value)]) -> Vec<Result<Value, String>> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_taskapp-mcp"))
        .arg("--db")
        .arg(db)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn server");
    let mut stdin = child.stdin.take().expect("stdin");
    let mut lines = BufReader::new(child.stdout.take().expect("stdout")).lines();
    let mut send = |v: Value| writeln!(stdin, "{v}").expect("write");
    send(
        json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{
        "protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}),
    );
    lines.next().expect("init reply").expect("line");
    send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
    let mut results = Vec::new();
    for (i, (name, args)) in calls.iter().enumerate() {
        send(json!({"jsonrpc":"2.0","id":i + 1,"method":"tools/call",
            "params":{"name":name,"arguments":args}}));
        let v: Value =
            serde_json::from_str(&lines.next().expect("reply").expect("line")).expect("json");
        let text = v["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        results.push(if v["result"]["isError"] == json!(true) {
            Err(text.to_string())
        } else {
            Ok(serde_json::from_str(text).unwrap_or(Value::Null))
        });
    }
    drop(stdin);
    child.wait().expect("exit");
    results
}

fn snapshots(db: &Path) -> Vec<String> {
    let dir = db.parent().expect("parent").join("backups/snapshots");
    let Ok(rd) = std::fs::read_dir(dir) else {
        return vec![];
    };
    rd.map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect()
}

fn tmp() -> (tempfile::TempDir, std::path::PathBuf) {
    let d = tempfile::tempdir().expect("tempdir");
    let p = d.path().join("t.db");
    (d, p)
}

fn make_issues(n: usize) -> Vec<(&'static str, Value)> {
    (1..=n)
        .map(|i| ("create_issue", json!({"title": format!("t{i}")})))
        .collect()
}

#[test]
fn every_tool_listed() {
    let (_d, db) = tmp();
    let mut child = Command::new(env!("CARGO_BIN_EXE_taskapp-mcp"))
        .arg("--db")
        .arg(&db)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn");
    let msg = "{\"jsonrpc\":\"2.0\",\"id\":0,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-06-18\",\"capabilities\":{},\"clientInfo\":{\"name\":\"t\",\"version\":\"0\"}}}\n{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}\n";
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(msg.as_bytes())
        .expect("write");
    let mut out = String::new();
    child
        .stdout
        .take()
        .expect("stdout")
        .read_to_string(&mut out)
        .expect("read");
    child.wait().expect("exit");
    let listed = out
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).expect("json"))
        .find(|v| v["id"] == json!(1))
        .expect("tools reply");
    let mut names: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|t| t["name"].as_str().expect("name"))
        .collect();
    names.sort_unstable();
    let mut want = TOOL_NAMES.to_vec();
    want.sort_unstable();
    assert_eq!(names, want);
}

#[test]
fn create_issue_with_migration_timestamps() {
    let (_d, db) = tmp();
    let r = session(
        &db,
        &[
            (
                "create_property",
                json!({"key":"linear_id","name":"Linear id","kind":"Text"}),
            ),
            (
                "create_issue",
                json!({"title":"old","created_at":"2020-01-02T03:04:05Z",
            "completed_at":"2020-02-03T00:00:00Z","properties":{"linear_id":"LIN-9"},"migration":true}),
            ),
            ("get_history", json!({"key":"ABC-1"})),
        ],
    );
    let issue = r[1].as_ref().expect("create");
    assert_eq!(issue["created_at"], "2020-01-02T03:04:05Z");
    assert_eq!(issue["completed_at"], "2020-02-03T00:00:00Z");
    assert_eq!(issue["properties"]["linear_id"], "LIN-9");
    assert_eq!(r[2].as_ref().expect("history")[0]["source"], "Migration");
}

#[test]
fn bulk_update_over_10_snapshots_first() {
    let (_d, db) = tmp();
    let mut calls = make_issues(11);
    let items: Vec<Value> = (1..=11)
        .map(|i| json!({"key":format!("ABC-{i}"),"patch":{"priority":2}}))
        .collect();
    calls.push(("bulk_update", json!({"items": items})));
    let r = session(&db, &calls);
    assert_eq!(r[11].as_ref().expect("bulk")["updated"], 11);
    let s = snapshots(&db);
    assert!(s.len() == 1 && s[0].contains("bulk-update"), "{s:?}");
}

#[test]
fn bulk_update_of_10_takes_no_snapshot() {
    let (_d, db) = tmp();
    let mut calls = make_issues(10);
    let items: Vec<Value> = (1..=10)
        .map(|i| json!({"key":format!("ABC-{i}"),"patch":{"priority":2}}))
        .collect();
    calls.push(("bulk_update", json!({"items": items})));
    session(&db, &calls);
    assert_eq!(snapshots(&db), Vec::<String>::new());
}

#[test]
fn bulk_update_with_unknown_key_applies_nothing_but_snapshots() {
    let (_d, db) = tmp();
    let mut calls = make_issues(11);
    let mut items: Vec<Value> = (1..=10)
        .map(|i| json!({"key":format!("ABC-{i}"),"patch":{"priority":2}}))
        .collect();
    items.push(json!({"key":"ABC-999","patch":{"priority":2}}));
    calls.push(("bulk_update", json!({"items": items})));
    calls.push(("get_issue", json!({"key":"ABC-1"})));
    let r = session(&db, &calls);
    assert!(r[11].is_err());
    assert_eq!(r[12].as_ref().expect("get")["issue"]["priority"], 0);
    assert_eq!(snapshots(&db).len(), 1);
}

#[test]
fn delete_property_snapshots_first() {
    let (_d, db) = tmp();
    let r = session(
        &db,
        &[
            (
                "create_property",
                json!({"key":"p","name":"P","kind":"Text"}),
            ),
            ("delete_property", json!({"key":"p"})),
        ],
    );
    assert!(r[1].is_ok());
    let s = snapshots(&db);
    assert!(s.len() == 1 && s[0].contains("delete-property"), "{s:?}");
}

#[test]
fn unknown_property_snapshots_nothing_and_errors() {
    let (_d, db) = tmp();
    let r = session(
        &db,
        &[
            ("delete_property", json!({"key":"nope"})),
            (
                "change_property_kind",
                json!({"key":"nope","kind":"Number"}),
            ),
        ],
    );
    assert!(r[0].is_err() && r[1].is_err());
    assert_eq!(snapshots(&db), Vec::<String>::new());
}

#[test]
fn change_property_kind_snapshots_first() {
    let (_d, db) = tmp();
    let r = session(
        &db,
        &[
            (
                "create_property",
                json!({"key":"p","name":"P","kind":"Text"}),
            ),
            ("change_property_kind", json!({"key":"p","kind":"Url"})),
        ],
    );
    assert!(r[1].is_ok(), "{:?}", r[1]);
    assert!(
        snapshots(&db)
            .iter()
            .any(|n| n.contains("change-property-kind"))
    );
}

#[test]
fn writes_record_agent_source() {
    let (_d, db) = tmp();
    let r = session(
        &db,
        &[
            ("create_issue", json!({"title":"a"})),
            ("update_issue", json!({"key":"ABC-1","patch":{"title":"b"}})),
            ("get_history", json!({"key":"ABC-1"})),
        ],
    );
    let log = r[2]
        .as_ref()
        .expect("history")
        .as_array()
        .expect("array")
        .clone();
    assert!(log.len() >= 3);
    assert!(log.iter().all(|c| c["source"] == "Agent"), "{log:?}");
}

#[test]
fn points_and_progress_tools() {
    let (_d, db) = tmp();
    let r = session(
        &db,
        &[
            ("create_project", json!({"slug": "p", "name": "P"})),
            ("create_issue", json!({"title": "ABC a", "project_id": 1})),
            ("set_points", json!({"key": "ABC-1", "points": 5})),
            ("set_points", json!({"key": "ABC-1", "points": 0.25})),
            (
                "insights_progress",
                json!({"scope": {"Project": 1}, "measure": "points", "from": "2026-01-01", "to": "2026-01-01"}),
            ),
            (
                "insights_velocity",
                json!({"scope": {"Project": 1}, "measure": "count", "periods": 2}),
            ),
            ("set_points", json!({"key": "ABC-1", "points": null})),
        ],
    );
    assert_eq!(r[2].as_ref().expect("set")["points"], 5.0);
    assert!(
        r[3].as_ref().is_err_and(|e| e.contains("invalid")),
        "{:?}",
        r[3]
    );
    assert_eq!(
        r[4].as_ref().expect("progress")["days"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        r[5].as_ref().expect("velocity").as_array().map(Vec::len),
        Some(2)
    );
    assert_eq!(r[6].as_ref().expect("cleared")["points"], Value::Null);
}

#[test]
fn errors_are_tool_errors_not_crashes() {
    let (_d, db) = tmp();
    let r = session(
        &db,
        &[
            ("get_issue", json!({})),
            ("get_issue", json!({"key":"ABC-404"})),
            ("create_issue", json!({"title":""})),
            ("no_such_tool", json!({})),
            ("create_issue", json!({"title":"still alive"})),
        ],
    );
    assert!(
        r[0].as_ref()
            .expect_err("missing")
            .contains("missing required argument `key`")
    );
    assert!(r[1].is_err() && r[2].is_err() && r[3].is_err());
    assert!(r[4].is_ok());
}

#[test]
fn db_flag_is_used_and_home_is_untouched() {
    let (_d, db) = tmp();
    let home = tempfile::tempdir().expect("home");
    let mut child = Command::new(env!("CARGO_BIN_EXE_taskapp-mcp"))
        .env("HOME", home.path())
        .arg("--db")
        .arg(&db)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
        .expect("spawn");
    child.wait().expect("exit");
    assert!(db.exists());
    assert_eq!(std::fs::read_dir(home.path()).expect("dir").count(), 0);
}

#[test]
fn bulk_update_failing_item_leaves_earlier_items_unchanged() {
    let (_d, db) = tmp();
    let mut calls = make_issues(7);
    // Priority 9 passes argument parsing and is rejected by core on the 7th item.
    let items: Vec<Value> = (1..=7)
        .map(|i| json!({"key":format!("ABC-{i}"),"patch":{"priority": if i == 7 { 9 } else { 2 }}}))
        .collect();
    calls.push(("bulk_update", json!({"items": items})));
    calls.push(("get_issue", json!({"key":"ABC-1"})));
    let r = session(&db, &calls);
    assert!(r[7].is_err());
    assert_eq!(r[8].as_ref().expect("get")["issue"]["priority"], 0);
}

#[test]
fn failed_snapshot_aborts_delete_property() {
    let (_d, db) = tmp();
    // A file named `backups` makes the snapshot directory impossible to create.
    std::fs::write(db.parent().expect("parent").join("backups"), "not a dir").expect("block");
    let r = session(
        &db,
        &[
            (
                "create_property",
                json!({"key":"p","name":"P","kind":"Text"}),
            ),
            ("delete_property", json!({"key":"p"})),
            ("list_properties", json!({})),
        ],
    );
    assert!(r[1].is_err(), "{:?}", r[1]);
    assert!(r[2].as_ref().expect("list").to_string().contains("\"p\""));
}

#[test]
fn import_plan_writes_nothing_until_apply() {
    let (_d, db) = tmp();
    let issue = json!({"external_id":"csv:x","title":"Imported","description":"","status":null,"priority":0,
        "project":null,"labels":[],"start_date":null,"due_date":null,"estimate_minutes":null,
        "properties":{},"parent":null,"cycle":null});
    let r = session(
        &db,
        &[
            ("import_csv_plan", json!({"csv":"Title\nOne\n"})),
            ("search_issues", json!({"text":"One"})),
            (
                "import_apply",
                json!({"plan":{"source":"csv","issues":[issue]}}),
            ),
            ("search_issues", json!({"text":"Imported"})),
        ],
    );
    assert_eq!(r[0].as_ref().expect("plan")["issues"][0]["title"], "One");
    assert_eq!(r[1].as_ref().expect("search"), &json!([]));
    assert_eq!(r[2].as_ref().expect("apply")["created"], 1);
    assert_eq!(r[3].as_ref().expect("search")[0]["title"], "Imported");
}

#[test]
fn an_agent_edit_and_a_bulk_update_are_each_one_undoable_step() {
    let (_d, db) = tmp();
    let r = session(
        &db,
        &[
            ("create_issue", json!({"title":"ABC a"})),
            ("create_issue", json!({"title":"ABC b"})),
            (
                "update_issue",
                json!({"key":"ABC-1","patch":{"title":"ABC x"}}),
            ),
            (
                "bulk_update",
                json!({"items":[
                    {"key":"ABC-1","patch":{"priority":2}},
                    {"key":"ABC-2","patch":{"priority":2}}]}),
            ),
            ("undo", json!({})),
            ("get_issue", json!({"key":"ABC-2"})),
            ("undo", json!({})),
            ("get_issue", json!({"key":"ABC-1"})),
            ("undo_status", json!({})),
        ],
    );
    assert_eq!(
        r[5].as_ref().expect("get")["issue"]["priority"],
        0,
        "{:?}",
        r[5]
    );
    assert_eq!(
        r[7].as_ref().expect("get")["issue"]["title"],
        "ABC a",
        "{:?}",
        r[7]
    );
    assert_eq!(r[8].as_ref().expect("status")["redo"], "update_issue");
}

#[test]
fn workspace_setup_then_status() {
    let (_d, db) = tmp();
    let r = session(
        &db,
        &[
            ("workspace_status", json!({})),
            ("workspace_setup", json!({"name":"Cairn","key":"XY"})),
            ("workspace_status", json!({})),
        ],
    );
    assert_eq!(r[0].as_ref().expect("status")["setup_done"], false);
    assert_eq!(r[1].as_ref().expect("setup")["key"], "XY");
    assert_eq!(r[2].as_ref().expect("status")["name"], "Cairn");
}

#[test]
fn view_templates_round_trip() {
    let (_d, db) = tmp();
    let r = session(
        &db,
        &[
            (
                "save_view_as_template",
                json!({"view_id": 1, "name": "ABC t"}),
            ),
            ("create_view_from_template", json!({"template_id": 1})),
            ("reset_view_to_template", json!({"view_id": 4})),
            ("list_view_templates", json!({})),
            ("delete_view_template", json!({"id": 1})),
        ],
    );
    let t = r[0].as_ref().expect("saved");
    assert_eq!(t["name"], "ABC t");
    let v = r[1].as_ref().expect("created");
    assert_eq!(
        (v["name"].as_str(), v["template_id"].as_i64()),
        (Some("ABC t"), Some(1))
    );
    assert!(
        r[2].is_ok()
            && r[3]
                .as_ref()
                .expect("list")
                .as_array()
                .is_some_and(|a| a.len() == 1)
    );
    assert_eq!(r[4].as_ref().expect("deleted")["deleted"], true);
}

#[test]
fn unknown_argument_names_are_rejected_not_ignored() {
    let (_d, db) = tmp();
    let r = session(
        &db,
        &[
            ("create_issue", json!({"title":"x","bogus":1})),
            ("create_issue", json!({"title":"y","migration":true})),
            ("update_issue", json!({"key":"ABC-1","patch":{"prioriy":3}})),
            (
                "bulk_update",
                json!({"items":[{"key":"ABC-1","patch":{"status":"Done"}}]}),
            ),
            (
                "update_issue",
                json!({"key":"ABC-1","patch":{"priority":3},"migration":true}),
            ),
            ("add_comment", json!({"key":"ABC-1","body":"  "})),
        ],
    );
    let err = |i: usize, name: &str| {
        let e = r[i].as_ref().expect_err("must be rejected");
        assert!(e.contains(name), "{e}");
    };
    err(0, "bogus");
    r[1].as_ref().expect("migration is a known argument");
    err(2, "prioriy");
    err(3, "status");
    assert_eq!(r[4].as_ref().expect("valid patch")["priority"], 3);
    err(5, "comment body is empty");
}

#[test]
fn instructions_name_the_one_tool_that_reaches_the_network() {
    let (_d, db) = tmp();
    let mut child = Command::new(env!("CARGO_BIN_EXE_taskapp-mcp"))
        .arg("--db")
        .arg(&db)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn");
    let msg = "{\"jsonrpc\":\"2.0\",\"id\":0,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-06-18\",\"capabilities\":{},\"clientInfo\":{\"name\":\"t\",\"version\":\"0\"}}}\n";
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(msg.as_bytes())
        .expect("write");
    let mut out = String::new();
    child
        .stdout
        .take()
        .expect("stdout")
        .read_to_string(&mut out)
        .expect("read");
    child.wait().expect("exit");
    let v: Value = serde_json::from_str(out.lines().next().expect("reply")).expect("json");
    let text = v["result"]["instructions"].as_str().expect("instructions");
    assert!(!text.contains("Never touches"), "{text}");
    assert!(text.contains("import_linear_plan"), "{text}");
}

#[test]
fn unknown_argument_names_are_refused() {
    let (_d, db) = tmp();
    let r = session(
        &db,
        &[
            ("create_issue", json!({"title": "A"})),
            ("create_issue", json!({"title": "B", "bogus": 1})),
            (
                "update_issue",
                json!({"key": "ABC-1", "patch": {"bogus": 1}}),
            ),
            ("search_issues", json!({"text": "A", "bogus": 1})),
            ("list_statuses", json!({"bogus": 1})),
        ],
    );
    assert!(r[0].is_ok());
    for (i, call) in r.iter().enumerate().skip(1) {
        assert!(call.is_err(), "call {i} accepted an unknown name: {call:?}");
    }
}

#[test]
fn tool_errors_share_one_prefix_form() {
    let (_d, db) = tmp();
    let r = session(&db, &[("create_issue", json!({"title": ""}))]);
    assert_eq!(
        r[0].as_ref().expect_err("empty title"),
        "invalid: title is empty"
    );
}

#[test]
fn pin_refuses_an_issue_that_does_not_exist() {
    let (_d, db) = tmp();
    let r = session(
        &db,
        &[("pin", json!({"kind": "Issue", "target": "ABC-99"}))],
    );
    assert!(
        r[0].as_ref().is_err_and(|e| e.starts_with("not found")),
        "{:?}",
        r[0]
    );
}
