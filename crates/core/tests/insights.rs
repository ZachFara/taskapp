use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use taskapp_core::{
    ChangeSource, Db, IssueId, IssuePatch, Measure, NewIssue, ProgressScope, ProgressSeries,
    ProjectId, StatusId, ViewDef, cycles, insights, issues, palette::Look, projects, settings,
    views,
};

type R = Result<(), Box<dyn std::error::Error>>;
const UI: ChangeSource = ChangeSource::Ui;
// Statuses from the migration: 1 backlog, 2 unstarted, 3 started, 4 completed, 5 canceled.
const TODO: i64 = 2;
const DOING: i64 = 3;
const DONE: i64 = 4;
const CANCELED: i64 = 5;

fn d(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 1, day).unwrap_or_default()
}

fn noon(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, day, 12, 0, 0).unwrap()
}

struct World {
    db: Db,
    project: ProjectId,
}

async fn world() -> Result<World, Box<dyn std::error::Error>> {
    let db = Db::open_in_memory().await?;
    settings::set(&db, "timezone", &"UTC").await?;
    let project = projects::create(&db, "alpha", "Alpha", None, None, UI)
        .await?
        .id;
    Ok(World { db, project })
}

impl World {
    async fn issue(
        &self,
        day: u32,
        points: Option<f64>,
        minutes: Option<u32>,
    ) -> Result<IssueId, Box<dyn std::error::Error>> {
        let i = issues::create(
            &self.db,
            NewIssue {
                title: format!("ABC issue {day}"),
                project_id: Some(self.project),
                status_id: Some(StatusId(TODO)),
                points,
                estimate_minutes: minutes,
                created_at: Some(noon(day)),
                ..NewIssue::default()
            },
            UI,
        )
        .await?;
        Ok(i.id)
    }

    /// Applies the patch, then dates the change rows it wrote.
    async fn edit(&self, id: IssueId, patch: IssuePatch, at: DateTime<Utc>) -> R {
        let before: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(id), 0) FROM changes")
            .fetch_one(&self.db.pool)
            .await?;
        issues::update(&self.db, id, patch, UI).await?;
        sqlx::query("UPDATE changes SET at = ? WHERE id > ?")
            .bind(at)
            .bind(before)
            .execute(&self.db.pool)
            .await?;
        Ok(())
    }

    async fn status(&self, id: IssueId, s: i64, at: DateTime<Utc>) -> R {
        self.edit(
            id,
            IssuePatch {
                status_id: Some(StatusId(s)),
                ..IssuePatch::default()
            },
            at,
        )
        .await
    }

    async fn points(&self, id: IssueId, p: Option<f64>, at: DateTime<Utc>) -> R {
        self.edit(
            id,
            IssuePatch {
                points: Some(p),
                ..IssuePatch::default()
            },
            at,
        )
        .await
    }

    /// An issue made on the 1st with `points`, in `cycle`.
    async fn in_cycle(
        &self,
        points: f64,
        cycle: Option<taskapp_core::CycleId>,
    ) -> Result<IssueId, Box<dyn std::error::Error>> {
        let i = issues::create(
            &self.db,
            NewIssue {
                title: format!("ABC cycle issue {points}"),
                project_id: Some(self.project),
                status_id: Some(StatusId(TODO)),
                points: Some(points),
                cycle_id: cycle,
                created_at: Some(noon(1)),
                ..NewIssue::default()
            },
            UI,
        )
        .await?;
        Ok(i.id)
    }

    async fn series(
        &self,
        m: Measure,
        from: u32,
        to: u32,
    ) -> Result<ProgressSeries, Box<dyn std::error::Error>> {
        Ok(insights::progress_on(
            &self.db,
            ProgressScope::Project(self.project),
            m,
            Some(d(from)),
            Some(d(to)),
            d(28),
        )
        .await?)
    }
}

fn rows(s: &ProgressSeries) -> Vec<(u32, f64, f64, f64)> {
    use chrono::Datelike;
    s.days
        .iter()
        .map(|x| (x.date.day(), x.scope, x.started, x.completed))
        .collect()
}

#[tokio::test]
async fn issue_created_mid_range_counts_from_its_day_and_points_changes_apply_from_theirs() -> R {
    let w = world().await?;
    let a = w.issue(1, Some(3.0), None).await?;
    let b = w.issue(3, Some(5.0), None).await?;
    w.status(a, DOING, noon(2)).await?;
    w.status(a, DONE, noon(4)).await?;
    w.points(b, Some(8.0), noon(5)).await?;
    w.status(b, DOING, noon(5)).await?;
    let s = w.series(Measure::Points, 1, 6).await?;
    assert_eq!((s.from, s.to, s.measure), (d(1), d(6), Measure::Points));
    assert_eq!(
        rows(&s),
        [
            (1, 3.0, 0.0, 0.0),
            (2, 3.0, 3.0, 0.0),
            (3, 8.0, 3.0, 0.0),
            (4, 8.0, 3.0, 3.0),
            (5, 11.0, 11.0, 3.0),
            (6, 11.0, 11.0, 3.0),
        ]
    );
    Ok(())
}

#[tokio::test]
async fn completed_then_reopened_drops_out_of_completed_but_stays_started() -> R {
    let w = world().await?;
    let a = w.issue(1, Some(2.0), None).await?;
    w.status(a, DONE, noon(2)).await?;
    w.status(a, DOING, noon(4)).await?;
    let s = w.series(Measure::Points, 1, 5).await?;
    assert_eq!(
        rows(&s),
        [
            (1, 2.0, 0.0, 0.0),
            (2, 2.0, 2.0, 2.0),
            (3, 2.0, 2.0, 2.0),
            (4, 2.0, 2.0, 0.0),
            (5, 2.0, 2.0, 0.0)
        ]
    );
    Ok(())
}

#[tokio::test]
async fn a_change_belongs_to_the_workspace_zones_day() -> R {
    let w = world().await?;
    settings::set(&w.db, "timezone", &"America/Los_Angeles").await?;
    let a = w.issue(1, Some(1.0), None).await?;
    // 05:00 UTC on the 3rd is 21:00 on the 2nd in Los Angeles (UTC-8 in January).
    w.status(a, DONE, Utc.with_ymd_and_hms(2026, 1, 3, 5, 0, 0).unwrap())
        .await?;
    let s = w.series(Measure::Count, 1, 3).await?;
    assert_eq!(
        rows(&s),
        [(1, 1.0, 0.0, 0.0), (2, 1.0, 1.0, 1.0), (3, 1.0, 1.0, 1.0)]
    );
    Ok(())
}

#[tokio::test]
async fn an_issue_created_just_after_local_midnight_utc_counts_on_the_local_day() -> R {
    let w = world().await?;
    settings::set(&w.db, "timezone", &"America/Los_Angeles").await?;
    issues::create(
        &w.db,
        NewIssue {
            title: "ABC late".into(),
            project_id: Some(w.project),
            created_at: Some(Utc.with_ymd_and_hms(2026, 1, 3, 3, 0, 0).unwrap()),
            ..NewIssue::default()
        },
        UI,
    )
    .await?;
    let s = w.series(Measure::Count, 1, 3).await?;
    assert_eq!(
        rows(&s),
        [(1, 0.0, 0.0, 0.0), (2, 1.0, 0.0, 0.0), (3, 1.0, 0.0, 0.0)]
    );
    Ok(())
}

#[tokio::test]
async fn empty_scope_gives_zero_days_over_the_requested_range() -> R {
    let w = world().await?;
    let s = w.series(Measure::Points, 1, 3).await?;
    assert_eq!(
        rows(&s),
        [(1, 0.0, 0.0, 0.0), (2, 0.0, 0.0, 0.0), (3, 0.0, 0.0, 0.0)]
    );
    let dflt = insights::progress_on(
        &w.db,
        ProgressScope::Project(w.project),
        Measure::Points,
        None,
        None,
        d(10),
    )
    .await?;
    assert_eq!((dflt.from, dflt.to, dflt.days.len()), (d(10), d(10), 1));
    let backwards = insights::progress_on(
        &w.db,
        ProgressScope::Project(w.project),
        Measure::Points,
        Some(d(5)),
        Some(d(1)),
        d(10),
    )
    .await;
    assert!(backwards.is_err());
    Ok(())
}

#[tokio::test]
async fn measure_switching_sums_points_count_or_minutes_and_canceled_leaves_scope() -> R {
    let w = world().await?;
    let a = w.issue(1, Some(3.0), Some(60)).await?;
    let b = w.issue(1, None, Some(30)).await?;
    let c = w.issue(1, Some(8.0), Some(480)).await?;
    w.status(a, DONE, noon(2)).await?;
    w.status(c, CANCELED, noon(2)).await?;
    let _ = b;
    let at = |s: &ProgressSeries| rows(s)[1];
    assert_eq!(
        at(&w.series(Measure::Points, 1, 2).await?),
        (2, 3.0, 3.0, 3.0)
    );
    assert_eq!(
        at(&w.series(Measure::Count, 1, 2).await?),
        (2, 2.0, 1.0, 1.0)
    );
    assert_eq!(
        at(&w.series(Measure::Time, 1, 2).await?),
        (2, 90.0, 60.0, 60.0)
    );
    assert_eq!(
        rows(&w.series(Measure::Count, 1, 2).await?)[0],
        (1, 3.0, 0.0, 0.0)
    );
    Ok(())
}

#[tokio::test]
async fn cycle_and_view_scopes() -> R {
    let w = world().await?;
    let cy = cycles::create(&w.db, None, d(1), d(7), Look::default(), UI).await?;
    let a = w.issue(1, Some(2.0), None).await?;
    w.issue(1, Some(4.0), None).await?;
    w.edit(
        a,
        IssuePatch {
            cycle_id: Some(Some(cy.id)),
            ..IssuePatch::default()
        },
        noon(1),
    )
    .await?;
    w.status(a, DONE, noon(3)).await?;
    let s = insights::progress_on(
        &w.db,
        ProgressScope::Cycle(cy.id),
        Measure::Points,
        None,
        None,
        d(10),
    )
    .await?;
    assert_eq!((s.from, s.to), (d(1), d(7)), "a cycle's own dates");
    assert_eq!(rows(&s)[3], (4, 2.0, 2.0, 2.0));
    let v = views::save(&w.db, None, "All", ViewDef::default(), Look::default(), UI).await?;
    let s = insights::progress_on(
        &w.db,
        ProgressScope::View(v.id),
        Measure::Points,
        None,
        None,
        d(20),
    )
    .await?;
    assert_eq!(
        (s.from, s.to, s.days.len()),
        (d(20) - chrono::Duration::days(29), d(20), 30),
        "last 30 days"
    );
    assert_eq!(s.days.last().map(|x| x.scope), Some(6.0));
    Ok(())
}

#[tokio::test]
async fn default_project_range_is_its_dates_else_first_to_last_activity() -> R {
    let w = world().await?;
    let a = w.issue(2, Some(1.0), None).await?;
    w.status(a, DONE, noon(5)).await?;
    let s = insights::progress_on(
        &w.db,
        ProgressScope::Project(w.project),
        Measure::Count,
        None,
        None,
        d(20),
    )
    .await?;
    assert_eq!((s.from, s.to), (d(2), d(5)));
    let mut p = projects::list(&w.db).await?.remove(0);
    (p.start_date, p.target_date) = (Some(d(1)), Some(d(9)));
    projects::update(&w.db, p, UI).await?;
    let s = insights::progress_on(
        &w.db,
        ProgressScope::Project(w.project),
        Measure::Count,
        None,
        None,
        d(20),
    )
    .await?;
    assert_eq!((s.from, s.to), (d(1), d(9)));
    Ok(())
}

#[tokio::test]
async fn velocity_per_week_when_there_are_no_cycles() -> R {
    let w = world().await?;
    let a = w.issue(1, Some(3.0), None).await?;
    let b = w.issue(1, Some(5.0), None).await?;
    // 2026-01-05 is a Monday: a finishes in the week of Jan 5, b in the week of Jan 12.
    w.status(a, DONE, noon(7)).await?;
    w.status(b, DONE, noon(14)).await?;
    let v = insights::velocity_on(
        &w.db,
        ProgressScope::Project(w.project),
        Measure::Points,
        3,
        d(16),
    )
    .await?;
    let got: Vec<_> = v
        .iter()
        .map(|p| (p.start, p.end, p.label.as_str(), p.completed))
        .collect();
    assert_eq!(
        got,
        [
            (
                d(5) - chrono::Duration::days(7),
                d(11) - chrono::Duration::days(7),
                "Week of Dec 29",
                0.0
            ),
            (d(5), d(11), "Week of Jan 5", 3.0),
            (d(12), d(18), "Week of Jan 12", 5.0),
        ]
    );
    Ok(())
}

#[tokio::test]
async fn velocity_per_cycle_when_cycles_exist() -> R {
    let w = world().await?;
    let c1 = cycles::create(&w.db, Some("One".into()), d(1), d(7), Look::default(), UI).await?;
    cycles::create(&w.db, None, d(8), d(14), Look::default(), UI).await?;
    cycles::create(&w.db, None, d(15), d(21), Look::default(), UI).await?;
    let a = w.issue(1, Some(3.0), None).await?;
    let b = w.issue(1, Some(5.0), None).await?;
    let _ = c1;
    w.status(a, DONE, noon(3)).await?;
    w.status(b, DONE, noon(9)).await?;
    // Today is in cycle 2; cycle 3 has not started and is left out.
    let v = insights::velocity_on(
        &w.db,
        ProgressScope::Project(w.project),
        Measure::Points,
        5,
        d(10),
    )
    .await?;
    let got: Vec<_> = v.iter().map(|p| (p.label.as_str(), p.completed)).collect();
    assert_eq!(got, [("One", 3.0), ("Cycle 2", 5.0)]);
    let last = insights::velocity_on(
        &w.db,
        ProgressScope::Project(w.project),
        Measure::Points,
        1,
        d(10),
    )
    .await?;
    assert_eq!(last.len(), 1);
    Ok(())
}

/// A cycle's burn-up counts work from the day it joined the cycle until the day it left, and leaves out work that
/// was already done when the cycle began.
#[tokio::test]
async fn cycle_series_follows_membership_and_ignores_work_done_before_the_cycle() -> R {
    let w = world().await?;
    let cy = cycles::create(&w.db, None, d(5), d(11), Look::default(), UI).await?;
    let doing = w.in_cycle(3.0, Some(cy.id)).await?; // started on the 6th, done on the 8th
    let joins = w.in_cycle(5.0, None).await?; // joins on the 7th
    let stays_done = w.in_cycle(2.0, Some(cy.id)).await?; // done on the 3rd: not this cycle's work
    let reopened = w.in_cycle(8.0, Some(cy.id)).await?; // done on the 3rd, reopened on the 6th
    let leaves = w.in_cycle(13.0, Some(cy.id)).await?; // leaves on the 9th
    w.status(doing, DOING, noon(6)).await?;
    w.status(doing, DONE, noon(8)).await?;
    let to_cycle = |cycle: Option<taskapp_core::CycleId>| IssuePatch {
        cycle_id: Some(cycle),
        ..IssuePatch::default()
    };
    w.edit(joins, to_cycle(Some(cy.id)), noon(7)).await?;
    w.status(stays_done, DONE, noon(3)).await?;
    w.status(reopened, DONE, noon(3)).await?;
    w.status(reopened, DOING, noon(6)).await?;
    w.edit(leaves, to_cycle(None), noon(9)).await?;
    let s = insights::progress_on(
        &w.db,
        ProgressScope::Cycle(cy.id),
        Measure::Points,
        None,
        None,
        d(10),
    )
    .await?;
    assert_eq!((s.from, s.to), (d(5), d(11)));
    assert_eq!(
        rows(&s),
        vec![
            (5, 16.0, 0.0, 0.0),
            (6, 24.0, 11.0, 0.0),
            (7, 29.0, 11.0, 0.0),
            (8, 29.0, 11.0, 3.0),
            (9, 16.0, 11.0, 3.0),
            (10, 16.0, 11.0, 3.0),
            (11, 16.0, 11.0, 3.0),
        ]
    );
    Ok(())
}
