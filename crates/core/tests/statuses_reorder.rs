use taskapp_core::{ChangeSource, Db, Error, StatusCategory, StatusId, history, statuses, undo};

type R = Result<(), Box<dyn std::error::Error>>;
const UI: ChangeSource = ChangeSource::Ui;

async fn started(db: &Db) -> Result<Vec<StatusId>, Error> {
    let mut have = Vec::new();
    for name in ["ABC one", "ABC two", "ABC three"] {
        have.push(
            statuses::create(db, name, StatusCategory::Started, "blue", None, UI)
                .await?
                .id,
        );
    }
    Ok(have)
}

async fn order(db: &Db, c: StatusCategory) -> Result<Vec<StatusId>, Error> {
    Ok(statuses::list(db)
        .await?
        .into_iter()
        .filter(|s| s.category == c)
        .map(|s| s.id)
        .collect())
}

#[tokio::test]
async fn reorder_is_one_undo_step_and_undo_restores_the_old_order() -> R {
    let db = Db::open_in_memory().await?;
    let ids = started(&db).await?;
    let before = order(&db, StatusCategory::Started).await?;
    let others = statuses::list(&db)
        .await?
        .into_iter()
        .filter(|s| s.category != StatusCategory::Started)
        .map(|s| (s.id, s.position))
        .collect::<Vec<_>>();
    let want = vec![ids[2], ids[0], ids[1]];

    let mut all = want.clone();
    all.extend(before.iter().filter(|b| !ids.contains(b)));
    history::step("Reorder statuses", statuses::reorder(&db, &all, UI)).await?;

    assert_eq!(&order(&db, StatusCategory::Started).await?[..3], &want[..]);
    let after = statuses::list(&db).await?;
    for (id, position) in others {
        assert_eq!(
            after.iter().find(|s| s.id == id).map(|s| s.position),
            Some(position),
            "other category untouched"
        );
    }
    undo::undo(&db).await?;
    assert_eq!(order(&db, StatusCategory::Started).await?, before);
    undo::redo(&db).await?;
    assert_eq!(&order(&db, StatusCategory::Started).await?[..3], &want[..]);
    Ok(())
}

#[tokio::test]
async fn reorder_refuses_mixed_categories_duplicates_and_unknown_ids() -> R {
    let db = Db::open_in_memory().await?;
    let ids = started(&db).await?;
    let other = statuses::list(&db)
        .await?
        .into_iter()
        .find(|s| s.category == StatusCategory::Completed)
        .expect("completed")
        .id;
    for bad in [
        vec![ids[0], other],
        vec![ids[0], ids[0]],
        vec![ids[0], StatusId(9999)],
        vec![],
    ] {
        let err = statuses::reorder(&db, &bad, UI).await.expect_err("refused");
        assert!(
            matches!(err, Error::Invalid(_) | Error::NotFound(_)),
            "{bad:?}: {err}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn deleting_a_status_into_itself_is_refused() -> R {
    let db = Db::open_in_memory().await?;
    let id = started(&db).await?[0];
    let e = statuses::delete(&db, id, id).await;
    assert!(matches!(e, Err(Error::Invalid(_))), "{e:?}");
    assert!(order(&db, StatusCategory::Started).await?.contains(&id));
    Ok(())
}
