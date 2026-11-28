use taskapp_core::{
    ChangeSource, Db, Error, IssuePatch, NewIssue, NextAction, ProjectId, ProjectStatusId, history,
    issues, labels, next_actions, projects, undo,
};

type R = Result<(), Box<dyn std::error::Error>>;
const UI: ChangeSource = ChangeSource::Ui;

async fn issue(
    db: &Db,
    title: &str,
    project: Option<ProjectId>,
) -> Result<taskapp_core::Issue, Box<dyn std::error::Error>> {
    let new = NewIssue {
        title: title.into(),
        project_id: project,
        ..NewIssue::default()
    };
    Ok(issues::create(db, new, UI).await?)
}

/// Moves the project to the seeded "In progress" status (id 2), which is a Started one.
async fn start(db: &Db, id: ProjectId) -> R {
    let mut p = projects::list(db)
        .await?
        .into_iter()
        .find(|p| p.id == id)
        .ok_or("no such project")?;
    p.status_id = ProjectStatusId(2);
    projects::update(db, p, UI).await?;
    Ok(())
}

async fn row(db: &Db, id: ProjectId) -> Result<NextAction, Box<dyn std::error::Error>> {
    Ok(next_actions::list(db)
        .await?
        .into_iter()
        .find(|n| n.project.id == id)
        .ok_or("no such project")?)
}

#[tokio::test]
async fn an_active_project_is_stuck_until_it_has_an_open_next_action() -> R {
    let db = Db::open_in_memory().await?;
    let p = projects::create(&db, "alpha", "Alpha", None, None, UI).await?;
    // A planned project is not active, so it is never stuck.
    assert!(!row(&db, p.id).await?.stuck);
    start(&db, p.id).await?;
    assert!(row(&db, p.id).await?.stuck);
    let a = issue(&db, "Call", Some(p.id)).await?;
    next_actions::set(&db, p.id, Some(a.id), UI).await?;
    let r = row(&db, p.id).await?;
    assert_eq!((r.stuck, r.issue.map(|i| i.id)), (false, Some(a.id)));
    Ok(())
}

#[tokio::test]
async fn area_and_collection_labels_are_never_stuck() -> R {
    let db = Db::open_in_memory().await?;
    let p = projects::create(&db, "home", "Home", None, None, UI).await?;
    start(&db, p.id).await?;
    let area = labels::create(&db, "Area", "blue", None, UI).await?;
    let mut with = projects::list(&db).await?.remove(0);
    with.label_ids = vec![area.id];
    projects::update(&db, with, UI).await?;
    assert!(!row(&db, p.id).await?.stuck);
    Ok(())
}

#[tokio::test]
async fn finishing_or_moving_the_issue_leaves_the_project_stuck() -> R {
    let db = Db::open_in_memory().await?;
    let p = projects::create(&db, "alpha", "Alpha", None, None, UI).await?;
    start(&db, p.id).await?;
    let a = issue(&db, "One", Some(p.id)).await?;
    let b = issue(&db, "Two", Some(p.id)).await?;
    next_actions::set(&db, p.id, Some(a.id), UI).await?;
    let done = taskapp_core::statuses::list(&db)
        .await?
        .into_iter()
        .find(|s| s.category == taskapp_core::StatusCategory::Completed)
        .ok_or("no done status")?;
    let finish = IssuePatch {
        status_id: Some(done.id),
        ..IssuePatch::default()
    };
    history::step("Finish", issues::update(&db, a.id, finish, UI)).await?;
    assert!(row(&db, p.id).await?.stuck);
    undo::undo(&db).await?;
    assert!(!row(&db, p.id).await?.stuck);
    next_actions::set(&db, p.id, Some(b.id), UI).await?;
    let out = IssuePatch {
        project_id: Some(None),
        ..IssuePatch::default()
    };
    issues::update(&db, b.id, out, UI).await?;
    assert!(row(&db, p.id).await?.stuck);
    Ok(())
}

#[tokio::test]
async fn set_checks_the_issue_and_undoes() -> R {
    let db = Db::open_in_memory().await?;
    let p = projects::create(&db, "alpha", "Alpha", None, None, UI).await?;
    let q = projects::create(&db, "beta", "Beta", None, None, UI).await?;
    let elsewhere = issue(&db, "Other", Some(q.id)).await?;
    let a = issue(&db, "Mine", Some(p.id)).await?;
    let err = next_actions::set(&db, p.id, Some(elsewhere.id), UI).await;
    assert!(matches!(err, Err(Error::Invalid(_))), "{err:?}");
    history::step(
        "Set next action",
        next_actions::set(&db, p.id, Some(a.id), UI),
    )
    .await?;
    assert!(row(&db, p.id).await?.issue.is_some());
    undo::undo(&db).await?;
    assert!(row(&db, p.id).await?.issue.is_none());
    undo::redo(&db).await?;
    next_actions::set(&db, p.id, None, UI).await?;
    assert!(row(&db, p.id).await?.issue.is_none());
    Ok(())
}
