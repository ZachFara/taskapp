use taskapp_core::{
    ChangeSource, Db, Error, Goal, GoalStatus, NewIssue, goals, history, issues, projects,
    statuses, undo,
};

type R = Result<(), Box<dyn std::error::Error>>;
const UI: ChangeSource = ChangeSource::Ui;

#[tokio::test]
async fn create_validates_and_lists_by_quarter() -> R {
    let db = Db::open_in_memory().await?;
    let a = goals::create(&db, "Ship v1", "2027-Q2", None, UI).await?;
    assert_eq!((a.status, a.description.as_str()), (GoalStatus::Active, ""));
    let b = goals::create(&db, "Plan", "2027-Q1", Some("why".into()), UI).await?;
    let ids: Vec<_> = goals::list(&db).await?.iter().map(|g| g.id).collect();
    assert_eq!(ids, vec![b.id, a.id]);
    for (title, quarter) in [(" ", "2027-Q1"), ("x", "2027-Q5"), ("x", "Q1"), ("x", "")] {
        let e = goals::create(&db, title, quarter, None, UI).await;
        assert!(matches!(e, Err(Error::Invalid(_))), "{title:?} {quarter:?}");
    }
    let mut changed: Goal = a.clone();
    changed.status = GoalStatus::Achieved;
    changed.title = "Shipped".into();
    let got = goals::update(&db, changed, UI).await?;
    assert_eq!(
        (got.status, got.title.as_str()),
        (GoalStatus::Achieved, "Shipped")
    );
    Ok(())
}

#[tokio::test]
async fn links_progress_and_undo() -> R {
    let db = Db::open_in_memory().await?;
    let p = projects::create(&db, "web", "Web", None, None, UI).await?;
    let q = projects::create(&db, "api", "API", None, None, UI).await?;
    let done = statuses::list(&db)
        .await?
        .into_iter()
        .find(|s| s.category == taskapp_core::StatusCategory::Completed)
        .map(|s| s.id);
    for (project, status) in [(p.id, done), (p.id, None), (q.id, None)] {
        let new = NewIssue {
            title: "t".into(),
            project_id: Some(project),
            status_id: status,
            ..NewIssue::default()
        };
        issues::create(&db, new, UI).await?;
    }
    let g = goals::create(&db, "G", "2027-Q1", None, UI).await?;
    assert_eq!(goals::progress(&db, g.id).await?, (0, 0));
    history::step("Link", goals::link(&db, g.id, p.id, UI)).await?;
    goals::link(&db, g.id, p.id, UI).await?; // idempotent
    goals::link(&db, g.id, q.id, UI).await?;
    assert_eq!(goals::get(&db, g.id).await?.project_ids, vec![p.id, q.id]);
    assert_eq!(goals::progress(&db, g.id).await?, (1, 3));
    assert!(
        goals::link(&db, g.id, taskapp_core::ProjectId(999), UI)
            .await
            .is_err()
    );
    goals::unlink(&db, g.id, q.id, UI).await?;
    assert_eq!(goals::progress(&db, g.id).await?, (1, 2));
    Ok(())
}

#[tokio::test]
async fn delete_goal_and_project_undo() -> R {
    let db = Db::open_in_memory().await?;
    let p = projects::create(&db, "web", "Web", None, None, UI).await?;
    let g = goals::create(&db, "G", "2027-Q1", None, UI).await?;
    goals::link(&db, g.id, p.id, UI).await?;
    history::step("Delete project", projects::delete(&db, p.id, UI)).await?;
    assert_eq!(goals::get(&db, g.id).await?.project_ids, vec![]);
    undo::undo(&db).await?;
    assert_eq!(goals::get(&db, g.id).await?.project_ids, vec![p.id]);
    history::step("Delete goal", goals::delete(&db, g.id, UI)).await?;
    assert!(goals::get(&db, g.id).await.is_err());
    undo::undo(&db).await?;
    assert_eq!(goals::get(&db, g.id).await?.project_ids, vec![p.id]);
    Ok(())
}
