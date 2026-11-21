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
    assert_eq!(out, json!("ZF"));
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
        json!({"reminders": "Off", "gcal": "Off", "export": "Off"})
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
