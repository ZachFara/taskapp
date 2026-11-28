use chrono::{TimeZone, Utc};
use taskapp_core::{
    ChangeSource::Ui, Db, IssuePatch, NewIssue, StatusCategory, StatusId, issues, stale, statuses,
};

type R = Result<(), Box<dyn std::error::Error>>;

/// A backlog status that nudges after `days`, and an issue there whose timestamps are set `years` back.
async fn parked(db: &Db, days: i32) -> Result<StatusId, Box<dyn std::error::Error>> {
    let s = statuses::create(db, "Parked", StatusCategory::Backlog, "gray", None, Ui).await?;
    statuses::set_flow(db, s.id, None, None, Some(days), Ui).await?;
    Ok(s.id)
}

async fn old_issue(
    db: &Db,
    title: &str,
    status: StatusId,
    year: i32,
) -> Result<taskapp_core::Issue, Box<dyn std::error::Error>> {
    let at = Utc.with_ymd_and_hms(year, 1, 1, 0, 0, 0).unwrap();
    let i = issues::create(
        db,
        NewIssue {
            title: title.into(),
            status_id: Some(status),
            created_at: Some(at),
            ..NewIssue::default()
        },
        Ui,
    )
    .await?;
    sqlx::query("UPDATE issues SET updated_at = ? WHERE id = ?")
        .bind(at)
        .bind(i.id.0)
        .execute(&db.pool)
        .await?;
    Ok(i)
}

#[tokio::test]
async fn lists_issues_past_their_status_nudge_oldest_first() -> R {
    let db = Db::open_in_memory().await?;
    let p = parked(&db, 7).await?;
    let newer = old_issue(&db, "newer", p, 2024).await?;
    let oldest = old_issue(&db, "oldest", p, 2020).await?;
    let got = stale::list(&db).await?;
    let keys: Vec<_> = got.iter().map(|s| s.issue.key.as_str()).collect();
    assert_eq!(keys, [oldest.key.as_str(), newer.key.as_str()]);
    assert_eq!(got[0].status_name, "Parked");
    assert_eq!(got[0].nudge_days, 7);
    assert!(got[0].days > 365);
    Ok(())
}

#[tokio::test]
async fn a_recent_status_change_restarts_the_clock() -> R {
    let db = Db::open_in_memory().await?;
    let p = parked(&db, 7).await?;
    let elsewhere =
        statuses::create(&db, "Elsewhere", StatusCategory::Backlog, "gray", None, Ui).await?;
    let i = old_issue(&db, "moved", elsewhere.id, 2020).await?;
    // The move is stamped now, so the issue is fresh in Parked.
    issues::update(
        &db,
        i.id,
        IssuePatch {
            status_id: Some(p),
            ..IssuePatch::default()
        },
        Ui,
    )
    .await?;
    assert_eq!(stale::list(&db).await?, Vec::new());
    Ok(())
}

#[tokio::test]
async fn no_nudge_or_archived_never_lists() -> R {
    let db = Db::open_in_memory().await?;
    let never = statuses::create(&db, "Someday", StatusCategory::Backlog, "gray", None, Ui).await?;
    old_issue(&db, "no nudge", never.id, 2020).await?;
    let p = parked(&db, 7).await?;
    let gone = old_issue(&db, "archived", p, 2020).await?;
    issues::archive(&db, gone.id, Ui).await?;
    assert_eq!(stale::list(&db).await?, Vec::new());
    Ok(())
}
