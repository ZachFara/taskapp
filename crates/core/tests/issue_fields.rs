use chrono::NaiveDate;
use serde_json::json;
use taskapp_core::{Db, Error, quickadd, settings};

type R = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn issue_fields_refuses_unknown_repeated_and_malformed() -> R {
    let db = Db::open_in_memory().await?;
    for bad in [
        json!("priority"),
        json!([{ "field": "status", "on": false }]),
        json!([{ "field": "due", "on": true }, { "field": "due", "on": false }]),
        json!([{ "field": "due" }]),
    ] {
        let e = settings::set_from_user(&db, "issue_fields", &bad).await;
        assert!(matches!(e, Err(Error::Invalid(_))), "{bad}: {e:?}");
    }
    let ok = json!([{ "field": "due", "on": false }, { "field": "priority", "on": true }]);
    settings::set_from_user(&db, "issue_fields", &ok).await?;
    assert_eq!(
        settings::get::<serde_json::Value>(&db, "issue_fields").await?,
        Some(ok)
    );
    Ok(())
}

#[tokio::test]
async fn a_field_that_is_off_leaves_its_quick_add_token_in_the_title() -> R {
    let db = Db::open_in_memory().await?;
    let today = NaiveDate::from_ymd_opt(2026, 1, 5).ok_or("date")?;
    let on = quickadd::parse_for(&db, "fix it p1 ~1h", today, "UTC").await?;
    assert_eq!((on.priority, on.estimate_minutes), (Some(1), Some(60)));
    let off = json!([{ "field": "priority", "on": false }, { "field": "estimate", "on": false }]);
    settings::set_from_user(&db, "issue_fields", &off).await?;
    let p = quickadd::parse_for(&db, "fix it p1 ~1h", today, "UTC").await?;
    assert_eq!((p.priority, p.estimate_minutes), (None, None));
    assert_eq!(p.title, "fix it p1 ~1h");
    Ok(())
}
