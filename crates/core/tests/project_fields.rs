use serde_json::json;
use taskapp_core::{
    ChangeSource, Db, Error, Project, ProjectStatusCategory, PropertyAppliesTo, PropertyDef,
    PropertyKind, history, labels, project_statuses, projects, properties, undo,
};

type R = Result<(), Box<dyn std::error::Error>>;
const UI: ChangeSource = ChangeSource::Ui;

fn def(key: &str, applies_to: PropertyAppliesTo) -> PropertyDef {
    PropertyDef {
        key: key.into(),
        name: key.into(),
        kind: PropertyKind::Text,
        options: vec![],
        project_id: None,
        position: 0,
        applies_to,
    }
}

#[tokio::test]
async fn new_projects_start_in_the_first_status_and_edits_undo() -> R {
    let db = Db::open_in_memory().await?;
    let statuses = project_statuses::list(&db).await?;
    assert_eq!(statuses.len(), 5);
    let p = projects::create(&db, "alpha", "Alpha", None, None, UI).await?;
    assert_eq!(p.status_id, statuses[0].id);
    let label = labels::create(&db, "bug", "red", None, UI).await?;
    properties::create_def(&db, def("owner", PropertyAppliesTo::Both), UI).await?;
    let mut edited = p.clone();
    edited.status_id = statuses[1].id;
    edited.priority = 3;
    edited.label_ids = vec![label.id];
    edited.properties.insert("owner".into(), json!("sam"));
    let saved = history::step("Edit project", projects::update(&db, edited.clone(), UI)).await?;
    assert_eq!(projects::list(&db).await?[0], saved);
    assert_eq!(saved.priority, 3);
    undo::undo(&db).await?;
    // An undo is a write too, so updated_at moves on.
    let back = projects::list(&db).await?.remove(0);
    assert_eq!(
        Project {
            updated_at: back.updated_at,
            ..p.clone()
        },
        back
    );
    undo::redo(&db).await?;
    assert_eq!(projects::list(&db).await?[0].priority, edited.priority);
    Ok(())
}

#[tokio::test]
async fn bad_priority_status_and_property_values_are_refused() -> R {
    let db = Db::open_in_memory().await?;
    let p = projects::create(&db, "alpha", "Alpha", None, None, UI).await?;
    properties::create_def(&db, def("issue_only", PropertyAppliesTo::Issues), UI).await?;
    let mut bad = p.clone();
    bad.priority = 9;
    assert!(matches!(
        projects::update(&db, bad, UI).await,
        Err(Error::Invalid(_))
    ));
    let mut bad = p.clone();
    bad.properties.insert("issue_only".into(), json!("x"));
    assert!(matches!(
        projects::update(&db, bad, UI).await,
        Err(Error::Invalid(_))
    ));
    let mut bad = p.clone();
    bad.status_id = taskapp_core::ProjectStatusId(999);
    assert!(matches!(
        projects::update(&db, bad, UI).await,
        Err(Error::NotFound(_))
    ));
    Ok(())
}

#[tokio::test]
async fn deleting_a_status_moves_its_projects_and_undo_restores_both() -> R {
    let db = Db::open_in_memory().await?;
    let s = project_statuses::list(&db).await?;
    let p = projects::create(&db, "alpha", "Alpha", None, None, UI).await?;
    history::step(
        "Delete project status",
        project_statuses::delete(&db, s[0].id, s[3].id, UI),
    )
    .await?;
    assert_eq!(projects::list(&db).await?[0].status_id, s[3].id);
    assert_eq!(project_statuses::list(&db).await?.len(), 4);
    undo::undo(&db).await?;
    // An undo is a write too, so updated_at moves on.
    let back = projects::list(&db).await?.remove(0);
    assert_eq!(
        Project {
            updated_at: back.updated_at,
            ..p.clone()
        },
        back
    );
    assert_eq!(project_statuses::list(&db).await?.len(), 5);
    assert!(
        project_statuses::delete(&db, s[0].id, s[0].id, UI)
            .await
            .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn statuses_can_be_created_renamed_and_reordered() -> R {
    let db = Db::open_in_memory().await?;
    let made = project_statuses::create(
        &db,
        "Blocked",
        ProjectStatusCategory::Paused,
        "red",
        None,
        UI,
    )
    .await?;
    let dup = project_statuses::create(
        &db,
        "Blocked",
        ProjectStatusCategory::Paused,
        "red",
        None,
        UI,
    )
    .await;
    assert!(matches!(dup, Err(Error::Conflict(_))));
    project_statuses::update(
        &db,
        made.id,
        Some("Stuck".into()),
        taskapp_core::palette::Look::default(),
        UI,
    )
    .await?;
    let mut ids: Vec<_> = project_statuses::list(&db)
        .await?
        .iter()
        .map(|s| s.id)
        .collect();
    ids.reverse();
    project_statuses::reorder(&db, &ids, UI).await?;
    let now = project_statuses::list(&db).await?;
    assert_eq!(now[0].name, "Stuck");
    assert!(project_statuses::reorder(&db, &ids[1..], UI).await.is_err());
    Ok(())
}

#[tokio::test]
async fn applies_to_guards_values_and_conversion_covers_projects() -> R {
    let db = Db::open_in_memory().await?;
    let p = projects::create(&db, "alpha", "Alpha", None, None, UI).await?;
    properties::create_def(&db, def("owner", PropertyAppliesTo::Projects), UI).await?;
    let mut with = p.clone();
    with.properties.insert("owner".into(), json!("sam"));
    projects::update(&db, with, UI).await?;
    // Narrowing to issues would orphan the project's value.
    let err = properties::set_applies_to(&db, "owner", PropertyAppliesTo::Issues, UI).await;
    assert!(matches!(err, Err(Error::Conflict(_))));
    properties::set_applies_to(&db, "owner", PropertyAppliesTo::Both, UI).await?;
    // A projects-only property takes no per-project limit.
    let mut limited = def("lim", PropertyAppliesTo::Projects);
    limited.project_id = Some(p.id);
    assert!(properties::create_def(&db, limited, UI).await.is_err());
    // Changing the kind rewrites project values too; deleting the property strips them.
    properties::change_kind(&db, "owner", PropertyKind::Text, vec![], UI).await?;
    properties::delete_def(&db, "owner", UI).await?;
    assert!(projects::list(&db).await?[0].properties.is_empty());
    Ok(())
}

#[tokio::test]
async fn deleting_a_label_or_project_with_labels_undoes_cleanly() -> R {
    let db = Db::open_in_memory().await?;
    let mut p = projects::create(&db, "alpha", "Alpha", None, None, UI).await?;
    let l = labels::create(&db, "bug", "red", None, UI).await?;
    p.label_ids = vec![l.id];
    let p = projects::update(&db, p, UI).await?;
    history::step("Delete label", labels::delete(&db, l.id, UI)).await?;
    assert_eq!(projects::list(&db).await?[0].label_ids, vec![]);
    undo::undo(&db).await?;
    assert_eq!(projects::list(&db).await?[0].label_ids, vec![l.id]);
    history::step("Delete project", projects::delete(&db, p.id, UI)).await?;
    undo::undo(&db).await?;
    // An undo is a write too, so updated_at moves on.
    let back = projects::list(&db).await?.remove(0);
    assert_eq!(
        Project {
            updated_at: back.updated_at,
            ..p.clone()
        },
        back
    );
    Ok(())
}
