use serde_json::json;
use taskapp_commands::{AppState, dispatch, names};
use taskapp_core::{Db, Error};

type R = Result<(), Box<dyn std::error::Error>>;

async fn state() -> AppState {
    AppState {
        db: Db::open_in_memory().await.expect("in-memory db"),
        helper_path: "/nonexistent".into(),
        tz: "UTC".into(),
    }
}

#[tokio::test]
async fn dispatch_unknown_command_is_not_found() {
    let err = dispatch(&state().await, "nope", json!({}))
        .await
        .expect_err("should fail");
    assert!(matches!(err, Error::NotFound(_)), "got {err:?}");
}

#[tokio::test]
async fn dispatch_routes_by_name_with_named_args() -> R {
    let out = dispatch(
        &state().await,
        "settings_get",
        json!({"key": "issue_prefix"}),
    )
    .await?;
    assert_eq!(out, json!("ABC"));
    Ok(())
}
#[tokio::test]
async fn dispatch_bad_args_is_invalid() {
    let err = dispatch(&state().await, "settings_get", json!({}))
        .await
        .expect_err("should fail");
    assert!(matches!(err, Error::Invalid(_)), "got {err:?}");
}

#[tokio::test]
async fn sync_status_defaults_to_off() -> R {
    let out = dispatch(&state().await, "sync_status", json!({})).await?;
    assert_eq!(
        out,
        json!({"reminders": "Off", "gcal": "Off", "export": "Off", "backup": "Off"})
    );
    Ok(())
}
#[test]
fn command_names_are_unique_and_exclude_sync_crate_commands() {
    let all: Vec<_> = names().collect();
    let mut sorted = all.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(all.len(), sorted.len());
    for banned in [
        "gcal_connect",
        "gcal_list_calendars",
        "reminders_list_lists",
        "sync_now",
    ] {
        assert!(
            !all.contains(&banned),
            "{banned} must live in the app crate"
        );
    }
}

#[test]
fn backup_restore_is_not_a_command() {
    // Restoring over the open database cannot work from the app. Settings points to the CLI instead.
    assert!(!names().any(|n| n == "backup_restore"));
}

#[tokio::test]
async fn backup_snapshot_then_list_finds_it() -> R {
    let dir = std::env::temp_dir().join(format!("taskapp-snapshot-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let st = AppState {
        db: Db::open(&dir.join("live.db")).await?,
        helper_path: "/nonexistent".into(),
        tz: "UTC".into(),
    };
    let snap = dispatch(&st, "backup_snapshot", json!({"dir": dir, "reason": "t"})).await?;
    let listed = dispatch(&st, "backup_list", json!({"dir": dir})).await?;
    assert_eq!(listed, json!([snap]));
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}

#[tokio::test]
async fn an_ui_edit_is_one_undoable_step() -> R {
    let st = state().await;
    let made = dispatch(&st, "issues_create", json!({"input": {"title": "ABC one"}})).await?;
    let id = made["id"].clone();
    dispatch(
        &st,
        "issues_update",
        json!({"id": id, "patch": {"title": "ABC two"}}),
    )
    .await?;
    let status = dispatch(&st, "history_undo_status", json!({})).await?;
    assert_eq!(status["undo"], "Edited ABC-1");
    dispatch(&st, "history_undo", json!({})).await?;
    let back = dispatch(&st, "issues_get", json!({"id": id})).await?;
    assert_eq!(back["title"], "ABC one");
    dispatch(&st, "history_redo", json!({})).await?;
    let again = dispatch(&st, "issues_get", json!({"id": id})).await?;
    assert_eq!(again["title"], "ABC two");
    Ok(())
}

#[tokio::test]
async fn a_bulk_edit_through_dispatch_is_one_undoable_step() -> R {
    let st = state().await;
    let mut ids = Vec::new();
    for t in ["ABC a", "ABC b", "ABC c"] {
        ids.push(
            dispatch(&st, "issues_create", json!({"input": {"title": t}})).await?["id"].clone(),
        );
    }
    dispatch(&st, "history_undo", json!({})).await?; // drop the last create so only the bulk step is under test
    ids.pop();
    let list = json!(ids);
    dispatch(
        &st,
        "issues_bulk_update",
        json!({"ids": list, "patch": {"patch": {"priority": 2}}}),
    )
    .await?;
    assert_eq!(
        dispatch(&st, "history_undo_status", json!({})).await?["undo"],
        "Set priority on 2 issues"
    );
    dispatch(&st, "history_undo", json!({})).await?;
    for id in &ids {
        assert_eq!(
            dispatch(&st, "issues_get", json!({"id": id})).await?["priority"],
            0
        );
    }
    Ok(())
}

#[tokio::test]
async fn reorder_statuses_and_pin_many_are_one_undo_step_each() -> R {
    let s = state().await;
    let mut ids = vec![];
    for name in ["ABC one", "ABC two"] {
        let made = dispatch(
            &s,
            "statuses_create",
            json!({"name": name, "category": "Started", "color": "blue", "icon": null}),
        )
        .await?;
        ids.push(made["id"].clone());
    }
    dispatch(
        &s,
        "statuses_reorder",
        json!({"ids_in_order": [ids[1], ids[0]]}),
    )
    .await?;
    let status = dispatch(&s, "history_undo_status", json!({})).await?;
    assert_eq!(status["undo"], "Reorder status");
    dispatch(&s, "history_undo", json!({})).await?;
    let list = dispatch(&s, "statuses_list", json!({})).await?;
    let started: Vec<_> = list
        .as_array()
        .expect("list")
        .iter()
        .filter(|x| x["category"] == "Started")
        .map(|x| x["id"].clone())
        .collect();
    assert_eq!(started[started.len() - 2..], ids[..]);

    let urls = json!(["https://a.example", "https://b.example"]);
    dispatch(
        &s,
        "pins_set",
        json!({"kind": "Url", "targets": urls, "pinned": true}),
    )
    .await?;
    assert_eq!(
        dispatch(&s, "pins_list_resolved", json!({}))
            .await?
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    let status = dispatch(&s, "history_undo_status", json!({})).await?;
    assert_eq!(status["undo"], "Pinned 2 items");
    dispatch(&s, "history_undo", json!({})).await?;
    assert_eq!(
        dispatch(&s, "pins_list_resolved", json!({}))
            .await?
            .as_array()
            .map(Vec::len),
        Some(0)
    );
    Ok(())
}

#[tokio::test]
async fn insights_commands_take_the_wire_shapes() -> R {
    let s = state().await;
    let p = dispatch(
        &s,
        "projects_create",
        json!({"slug": "alpha", "name": "Alpha", "icon": null, "color": null}),
    )
    .await?;
    let scope = json!({"Project": p["id"]});
    let args =
        json!({"scope": scope, "measure": "points", "from": "2026-01-01", "to": "2026-01-02"});
    let out = dispatch(&s, "insights_progress", args).await?;
    assert_eq!(out["measure"], "points");
    assert_eq!(
        out["days"][0],
        json!({"date": "2026-01-01", "scope": 0.0, "started": 0.0, "completed": 0.0})
    );
    let v = dispatch(
        &s,
        "insights_velocity",
        json!({"scope": scope, "measure": "count", "periods": 2}),
    )
    .await?;
    assert_eq!(v.as_array().map(Vec::len), Some(2));
    let issue = dispatch(
        &s,
        "issues_create",
        json!({"input": {"title": "ABC a", "points": 3.0}}),
    )
    .await?;
    assert_eq!(issue["points"], 3.0);
    let issue = dispatch(
        &s,
        "issues_update",
        json!({"id": issue["id"], "patch": {"points": null}}),
    )
    .await?;
    assert_eq!(issue["points"], json!(null));
    Ok(())
}

#[tokio::test]
async fn attachments_round_trip_and_leave_no_undo_step() -> R {
    let st = state().await;
    let png: Vec<u8> = b"\x89PNG\r\n\x1a\nxx".to_vec();
    let a = dispatch(
        &st,
        "attachments_add",
        json!({"bytes": png, "mime": "image/png"}),
    )
    .await?;
    let got = dispatch(&st, "attachments_get", json!({"id": a["id"]})).await?;
    assert_eq!(got["bytes"], json!(png));
    let undo = dispatch(&st, "history_undo_status", json!({})).await?;
    assert_eq!(undo["undo"], json!(null), "{undo}");
    Ok(())
}

#[tokio::test]
async fn metrics_for_takes_the_wire_shape() -> R {
    let s = state().await;
    let query = json!({"scope": {"Range": {"from": "2026-01-01", "to": "2026-01-02"}},
        "bucket": "Day", "filter": null, "measure": "count"});
    let out = dispatch(&s, "metrics_for", json!({ "query": query })).await?;
    assert_eq!(out["buckets"].as_array().map(Vec::len), Some(2));
    assert_eq!(
        out["lead_time"],
        json!({"p50_days": null, "p85_days": null, "n": 0})
    );
    assert_eq!(out["thin"], true);
    Ok(())
}

#[tokio::test]
async fn forecast_for_takes_the_wire_shape() -> R {
    let s = state().await;
    let out = dispatch(
        &s,
        "forecast_for",
        json!({ "query": {"filter": null, "cycle": null} }),
    )
    .await?;
    assert_eq!(out["enough"], false);
    assert_eq!(out["p50"], json!(null));
    Ok(())
}
