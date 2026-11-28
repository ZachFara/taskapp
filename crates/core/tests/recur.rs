use chrono::{Days, NaiveDate};
use taskapp_core::{
    ChangeSource::Ui, Db, Due, Issue, IssuePatch, NewIssue, StatusCategory, StatusId, ViewDef,
    bulk, history, issues, labels, metrics, relations, statuses, undo, views,
};

type R = Result<(), Box<dyn std::error::Error>>;

async fn status(db: &Db, c: StatusCategory) -> Result<StatusId, taskapp_core::Error> {
    Ok(statuses::list(db)
        .await?
        .into_iter()
        .find(|s| s.category == c)
        .expect("status")
        .id)
}

fn patch(f: impl FnOnce(&mut IssuePatch)) -> IssuePatch {
    let mut p = IssuePatch::default();
    f(&mut p);
    p
}

async fn repeating(db: &Db, rule: &str, due: &str) -> Result<Issue, Box<dyn std::error::Error>> {
    let label = labels::create(db, "chore", "blue", None, Ui).await?;
    let a = issues::create(
        db,
        NewIssue {
            title: "Water plants".into(),
            description: Some("- [x] front\n- [ ] back".into()),
            points: Some(2.0),
            label_ids: Some(vec![label.id]),
            due: Some(Due::Date(due.parse()?)),
            ..NewIssue::default()
        },
        Ui,
    )
    .await?;
    let rule = rule.to_string();
    Ok(issues::update(db, a.id, patch(|p| p.repeat = Some(Some(rule))), Ui).await?)
}

async fn complete(db: &Db, id: taskapp_core::IssueId) -> Result<(), taskapp_core::Error> {
    let done = status(db, StatusCategory::Completed).await?;
    issues::update(db, id, patch(|p| p.status_id = Some(done)), Ui).await?;
    Ok(())
}

async fn all(db: &Db) -> Result<Vec<Issue>, taskapp_core::Error> {
    views::query(db, &ViewDef::default()).await
}

#[tokio::test]
async fn setting_a_rule_stores_it_canonical_and_a_bad_one_is_refused() -> R {
    let db = Db::open_in_memory().await?;
    let a = repeating(&db, "every monday", "2999-01-01").await?;
    assert_eq!(a.repeat.as_deref(), Some("every 1 week on mon from due"));
    let bad = issues::update(
        &db,
        a.id,
        patch(|p| p.repeat = Some(Some("whenever".into()))),
        Ui,
    )
    .await;
    assert!(bad.is_err());
    let cleared = issues::update(&db, a.id, patch(|p| p.repeat = Some(None)), Ui).await?;
    assert_eq!(cleared.repeat, None);
    Ok(())
}

#[tokio::test]
async fn completing_creates_the_next_occurrence_and_links_it() -> R {
    let db = Db::open_in_memory().await?;
    let a = repeating(&db, "every 3 days", "2999-01-01").await?;
    complete(&db, a.id).await?;
    let list = all(&db).await?;
    assert_eq!(list.len(), 2);
    let next = list.iter().find(|i| i.id != a.id).expect("next");
    assert_eq!(next.title, "Water plants");
    assert_eq!(next.description, "- [ ] front\n- [ ] back");
    assert_eq!(next.points, Some(2.0));
    assert_eq!(next.label_ids, a.label_ids);
    assert_eq!(next.due, Some(Due::Date("2999-01-04".parse()?)));
    assert_eq!(next.repeat.as_deref(), Some("every 3 day from due"));
    assert_eq!(next.completed_at, None);
    // The finished one stops repeating, so reopening and finishing it again adds nothing.
    assert_eq!(issues::get(&db, a.id).await?.repeat, None);
    let rels = relations::for_issue(&db, next.id).await?;
    assert!(
        rels.iter()
            .any(|r| r.from == next.id && r.to == a.id && r.kind.0 == "follows")
    );
    Ok(())
}

#[tokio::test]
async fn from_completion_counts_from_today() -> R {
    let db = Db::open_in_memory().await?;
    let a = repeating(&db, "every 2 days from completion", "2999-01-01").await?;
    complete(&db, a.id).await?;
    let today: NaiveDate = metrics::today(&db).await?;
    let next = all(&db)
        .await?
        .into_iter()
        .find(|i| i.id != a.id)
        .expect("next");
    assert_eq!(next.due, Some(Due::Date(today + Days::new(2))));
    Ok(())
}

#[tokio::test]
async fn cancelling_does_not_repeat() -> R {
    let db = Db::open_in_memory().await?;
    let a = repeating(&db, "daily", "2999-01-01").await?;
    let c = status(&db, StatusCategory::Canceled).await?;
    issues::update(&db, a.id, patch(|p| p.status_id = Some(c)), Ui).await?;
    assert_eq!(all(&db).await?.len(), 1);
    Ok(())
}

#[tokio::test]
async fn bulk_completion_repeats_and_one_undo_removes_the_copy_and_redo_does_not_double_it() -> R {
    let db = Db::open_in_memory().await?;
    let a = repeating(&db, "weekly", "2999-01-01").await?;
    let done = status(&db, StatusCategory::Completed).await?;
    let bp = taskapp_core::BulkPatch {
        patch: patch(|p| p.status_id = Some(done)),
        ..Default::default()
    };
    history::step("Complete", bulk::update(&db, &[a.id], bp, Ui)).await?;
    assert_eq!(all(&db).await?.len(), 2);
    undo::undo(&db).await?;
    let list = all(&db).await?;
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].repeat.as_deref(), Some("every 1 week from due"));
    undo::redo(&db).await?;
    let list = all(&db).await?;
    assert_eq!(list.len(), 2);
    assert_eq!(issues::get(&db, a.id).await?.repeat, None);
    Ok(())
}

#[tokio::test]
async fn skipping_moves_the_due_date_and_creates_nothing() -> R {
    let db = Db::open_in_memory().await?;
    let a = repeating(&db, "every 3 days", "2999-01-01").await?;
    let skipped = issues::skip_repeat(&db, a.id, Ui).await?;
    assert_eq!(skipped.due, Some(Due::Date("2999-01-04".parse()?)));
    assert_eq!(all(&db).await?.len(), 1);
    let plain = issues::create(
        &db,
        NewIssue {
            title: "x".into(),
            ..NewIssue::default()
        },
        Ui,
    )
    .await?;
    assert!(issues::skip_repeat(&db, plain.id, Ui).await.is_err());
    Ok(())
}
