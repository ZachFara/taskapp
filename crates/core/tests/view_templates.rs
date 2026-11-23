use serde_json::json;
use taskapp_core::{
    ChangeSource, Db, Density, FieldRef, FixedField, Layout, NewIssue, PropertyDef, PropertyKind,
    SubIssues, ViewDef, history, issues, palette::Look, properties, settings, undo, view_templates,
    views, workspace,
};

type R = Result<(), Box<dyn std::error::Error>>;
const UI: ChangeSource = ChangeSource::Ui;

fn rich() -> ViewDef {
    ViewDef {
        layout: Layout::Board,
        columns: vec![
            FieldRef::Fixed(FixedField::Priority),
            FieldRef::Property("area".into()),
            FieldRef::Fixed(FixedField::CommentCount),
        ],
        density: Density::Compact,
        show_empty_groups: true,
        show_sub_issues: SubIssues::Flat,
        card_fields: vec![FieldRef::Property("area".into())],
        show_weekends: false,
        ..ViewDef::default()
    }
}

#[test]
fn old_stored_json_gets_display_defaults() -> R {
    let old = r#"{"filter":null,"sorts":[],"group_by":null,"layout":"List","columns":[],"include_archived":false}"#;
    let d: ViewDef = serde_json::from_str(old)?;
    assert_eq!(d.density, Density::Comfortable);
    assert_eq!(d.show_sub_issues, SubIssues::Nest);
    assert!(d.show_weekends && !d.show_empty_groups && d.card_fields.is_empty());
    Ok(())
}

#[tokio::test]
async fn display_settings_round_trip_and_query_returns_property_values() -> R {
    let db = Db::open_in_memory().await?;
    properties::create_def(
        &db,
        PropertyDef {
            key: "area".into(),
            name: "Area".into(),
            kind: PropertyKind::Text,
            options: vec![],
            project_id: None,
            position: 0,
            applies_to: taskapp_core::PropertyAppliesTo::Issues,
        },
        UI,
    )
    .await?;
    let i = issues::create(
        &db,
        NewIssue {
            title: "ABC one".into(),
            properties: serde_json::from_value(json!({"area": "web"}))?,
            ..NewIssue::default()
        },
        UI,
    )
    .await?;
    let v = views::save(&db, None, "Mine", rich(), Look::default(), UI).await?;
    assert_eq!(
        views::list(&db).await?.last().map(|x| &x.def),
        Some(&rich())
    );
    assert_eq!(v.template_id, None);
    let got = views::query(&db, &v.def).await?;
    assert_eq!(got[0].properties["area"], "web");
    let stats = views::stats(&db, &v.def).await?;
    assert_eq!(stats[0].issue_id, i.id);
    assert_eq!(
        (stats[0].sub_total, stats[0].comments, stats[0].relations),
        (0, 0, 0)
    );
    Ok(())
}

#[tokio::test]
async fn sorting_by_comment_count() -> R {
    let db = Db::open_in_memory().await?;
    let a = issues::create(
        &db,
        NewIssue {
            title: "ABC a".into(),
            ..NewIssue::default()
        },
        UI,
    )
    .await?;
    let b = issues::create(
        &db,
        NewIssue {
            title: "ABC b".into(),
            ..NewIssue::default()
        },
        UI,
    )
    .await?;
    taskapp_core::comments::add_comment(&db, b.id, "hi", None, UI).await?;
    let d = ViewDef {
        sorts: vec![taskapp_core::Sort {
            field: FieldRef::Fixed(FixedField::CommentCount),
            descending: true,
        }],
        ..ViewDef::default()
    };
    let ids: Vec<_> = views::query(&db, &d).await?.iter().map(|i| i.id).collect();
    assert_eq!(ids, vec![b.id, a.id]);
    let s = views::stats(&db, &d).await?;
    assert_eq!(s[0].comments, 1);
    Ok(())
}

#[tokio::test]
async fn first_run_seeds_three_templates_once() -> R {
    let db = Db::open_in_memory().await?;
    assert_eq!(view_templates::list(&db).await?.len(), 0);
    workspace::setup(&db, "Cairn", "ABC", Some("UTC")).await?;
    let names: Vec<_> = view_templates::list(&db)
        .await?
        .into_iter()
        .map(|t| t.name)
        .collect();
    assert_eq!(names, ["Status board", "Planning table", "This week"]);
    workspace::setup(&db, "Cairn", "ABD", Some("UTC")).await?;
    assert_eq!(view_templates::list(&db).await?.len(), 3);
    Ok(())
}

#[tokio::test]
async fn template_crud_with_undo() -> R {
    let db = Db::open_in_memory().await?;
    let t = history::step(
        "Create template",
        view_templates::create(&db, "ABC t", rich(), Some("icon:star".into()), None, UI),
    )
    .await?;
    assert_eq!(t.def, rich());
    let mut edited = t.clone();
    edited.name = "ABC u".into();
    history::step("Update template", view_templates::update(&db, edited, UI)).await?;
    assert_eq!(view_templates::list(&db).await?[0].name, "ABC u");
    undo::undo(&db).await?;
    assert_eq!(view_templates::list(&db).await?[0].name, "ABC t");
    undo::redo(&db).await?;
    history::step("Delete template", view_templates::delete(&db, t.id, UI)).await?;
    assert_eq!(view_templates::list(&db).await?.len(), 0);
    undo::undo(&db).await?;
    assert_eq!(view_templates::list(&db).await?.len(), 1);
    undo::undo(&db).await?;
    undo::undo(&db).await?;
    assert_eq!(view_templates::list(&db).await?.len(), 0);
    assert!(matches!(
        view_templates::delete(&db, t.id, UI).await,
        Err(taskapp_core::Error::NotFound(_))
    ));
    Ok(())
}

#[tokio::test]
async fn create_from_template_copies_def_and_reset_restores_it() -> R {
    let db = Db::open_in_memory().await?;
    let t = view_templates::create(&db, "ABC t", rich(), None, None, UI).await?;
    let v = history::step("New view", view_templates::create_view(&db, t.id, None, UI)).await?;
    assert_eq!(
        (v.name.as_str(), &v.def, v.template_id),
        ("ABC t", &rich(), Some(t.id))
    );
    let named = view_templates::create_view(&db, t.id, Some("Mine"), UI).await?;
    assert_eq!(named.name, "Mine");
    views::save(
        &db,
        Some(v.id),
        "ABC t",
        ViewDef::default(),
        Look::default(),
        UI,
    )
    .await?;
    assert_eq!(
        views::list(&db)
            .await?
            .iter()
            .find(|x| x.id == v.id)
            .map(|x| x.template_id),
        Some(Some(t.id))
    );
    let r = history::step("Reset view", view_templates::reset_view(&db, v.id, UI)).await?;
    assert_eq!(r.def, rich());
    undo::undo(&db).await?;
    assert_eq!(
        views::list(&db)
            .await?
            .iter()
            .find(|x| x.id == v.id)
            .map(|x| x.def.clone()),
        Some(ViewDef::default())
    );
    // a view with no template cannot be reset
    let plain = views::save(&db, None, "ABC p", ViewDef::default(), Look::default(), UI).await?;
    assert!(matches!(
        view_templates::reset_view(&db, plain.id, UI).await,
        Err(taskapp_core::Error::Invalid(_))
    ));
    Ok(())
}

#[tokio::test]
async fn save_as_template_and_undo() -> R {
    let db = Db::open_in_memory().await?;
    let v = views::save(
        &db,
        None,
        "ABC v",
        rich(),
        Look {
            icon: Some("icon:star".into()),
            color: Some("teal".into()),
        },
        UI,
    )
    .await?;
    let t = history::step(
        "Save as template",
        view_templates::from_view(&db, v.id, None, UI),
    )
    .await?;
    assert_eq!(
        (
            t.name.as_str(),
            &t.def,
            t.icon.as_deref(),
            t.color.as_deref()
        ),
        ("ABC v", &rich(), Some("icon:star"), Some("teal"))
    );
    undo::undo(&db).await?;
    assert_eq!(view_templates::list(&db).await?.len(), 0);
    Ok(())
}

#[tokio::test]
async fn deleting_a_template_unlinks_views_and_the_default() -> R {
    let db = Db::open_in_memory().await?;
    let t = view_templates::create(&db, "ABC t", rich(), None, None, UI).await?;
    let v = view_templates::create_view(&db, t.id, None, UI).await?;
    settings::set(&db, "default_view_template", &t.id.0).await?;
    history::step("Delete template", view_templates::delete(&db, t.id, UI)).await?;
    let after = views::list(&db).await?;
    assert_eq!(
        after.iter().find(|x| x.id == v.id).map(|x| x.template_id),
        Some(None)
    );
    assert_eq!(
        settings::get::<i64>(&db, "default_view_template").await?,
        None
    );
    undo::undo(&db).await?;
    let back = views::list(&db).await?;
    assert_eq!(
        back.iter().find(|x| x.id == v.id).map(|x| x.template_id),
        Some(Some(t.id))
    );
    Ok(())
}

async fn area_def(db: &Db) -> R {
    properties::create_def(
        db,
        PropertyDef {
            key: "area".into(),
            name: "Area".into(),
            kind: PropertyKind::Text,
            options: vec![],
            project_id: None,
            position: 0,
            applies_to: taskapp_core::PropertyAppliesTo::Issues,
        },
        UI,
    )
    .await?;
    Ok(())
}

fn area() -> FieldRef {
    FieldRef::Property("area".into())
}

#[tokio::test]
async fn deleting_a_property_scrubs_templates_and_undo_restores_them() -> R {
    let db = Db::open_in_memory().await?;
    area_def(&db).await?;
    let def = ViewDef {
        filter: Some(taskapp_core::Filter::Cond {
            field: area(),
            op: taskapp_core::Op::Eq,
            value: json!("web"),
        }),
        sorts: vec![taskapp_core::Sort {
            field: area(),
            descending: false,
        }],
        group_by: Some(area()),
        ..rich()
    };
    let t = view_templates::create(&db, "ABC t", def.clone(), None, None, UI).await?;
    history::step("Delete property", properties::delete_def(&db, "area", UI)).await?;
    let d = view_templates::list(&db).await?.remove(0).def;
    assert_eq!(
        (d.filter, d.sorts, d.group_by, d.columns, d.card_fields),
        (
            None,
            vec![],
            None,
            vec![
                FieldRef::Fixed(FixedField::Priority),
                FieldRef::Fixed(FixedField::CommentCount)
            ],
            vec![]
        )
    );
    let v = view_templates::create_view(&db, t.id, None, UI).await?;
    views::query(&db, &v.def).await?;
    undo::undo(&db).await?;
    assert_eq!(view_templates::list(&db).await?[0].def, def);
    Ok(())
}

#[tokio::test]
async fn a_stale_property_reference_does_not_fail_the_query() -> R {
    let db = Db::open_in_memory().await?;
    let stale = ViewDef {
        filter: Some(taskapp_core::Filter::Cond {
            field: area(),
            op: taskapp_core::Op::Eq,
            value: json!("web"),
        }),
        sorts: vec![taskapp_core::Sort {
            field: area(),
            descending: false,
        }],
        ..ViewDef::default()
    };
    views::query(&db, &stale).await?;
    views::stats(&db, &stale).await?;
    Ok(())
}
