use chrono::NaiveDate;
use taskapp_core::{
    ChangeSource::Ui, Db, Deferred, Due, InboxChoice, IssueId, IssuePatch, NewIssue,
    StatusCategory, ViewDef, comments, inbox, issues, next_actions, projects, statuses, tickler,
    views,
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
        .map_or_else(String::new, |s| s.name))
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
        (
            InboxChoice::Next {
                project: None,
                as_next_action: false,
            },
            "Next",
        ),
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
async fn next_can_also_flag_the_project_next_action() -> R {
    let (db, ids) = setup().await?;
    let today = day("2026-12-01");
    let p = projects::create(&db, "alpha", "Alpha", None, None, Ui).await?;
    let flagged = inbox::process(
        &db,
        ids[0],
        InboxChoice::Next {
            project: Some(p.id),
            as_next_action: true,
        },
        today,
        Ui,
    )
    .await?;
    assert_eq!(flagged.project_id, Some(p.id));
    let flag = |list: Vec<taskapp_core::NextAction>| {
        list.into_iter()
            .find(|n| n.project.id == p.id)
            .and_then(|n| n.issue)
            .map(|i| i.id)
    };
    assert_eq!(flag(next_actions::list(&db).await?), Some(ids[0]));
    // A next action needs a project.
    assert!(
        inbox::process(
            &db,
            ids[1],
            InboxChoice::Next {
                project: None,
                as_next_action: true
            },
            today,
            Ui
        )
        .await
        .is_err()
    );
    // Without the flag the project keeps its next action.
    inbox::process(
        &db,
        ids[2],
        InboxChoice::Next {
            project: Some(p.id),
            as_next_action: false,
        },
        today,
        Ui,
    )
    .await?;
    assert_eq!(flag(next_actions::list(&db).await?), Some(ids[0]));
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

#[tokio::test]
async fn tickler_lists_deferred_issues_and_defer_clears() -> R {
    let (db, ids) = setup().await?;
    let today = day("2026-12-01");
    assert_eq!(tickler::list(&db, today).await?.len(), 0);
    tickler::defer(&db, ids[1], Some(day("2026-12-09")), today, Ui).await?;
    tickler::defer(&db, ids[0], Some(day("2026-12-03")), today, Ui).await?;
    let keys: Vec<_> = tickler::list(&db, today)
        .await?
        .iter()
        .map(|i| i.id)
        .collect();
    assert_eq!(keys, vec![ids[0], ids[1]], "earliest return day first");
    // On its day an issue is back in the Inbox, not on the tickler.
    assert_eq!(tickler::list(&db, day("2026-12-03")).await?.len(), 1);
    assert!(
        tickler::defer(&db, ids[2], Some(today), today, Ui)
            .await
            .is_err()
    );
    let cleared = tickler::defer(&db, ids[0], None, today, Ui).await?;
    assert_eq!(cleared.hidden_until, None);
    assert_eq!(tickler::list(&db, today).await?.len(), 1);
    Ok(())
}

#[tokio::test]
async fn schedule_sets_due_and_moves_to_reminder_or_first_unstarted() -> R {
    let (db, ids) = setup().await?;
    let today = day("2026-12-01");
    let i = inbox::process(
        &db,
        ids[0],
        InboxChoice::Schedule {
            date: day("2026-12-04"),
            time: None,
        },
        today,
        Ui,
    )
    .await?;
    assert_eq!(i.due, Some(Due::Date(day("2026-12-04"))));
    assert_eq!(
        status_name(&db, ids[0]).await?,
        "Todo",
        "no Reminder status: first unstarted"
    );
    statuses::create(&db, "Reminder", StatusCategory::Unstarted, "gray", None, Ui).await?;
    taskapp_core::settings::set(&db, "timezone", &"Asia/Tokyo").await?;
    let i = inbox::process(
        &db,
        ids[1],
        InboxChoice::Schedule {
            date: day("2026-12-04"),
            time: Some("09:30".into()),
        },
        today,
        Ui,
    )
    .await?;
    assert_eq!(status_name(&db, ids[1]).await?, "Reminder");
    let Some(Due::DateTime { utc, tz }) = i.due else {
        panic!("a timed due");
    };
    assert_eq!(
        (utc.to_rfc3339(), tz.as_str()),
        ("2026-12-04T00:30:00+00:00".into(), "Asia/Tokyo")
    );
    let bad = InboxChoice::Schedule {
        date: today,
        time: Some("25:00".into()),
    };
    assert!(inbox::process(&db, ids[2], bad, today, Ui).await.is_err());
    Ok(())
}
