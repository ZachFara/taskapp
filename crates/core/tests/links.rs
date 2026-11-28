use taskapp_core::{
    ChangeSource, Db, Error, IssueId, LinkKind, NewIssue, history, issues, links, undo,
};

type R = Result<(), Box<dyn std::error::Error>>;
const UI: ChangeSource = ChangeSource::Ui;

async fn issue(db: &Db) -> Result<IssueId, Error> {
    let new = NewIssue {
        title: "ABC one".into(),
        ..NewIssue::default()
    };
    Ok(issues::create(db, new, UI).await?.id)
}

#[tokio::test]
async fn add_lists_is_idempotent_and_validates() -> R {
    let db = Db::open_in_memory().await?;
    let id = issue(&db).await?;
    let pr = "https://example.com/pr/1";
    let a = links::add(
        &db,
        id,
        pr,
        Some(" Fix ".into()),
        Some(LinkKind::Solution),
        UI,
    )
    .await?;
    assert_eq!((a.title.as_str(), a.kind), ("Fix", LinkKind::Solution));
    let again = links::add(&db, id, pr, None, None, UI).await?;
    assert_eq!(again, a);
    links::add(&db, id, "taskapp://issue/ABC-2", None, None, UI).await?;
    assert_eq!(links::list(&db, id).await?.len(), 2);

    for url in [
        "javascript:alert(1)",
        "file:///etc/passwd",
        "two words",
        "",
        "example.com",
    ] {
        assert!(matches!(
            links::add(&db, id, url, None, None, UI).await,
            Err(Error::Invalid(_))
        ));
    }
    assert!(matches!(
        links::add(&db, IssueId(999), pr, None, None, UI).await,
        Err(Error::NotFound(_))
    ));
    Ok(())
}

#[tokio::test]
async fn add_and_remove_are_undoable() -> R {
    let db = Db::open_in_memory().await?;
    let id = issue(&db).await?;
    let l = history::step(
        "Add link",
        links::add(&db, id, "https://example.com/a", None, None, UI),
    )
    .await?;
    undo::undo(&db).await?;
    assert_eq!(links::list(&db, id).await?, vec![]);
    undo::redo(&db).await?;
    assert_eq!(links::list(&db, id).await?, vec![l.clone()]);
    history::step("Remove link", links::remove(&db, l.id, UI)).await?;
    links::remove(&db, l.id, UI).await?;
    assert_eq!(links::list(&db, id).await?, vec![]);
    undo::undo(&db).await?;
    assert_eq!(links::list(&db, id).await?, vec![l]);
    Ok(())
}
