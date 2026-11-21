use taskapp_core::{ChangeSource, Db, Error, IssueId, Relation, RelationKind, history, relations};

type R = Result<(), Box<dyn std::error::Error>>;

async fn db_with_issues(n: i64) -> Result<Db, Box<dyn std::error::Error>> {
    let db = Db::open_in_memory().await?;
    for i in 1..=n {
        sqlx::query(
            "INSERT INTO issues (id, number, title, status_id, created_at, updated_at) VALUES (?1, ?1, 't', 1, 'x', 'x')",
        )
        .bind(i)
        .execute(&db.pool)
        .await?;
    }
    Ok(db)
}

fn rel(from: i64, to: i64, kind: RelationKind) -> Relation {
    Relation {
        from: IssueId(from),
        to: IssueId(to),
        kind,
    }
}

fn blocks(from: i64, to: i64) -> Relation {
    rel(from, to, RelationKind::Blocks)
}

#[tokio::test]
async fn blocks_cycle_rejected() -> R {
    let db = db_with_issues(3).await?;
    relations::add(&db, blocks(1, 2), ChangeSource::Ui).await?;
    relations::add(&db, blocks(2, 3), ChangeSource::Ui).await?;
    let err = relations::add(&db, blocks(3, 1), ChangeSource::Ui).await;
    assert!(matches!(err, Err(Error::Conflict(m)) if m == "would create a dependency cycle"));
    assert_eq!(relations::all_blocking(&db).await?.len(), 2);
    Ok(())
}

#[tokio::test]
async fn self_relation_rejected() -> R {
    let db = db_with_issues(1).await?;
    for kind in [
        RelationKind::Blocks,
        RelationKind::Related,
        RelationKind::Duplicate,
    ] {
        let err = relations::add(&db, rel(1, 1, kind), ChangeSource::Ui).await;
        assert!(matches!(err, Err(Error::Invalid(_))));
    }
    Ok(())
}

#[tokio::test]
async fn duplicate_add_is_idempotent() -> R {
    let db = db_with_issues(2).await?;
    relations::add(&db, blocks(1, 2), ChangeSource::Ui).await?;
    relations::add(&db, blocks(1, 2), ChangeSource::Ui).await?;
    assert_eq!(relations::for_issue(&db, IssueId(1)).await?.len(), 1);
    assert_eq!(history::list_for(&db, "issue", "1").await?.len(), 1);
    Ok(())
}

#[tokio::test]
async fn unknown_issue_is_not_found() -> R {
    let db = db_with_issues(1).await?;
    let err = relations::add(&db, blocks(1, 9), ChangeSource::Ui).await;
    assert!(matches!(err, Err(Error::NotFound(_))));
    Ok(())
}

#[tokio::test]
async fn non_blocking_kinds_may_loop_and_stay_out_of_all_blocking() -> R {
    let db = db_with_issues(2).await?;
    relations::add(&db, rel(1, 2, RelationKind::Related), ChangeSource::Ui).await?;
    relations::add(&db, rel(2, 1, RelationKind::Related), ChangeSource::Ui).await?;
    assert_eq!(relations::all_blocking(&db).await?, vec![]);
    assert_eq!(relations::for_issue(&db, IssueId(1)).await?.len(), 2);
    Ok(())
}

#[tokio::test]
async fn remove_deletes_and_records_once() -> R {
    let db = db_with_issues(2).await?;
    relations::add(&db, blocks(1, 2), ChangeSource::Ui).await?;
    relations::remove(&db, blocks(1, 2), ChangeSource::Agent).await?;
    relations::remove(&db, blocks(1, 2), ChangeSource::Agent).await?;
    assert_eq!(relations::for_issue(&db, IssueId(2)).await?, vec![]);
    assert_eq!(history::list_for(&db, "issue", "1").await?.len(), 2);
    // With the edge gone the reverse edge is no longer a cycle.
    relations::add(&db, blocks(2, 1), ChangeSource::Ui).await?;
    Ok(())
}
