use chrono::NaiveDate;
use taskapp_core::palette::Look;
use taskapp_core::{
    ChangeSource::Ui, Db, Layout, StatusCategory, ViewDef, cycles, history, labels, palette,
    projects, statuses, views,
};

fn day(d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 1, d).expect("valid date")
}

fn plain_view() -> ViewDef {
    ViewDef {
        filter: None,
        sorts: vec![],
        group_by: None,
        layout: Layout::List,
        columns: vec![],
        include_archived: false,
    }
}

#[tokio::test]
async fn project_keeps_icon_and_color() -> Result<(), Box<dyn std::error::Error>> {
    let db = Db::open_in_memory().await?;
    let p = projects::create(
        &db,
        "abc-one",
        "ABC one",
        Some("icon:folder".into()),
        Some("teal".into()),
        Ui,
    )
    .await?;
    assert_eq!(p.icon.as_deref(), Some("icon:folder"));
    let mut edited = p.clone();
    edited.color = Some("pink".into());
    edited.icon = Some("emoji:🚀".into());
    projects::update(&db, edited, Ui).await?;
    let got = projects::list(&db).await?.remove(0);
    assert_eq!(got.color.as_deref(), Some("pink"));
    assert_eq!(got.icon.as_deref(), Some("emoji:🚀"));
    let changed: Vec<_> = history::list_for(&db, "project", &p.id.0.to_string())
        .await?
        .into_iter()
        .map(|c| c.field)
        .collect();
    assert!(changed.contains(&"color".to_owned()) && changed.contains(&"icon".to_owned()));
    Ok(())
}

#[tokio::test]
async fn cycle_keeps_icon_and_color() -> Result<(), Box<dyn std::error::Error>> {
    let db = Db::open_in_memory().await?;
    let c = cycles::create(
        &db,
        None,
        day(1),
        day(14),
        Look {
            icon: Some("icon:sprint".into()),
            color: Some("amber".into()),
        },
        Ui,
    )
    .await?;
    let got = cycles::list(&db).await?.remove(0);
    assert_eq!(
        (got.icon.as_deref(), got.color.as_deref()),
        (Some("icon:sprint"), Some("amber"))
    );
    assert_eq!(c.id, got.id);
    Ok(())
}

#[tokio::test]
async fn view_keeps_icon_and_color_when_not_given() -> Result<(), Box<dyn std::error::Error>> {
    let db = Db::open_in_memory().await?;
    let v = views::save(
        &db,
        None,
        "ABC view",
        plain_view(),
        Look {
            icon: Some("icon:star".into()),
            color: Some("cyan".into()),
        },
        Ui,
    )
    .await?;
    let renamed = views::save(
        &db,
        Some(v.id),
        "ABC renamed",
        plain_view(),
        Look::default(),
        Ui,
    )
    .await?;
    assert_eq!(renamed.icon.as_deref(), Some("icon:star"));
    assert_eq!(renamed.color.as_deref(), Some("cyan"));
    let row = views::list(&db)
        .await?
        .into_iter()
        .find(|v| v.name == "ABC renamed");
    assert_eq!(row.and_then(|v| v.color), Some("cyan".to_owned()));
    Ok(())
}

#[tokio::test]
async fn label_and_status_store_key_and_resolve_hex() -> Result<(), Box<dyn std::error::Error>> {
    let db = Db::open_in_memory().await?;
    let l = labels::create(&db, "ABC bug", "red", Some("icon:bug".into()), Ui).await?;
    assert_eq!((l.color.as_str(), l.color_hex.as_str()), ("red", "#ef4444"));
    let s = statuses::create(
        &db,
        "ABC review",
        StatusCategory::Started,
        "indigo",
        None,
        Ui,
    )
    .await?;
    assert_eq!(s.color, "indigo");
    let s = statuses::update(
        &db,
        s.id,
        None,
        None,
        Look {
            icon: Some("icon:eye".into()),
            color: None,
        },
        Ui,
    )
    .await?;
    assert_eq!(
        (s.color.as_str(), s.icon.as_deref()),
        ("indigo", Some("icon:eye"))
    );
    Ok(())
}

#[tokio::test]
async fn status_update_records_color_and_icon() -> Result<(), Box<dyn std::error::Error>> {
    let db = Db::open_in_memory().await?;
    let s = statuses::create(&db, "ABC step", StatusCategory::Started, "gray", None, Ui).await?;
    statuses::update(
        &db,
        s.id,
        None,
        None,
        Look {
            icon: Some("icon:flag".into()),
            color: Some("blue".into()),
        },
        Ui,
    )
    .await?;
    let fields: Vec<_> = history::list_for(&db, "status", &s.id.0.to_string())
        .await?
        .into_iter()
        .map(|c| c.field)
        .collect();
    assert!(fields.contains(&"color".to_owned()) && fields.contains(&"icon".to_owned()));
    Ok(())
}

#[tokio::test]
async fn unknown_color_key_and_bad_icon_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let db = Db::open_in_memory().await?;
    assert!(
        projects::create(&db, "abc-two", "ABC two", None, Some("teal2".into()), Ui)
            .await
            .is_err()
    );
    assert!(
        projects::create(&db, "abc-two", "ABC two", Some("Folder".into()), None, Ui)
            .await
            .is_err()
    );
    assert!(
        labels::create(&db, "ABC x", "gray", Some("emoji:".into()), Ui)
            .await
            .is_err()
    );
    assert!(palette::check_icon("icon:Bad_Id").is_err());
    Ok(())
}

#[tokio::test]
async fn legacy_hex_colors_map_to_nearest_key() -> Result<(), Box<dyn std::error::Error>> {
    let db = Db::open_in_memory().await?;
    sqlx::query("INSERT INTO labels (name, color) VALUES ('ABC legacy', '#ff0000')")
        .execute(&db.pool)
        .await?;
    palette::map_legacy_colors(&db.pool).await?;
    let legacy = labels::list(&db)
        .await?
        .into_iter()
        .find(|l| l.name == "ABC legacy");
    assert_eq!(legacy.map(|l| l.color), Some("red".to_owned()));
    let done = statuses::list(&db)
        .await?
        .into_iter()
        .find(|s| s.name == "Done");
    assert_eq!(done.map(|s| s.color), Some("indigo".to_owned()));
    Ok(())
}
