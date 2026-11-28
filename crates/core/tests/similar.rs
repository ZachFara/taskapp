use taskapp_core::{ChangeSource::Ui, Db, IssueId, NewIssue, ProjectId, issues, search};

type R = Result<(), Box<dyn std::error::Error>>;

async fn ids(
    db: &Db,
    text: &str,
    project: Option<ProjectId>,
    limit: u32,
) -> Result<Vec<IssueId>, taskapp_core::Error> {
    let got = search::similar(db, text, project, limit).await?;
    Ok(got.iter().map(|i| i.id).collect())
}
const NONE: Vec<IssueId> = vec![];

async fn add(
    db: &Db,
    title: &str,
    description: &str,
) -> Result<taskapp_core::Issue, taskapp_core::Error> {
    let new = NewIssue {
        title: title.into(),
        description: Some(description.into()),
        ..NewIssue::default()
    };
    issues::create(db, new, Ui).await
}

#[tokio::test]
async fn finds_open_look_alikes_best_first() -> R {
    let db = Db::open_in_memory().await?;
    let close = add(&db, "Login crash on Safari", "").await?;
    let looser = add(&db, "Login crash on Chrome too", "").await?;
    add(&db, "Update the billing invoice template", "").await?;
    let got = search::similar(&db, "Fix login crash", None, 5).await?;
    let ids: Vec<_> = got.iter().map(|i| i.id).collect();
    assert_eq!(ids, [close.id, looser.id]);
    Ok(())
}

#[tokio::test]
async fn skips_done_and_archived_and_empty_text() -> R {
    let db = Db::open_in_memory().await?;
    let done = add(&db, "Login crash on Safari", "").await?;
    sqlx::query("UPDATE issues SET completed_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(done.id.0)
        .execute(&db.pool)
        .await?;
    let gone = add(&db, "Login crash on Safari", "").await?;
    issues::archive(&db, gone.id, Ui).await?;
    assert_eq!(ids(&db, "Login crash on Safari", None, 5).await?, NONE);
    assert_eq!(ids(&db, "  !! ", None, 5).await?, NONE);
    Ok(())
}

#[tokio::test]
async fn limit_and_project_filter() -> R {
    let db = Db::open_in_memory().await?;
    add(&db, "Login crash on Safari", "").await?;
    add(&db, "Login crash on Chrome", "").await?;
    assert_eq!(
        search::similar(&db, "login crash on safari", None, 1)
            .await?
            .len(),
        1
    );
    assert_eq!(ids(&db, "login crash", Some(ProjectId(99)), 5).await?, NONE);
    Ok(())
}
