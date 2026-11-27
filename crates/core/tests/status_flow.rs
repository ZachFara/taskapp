use taskapp_core::{ChangeSource::Ui, Db, FlowKind, StatusCategory, history, statuses, undo};

type R = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn kinds_default_from_category() -> R {
    let db = Db::open_in_memory().await?;
    let kinds: Vec<_> = statuses::list(&db)
        .await?
        .into_iter()
        .map(|s| (s.category, s.flow_kind))
        .collect();
    assert!(kinds.contains(&(StatusCategory::Started, FlowKind::Active)));
    assert!(kinds.contains(&(StatusCategory::Unstarted, FlowKind::Idle)));
    let s = statuses::create(&db, "Review", StatusCategory::Started, "gray", None, Ui).await?;
    assert_eq!(
        (s.flow_kind, s.definition.as_str(), s.nudge_days),
        (FlowKind::Active, "", None)
    );
    let s = statuses::create(&db, "Parked", StatusCategory::Backlog, "gray", None, Ui).await?;
    assert_eq!(s.flow_kind, FlowKind::Idle);
    Ok(())
}

#[tokio::test]
async fn set_flow_edits_validates_and_undoes() -> R {
    let db = Db::open_in_memory().await?;
    let s = statuses::create(&db, "Blocked", StatusCategory::Started, "red", None, Ui).await?;
    let s = history::step(
        "Edit status",
        statuses::set_flow(
            &db,
            s.id,
            Some("Waiting on someone else".into()),
            Some(FlowKind::Waiting),
            Some(5),
            Ui,
        ),
    )
    .await?;
    assert_eq!(
        (s.definition.as_str(), s.flow_kind, s.nudge_days),
        ("Waiting on someone else", FlowKind::Waiting, Some(5))
    );
    // Zero clears the nudge; a negative number is refused.
    let s = history::step(
        "Edit status",
        statuses::set_flow(&db, s.id, None, None, Some(0), Ui),
    )
    .await?;
    assert_eq!((s.flow_kind, s.nudge_days), (FlowKind::Waiting, None));
    assert!(
        statuses::set_flow(&db, s.id, None, None, Some(-2), Ui)
            .await
            .is_err()
    );
    undo::undo(&db).await?;
    undo::undo(&db).await?;
    let s = statuses::list(&db)
        .await?
        .into_iter()
        .find(|x| x.name == "Blocked")
        .expect("kept");
    assert_eq!((s.flow_kind, s.nudge_days), (FlowKind::Active, None));
    assert_eq!(s.definition, "");
    Ok(())
}

#[tokio::test]
async fn list_is_ordered_by_category_then_position() -> R {
    let db = Db::open_in_memory().await?;
    // Created last, so it has the highest position, but Backlog still comes first.
    statuses::create(&db, "Icebox", StatusCategory::Backlog, "gray", None, Ui).await?;
    let names: Vec<_> = statuses::list(&db)
        .await?
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(names[..2], ["Backlog", "Icebox"]);
    assert_eq!(names.last().map(String::as_str), Some("Canceled"));
    Ok(())
}
