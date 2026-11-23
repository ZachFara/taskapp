use chrono::{NaiveDate, NaiveDateTime};
use taskapp_core::{
    ChangeSource, Db, Error, EventInput, IssueId, NewIssue, events, history, issues, undo,
};

type R = Result<(), Box<dyn std::error::Error>>;
const UI: ChangeSource = ChangeSource::Ui;

fn at(d: u32, h: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 12, d)
        .and_then(|d| d.and_hms_opt(h, 0, 0))
        .expect("time")
}

fn input(title: &str, d: u32) -> EventInput {
    EventInput {
        title: title.into(),
        starts_at: at(d, 9),
        ends_at: None,
        all_day: false,
        place: None,
        notes: String::new(),
        icon: None,
        color: None,
        archived: false,
    }
}

async fn issue(db: &Db) -> Result<IssueId, Error> {
    let new = NewIssue {
        title: "ABC one".into(),
        ..NewIssue::default()
    };
    Ok(issues::create(db, new, UI).await?.id)
}

async fn count(db: &Db, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
        .fetch_one(&db.pool)
        .await
        .expect("count")
}

#[tokio::test]
async fn create_list_range_update_and_validation() -> R {
    let db = Db::open_in_memory().await?;
    let a = events::create(&db, input("Planning", 3), UI).await?;
    let mut long = input("Offsite", 10);
    long.ends_at = Some(at(12, 17));
    let b = events::create(&db, long, UI).await?;
    assert_eq!(events::list(&db, None, None).await?.len(), 2);
    let day = |d| NaiveDate::from_ymd_opt(2026, 12, d);
    // A multi-day event shows on every day it spans.
    let mid = events::list(&db, day(11), day(11)).await?;
    assert_eq!(mid.iter().map(|e| e.id).collect::<Vec<_>>(), vec![b.id]);
    assert_eq!(events::list(&db, day(4), None).await?.len(), 1);

    let mut edit = input("Planning 2", 3);
    edit.place = Some("Room 4".into());
    edit.color = Some("blue".into());
    let got = events::update(&db, a.id, edit, UI).await?;
    assert_eq!(
        (got.title.as_str(), got.place.as_deref()),
        ("Planning 2", Some("Room 4"))
    );

    let mut bad = input("x", 3);
    bad.ends_at = Some(at(2, 9));
    assert!(matches!(
        events::create(&db, bad, UI).await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        events::create(&db, input(" ", 3), UI).await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        events::get(&db, taskapp_core::EventId(99)).await,
        Err(Error::NotFound(_))
    ));
    Ok(())
}

#[tokio::test]
async fn link_is_idempotent_and_delete_keeps_the_issue() -> R {
    let db = Db::open_in_memory().await?;
    let i = issue(&db).await?;
    let e = events::create(&db, input("Demo", 5), UI).await?;
    events::link(&db, e.id, i, UI).await?;
    events::link(&db, e.id, i, UI).await?;
    assert_eq!(events::get(&db, e.id).await?.issue_ids, vec![i]);
    assert!(matches!(
        events::link(&db, e.id, IssueId(999), UI).await,
        Err(Error::NotFound(_))
    ));
    events::unlink(&db, e.id, i, UI).await?;
    events::unlink(&db, e.id, i, UI).await?;
    assert_eq!(events::get(&db, e.id).await?.issue_ids, vec![]);
    events::link(&db, e.id, i, UI).await?;
    events::delete(&db, e.id, UI).await?;
    assert_eq!(count(&db, "event_issues").await, 0);
    assert_eq!(issues::get(&db, i).await?.title, "ABC one");
    Ok(())
}

#[tokio::test]
async fn undo_and_redo_cover_every_event_write() -> R {
    let db = Db::open_in_memory().await?;
    let i = issue(&db).await?;
    let e = history::step("Create event", events::create(&db, input("Demo", 5), UI)).await?;
    history::step("Link event", events::link(&db, e.id, i, UI)).await?;
    let mut edit = input("Demo 2", 6);
    edit.all_day = true;
    history::step("Update event", events::update(&db, e.id, edit, UI)).await?;
    history::step("Delete event", events::delete(&db, e.id, UI)).await?;
    assert_eq!(count(&db, "events").await, 0);

    undo::undo(&db).await?;
    let back = events::get(&db, e.id).await?;
    assert_eq!(
        (back.title.as_str(), back.issue_ids.clone()),
        ("Demo 2", vec![i])
    );
    undo::undo(&db).await?;
    assert_eq!(events::get(&db, e.id).await?.title, "Demo");
    undo::undo(&db).await?;
    assert_eq!(events::get(&db, e.id).await?.issue_ids, vec![]);
    undo::undo(&db).await?;
    assert_eq!(count(&db, "events").await, 0);
    for _ in 0..4 {
        undo::redo(&db).await?;
    }
    assert_eq!(count(&db, "events").await, 0);
    undo::undo(&db).await?;
    assert_eq!(events::get(&db, e.id).await?.issue_ids, vec![i]);
    Ok(())
}
