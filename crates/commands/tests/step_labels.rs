use serde_json::{Value, json};
use taskapp_commands::{AppState, dispatch};
use taskapp_core::Db;

type R = Result<(), Box<dyn std::error::Error>>;

async fn state() -> AppState {
    AppState {
        db: Db::open_in_memory().await.expect("in-memory db"),
        helper_path: "/nonexistent".into(),
        tz: "UTC".into(),
    }
}

async fn label(s: &AppState, cmd: &str, args: Value) -> Result<String, Box<dyn std::error::Error>> {
    dispatch(s, cmd, args).await?;
    let st = dispatch(s, "history_undo_status", json!({})).await?;
    Ok(st["undo"].as_str().unwrap_or_default().to_owned())
}

fn plan(n: usize) -> Value {
    let issues: Vec<Value> = (0..n)
        .map(|i| {
            json!({"external_id": format!("x{i}"), "title": format!("Row {i}"), "description": "", "status": null,
              "priority": 0, "project": null, "labels": [], "start_date": null, "due_date": null,
              "estimate_minutes": null, "properties": {}, "parent": null, "cycle": null})
        })
        .collect();
    json!({"source": "csv", "issues": issues})
}

#[tokio::test]
async fn undo_steps_carry_human_labels_with_counts() -> R {
    let s = state().await;
    let l = |c: &'static str, a: Value| label(&s, c, a);
    assert_eq!(
        l("issues_create", json!({"input": {"title": "One"}})).await?,
        "Created ABC-1"
    );
    l("issues_create", json!({"input": {"title": "Two"}})).await?;
    assert_eq!(
        l(
            "issues_bulk_update",
            json!({"ids": [1, 2], "patch": {"patch": {"status_id": 3}}})
        )
        .await?,
        "Set status on 2 issues"
    );
    assert_eq!(
        l(
            "issues_bulk_update",
            json!({"ids": [1], "patch": {"archive": true}})
        )
        .await?,
        "Archived 1 issue"
    );
    assert_eq!(
        l("issues_update", json!({"id": 2, "patch": {"priority": 2}})).await?,
        "Edited ABC-2"
    );
    assert_eq!(
        l("issues_duplicate", json!({"ids": [1, 2]})).await?,
        "Duplicated 2 issues"
    );
    assert_eq!(
        l("import_apply", json!({"plan": plan(3)})).await?,
        "Imported 3 issues"
    );
    Ok(())
}

#[tokio::test]
async fn view_and_generic_labels() -> R {
    let s = state().await;
    let l = |c: &'static str, a: Value| label(&s, c, a);
    let def = json!({"filter": null, "sorts": [], "group_by": null, "layout": "List", "columns": [], "include_archived": false});
    assert_eq!(
        l(
            "views_save",
            json!({"id": null, "name": "Mine", "def": def, "icon": null, "color": null})
        )
        .await?,
        "Saved view Mine"
    );
    assert_eq!(
        l(
            "views_save",
            json!({"id": 1, "name": "Active", "def": def, "icon": null, "color": null})
        )
        .await?,
        "Changed display of Active"
    );
    assert_eq!(
        l(
            "labels_create",
            json!({"name": "Bug", "color": "red", "icon": null})
        )
        .await?,
        "Create label"
    );
    Ok(())
}

#[tokio::test]
async fn quick_add_is_labelled_like_a_plain_create() -> R {
    let s = state().await;
    assert_eq!(
        label(
            &s,
            "issues_create_from_quickadd",
            json!({"input": "ship it p2"})
        )
        .await?,
        "Created ABC-1"
    );
    Ok(())
}
