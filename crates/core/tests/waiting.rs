use chrono::NaiveDate;
use taskapp_core::{ChangeSource::Ui, Db, IssuePatch, NewIssue, issues, waiting};

type R = Result<(), Box<dyn std::error::Error>>;

fn day(s: &str) -> NaiveDate {
    s.parse().expect("date")
}

async fn make(db: &Db, title: &str) -> Result<taskapp_core::IssueId, Box<dyn std::error::Error>> {
    Ok(issues::create(
        db,
        NewIssue {
            title: title.into(),
            ..NewIssue::default()
        },
        Ui,
    )
    .await?
    .id)
}

#[tokio::test]
async fn waiting_fields_round_trip_and_clear() -> R {
    let db = Db::open_in_memory().await?;
    let id = make(&db, "ask Sam").await?;
    let patch = IssuePatch {
        waiting_on: Some(Some("Sam".into())),
        follow_up: Some(Some(day("2026-12-03"))),
        ..IssuePatch::default()
    };
    issues::update(&db, id, patch, Ui).await?;
    let got = issues::get(&db, id).await?;
    assert_eq!(got.waiting_on.as_deref(), Some("Sam"));
    assert_eq!(got.follow_up, Some(day("2026-12-03")));
    let cleared = IssuePatch {
        waiting_on: Some(None),
        follow_up: Some(None),
        ..IssuePatch::default()
    };
    issues::update(&db, id, cleared, Ui).await?;
    let got = issues::get(&db, id).await?;
    assert_eq!(got.waiting_on, None);
    assert_eq!(got.follow_up, None);
    Ok(())
}

#[tokio::test]
async fn list_has_open_waiting_issues_with_due_follow_ups_first() -> R {
    let db = Db::open_in_memory().await?;
    let later = make(&db, "later").await?;
    let undated = make(&db, "undated").await?;
    let due = make(&db, "due").await?;
    let done = make(&db, "done").await?;
    let plain = make(&db, "plain").await?;
    let set = |on: &str, until: Option<&str>| IssuePatch {
        waiting_on: Some(Some(on.into())),
        follow_up: Some(until.map(day)),
        ..IssuePatch::default()
    };
    issues::update(&db, later, set("Ana", Some("2999-01-01")), Ui).await?;
    issues::update(&db, undated, set("Bo", None), Ui).await?;
    issues::update(&db, due, set("Cy", Some("2026-11-01")), Ui).await?;
    issues::update(&db, done, set("Di", Some("2026-11-01")), Ui).await?;
    issues::update(&db, plain, IssuePatch::default(), Ui).await?;
    let completed: i64 =
        sqlx::query_scalar("SELECT id FROM statuses WHERE category = 'completed' LIMIT 1")
            .fetch_one(&db.pool)
            .await?;
    let finish = IssuePatch {
        status_id: Some(taskapp_core::StatusId(completed)),
        ..IssuePatch::default()
    };
    issues::update(&db, done, finish, Ui).await?;
    let titles: Vec<String> = waiting::list(&db)
        .await?
        .into_iter()
        .map(|i| i.title)
        .collect();
    assert_eq!(titles, ["due", "later", "undated"]);
    Ok(())
}
