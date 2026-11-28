use chrono::NaiveDate;
use taskapp_core::{
    ChangeSource::Ui, Db, Deferred, InboxChoice, IssueId, IssuePatch, NewIssue, StatusCategory,
    ViewDef, comments, inbox, issues, statuses, views,
};

type R = Result<(), Box<dyn std::error::Error>>;

fn day(s: &str) -> NaiveDate {
    s.parse().expect("date")
}

/// A workspace with the owner's statuses and three Inbox issues (the seeded Backlog status is renamed Inbox).
async fn setup() -> Result<(Db, Vec<IssueId>), Box<dyn std::error::Error>> {
    let db = Db::open_in_memory().await?;
    sqlx::query("UPDATE statuses SET name = 'Inbox' WHERE name = 'Backlog'")
        .execute(&db.pool)
        .await?;
    for (n, c) in [
        ("Next", StatusCategory::Unstarted),
        ("Delegated", StatusCategory::Started),
        ("Someday", StatusCategory::Backlog),
    ] {
        statuses::create(&db, n, c, "gray", None, Ui).await?;
    }
    let mut ids = vec![];
    for t in ["a", "b", "c"] {
        ids.push(
            issues::create(
                &db,
                NewIssue {
                    title: t.into(),
                    ..NewIssue::default()
                },
                Ui,
            )
            .await?
            .id,
        );
    }
    Ok((db, ids))
}

async fn status_name(db: &Db, id: IssueId) -> Result<String, Box<dyn std::error::Error>> {
    let i = issues::get(db, id).await?;
    Ok(statuses::list(db)
        .await?
        .into_iter()
        .find(|s| s.id == i.status_id)
        .map(|s| s.name)
        .unwrap_or_default())
}

#[tokio::test]
async fn lists_inbox_issues_oldest_first_and_skips_deferred() -> R {
    let (db, ids) = setup().await?;
    let today = day("2026-12-01");
    assert_eq!(
        inbox::list(&db, today)
            .await?
            .iter()
            .map(|i| i.id)
            .collect::<Vec<_>>(),
        ids
    );
    inbox::process(
        &db,
        ids[0],
        InboxChoice::Defer {
            until: day("2026-12-05"),
        },
        today,
        Ui,
    )
    .await?;
    assert_eq!(inbox::list(&db, today).await?.len(), 2);
    // It comes back on the day.
    assert_eq!(inbox::list(&db, day("2026-12-05")).await?.len(), 3);
    Ok(())
}

#[tokio::test]
async fn each_choice_sets_status() -> R {
    let (db, ids) = setup().await?;
    let today = day("2026-12-01");
    for (i, (choice, want)) in [
        (InboxChoice::Done, "Done"),
        (InboxChoice::Someday, "Someday"),
        (InboxChoice::Next { project: None }, "Next"),
    ]
    .into_iter()
    .enumerate()
    {
        inbox::process(&db, ids[i], choice, today, Ui).await?;
        assert_eq!(status_name(&db, ids[i]).await?, want);
    }
    let d = inbox::process(
        &db,
        ids[0],
        InboxChoice::Delegate {
            note: " Sam ".into(),
        },
        today,
        Ui,
    )
    .await?;
    assert_eq!(status_name(&db, d.id).await?, "Delegated");
    assert_eq!(comments::list(&db, d.id).await?[0].body, "Waiting on: Sam");
    Ok(())
}

#[tokio::test]
async fn trash_archives_and_bad_input_is_refused() -> R {
    let (db, ids) = setup().await?;
    let today = day("2026-12-01");
    let t = inbox::process(&db, ids[1], InboxChoice::Trash, today, Ui).await?;
    assert!(t.archived_at.is_some());
    assert!(
        inbox::process(&db, ids[0], InboxChoice::Defer { until: today }, today, Ui)
            .await
            .is_err()
    );
    assert!(
        inbox::process(
            &db,
            ids[0],
            InboxChoice::Delegate { note: "  ".into() },
            today,
            Ui
        )
        .await
        .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn hidden_issues_leave_views_until_cleared() -> R {
    let (db, ids) = setup().await?;
    let patch = IssuePatch {
        hidden_until: Some(Some(day("2999-01-01"))),
        ..IssuePatch::default()
    };
    issues::update(&db, ids[0], patch, Ui).await?;
    assert_eq!(views::query(&db, &ViewDef::default()).await?.len(), 2);
    let all = ViewDef {
        deferred: Deferred::Show,
        ..ViewDef::default()
    };
    assert_eq!(views::query(&db, &all).await?.len(), 3);
    issues::update(
        &db,
        ids[0],
        IssuePatch {
            hidden_until: Some(None),
            ..IssuePatch::default()
        },
        Ui,
    )
    .await?;
    assert_eq!(views::query(&db, &ViewDef::default()).await?.len(), 3);
    Ok(())
}
