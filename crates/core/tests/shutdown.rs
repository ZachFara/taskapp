use chrono::NaiveDate;
use taskapp_core::{
    ChangeSource, Db, Depth, IssueId, IssuePatch, NewIssue, history, inbox, issues, settings,
    shutdown, undo,
};

type R = Result<(), Box<dyn std::error::Error>>;

fn day(text: &str) -> NaiveDate {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("date")
}

async fn issue(db: &Db, title: &str, deep: bool) -> Result<IssueId, taskapp_core::Error> {
    let made = issues::create(
        db,
        NewIssue {
            title: title.into(),
            ..NewIssue::default()
        },
        ChangeSource::Ui,
    )
    .await?;
    if deep {
        issues::update(
            db,
            made.id,
            IssuePatch {
                depth: Some(Some(Depth::Deep)),
                ..IssuePatch::default()
            },
            ChangeSource::Ui,
        )
        .await?;
    }
    Ok(made.id)
}

#[tokio::test]
async fn a_fresh_workspace_has_nothing_to_do() -> R {
    let db = Db::open_in_memory().await?;
    let today = day("2026-11-27");
    let s = shutdown::state(&db, today).await?;
    assert_eq!(
        s.inbox_count,
        i64::try_from(inbox::list(&db, today).await?.len())?
    );
    assert_eq!(s.deep.len(), 0);
    assert_eq!(s.tomorrow.len(), 0);
    assert!(!s.done_today);
    assert_eq!(s.streak, 0);
    Ok(())
}

#[tokio::test]
async fn tomorrow_takes_up_to_three_open_deep_issues() -> R {
    let db = Db::open_in_memory().await?;
    let today = day("2026-11-27");
    let spec = issue(&db, "Spec", true).await?;
    let draft = issue(&db, "Draft", true).await?;
    let sketch = issue(&db, "Sketch", true).await?;
    let review = issue(&db, "Review", true).await?;
    let shallow = issue(&db, "Email", false).await?;

    let s = shutdown::state(&db, today).await?;
    assert_eq!(s.deep.len(), 4);

    let err = shutdown::set_tomorrow(&db, today, &[spec, draft, sketch, review]).await;
    assert!(
        err.is_err(),
        "four is more than the three the ritual allows"
    );
    let err = shutdown::set_tomorrow(&db, today, &[shallow]).await;
    assert!(err.is_err(), "only deep issues go on tomorrow");
    let err = shutdown::set_tomorrow(&db, today, &[IssueId(999)]).await;
    assert!(err.is_err(), "an unknown issue is refused");

    let s = shutdown::set_tomorrow(&db, today, &[sketch, spec]).await?;
    let ids: Vec<IssueId> = s.tomorrow.iter().map(|i| i.id).collect();
    assert_eq!(ids, vec![sketch, spec]);

    // The plan is for the day after it was made; the next day starts empty.
    let next = shutdown::state(&db, day("2026-11-28")).await?;
    assert_eq!(next.tomorrow.len(), 0);
    Ok(())
}

#[tokio::test]
async fn finishing_records_the_day_once_and_counts_the_streak() -> R {
    let db = Db::open_in_memory().await?;
    let today = day("2026-11-27");
    history::step("Finish shutdown", async {
        shutdown::complete(&db, day("2026-11-25")).await?;
        shutdown::complete(&db, day("2026-11-26")).await?;
        Ok::<_, taskapp_core::Error>(())
    })
    .await?;
    // Until today is finished, the run that ends yesterday still shows.
    assert_eq!(shutdown::state(&db, today).await?.streak, 2);
    let s = history::step("Finish shutdown", async {
        shutdown::complete(&db, today).await
    })
    .await?;
    assert!(s.done_today);
    assert_eq!(s.streak, 3);
    let again = shutdown::complete(&db, today).await?;
    assert_eq!(again.streak, 3, "finishing twice in a day records it once");
    Ok(())
}

#[tokio::test]
async fn undo_takes_back_the_finish() -> R {
    let db = Db::open_in_memory().await?;
    let today = day("2026-11-27");
    history::step("Finish shutdown", async {
        shutdown::complete(&db, today).await
    })
    .await?;
    assert!(shutdown::state(&db, today).await?.done_today);
    undo::undo(&db).await?;
    assert!(!shutdown::state(&db, today).await?.done_today);
    assert!(
        settings::get::<Vec<NaiveDate>>(&db, "shutdown_days")
            .await?
            .is_none_or(|d| d.is_empty())
    );
    Ok(())
}

#[tokio::test]
async fn the_feature_is_off_until_switched_on() -> R {
    let db = Db::open_in_memory().await?;
    assert!(!settings::feature_on(&db, "feature_shutdown").await?);
    settings::set_feature(&db, "shutdown", true).await?;
    assert!(settings::feature_on(&db, "feature_shutdown").await?);
    Ok(())
}
