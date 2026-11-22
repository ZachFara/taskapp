use taskapp_core::{
    BulkPatch, ChangeSource, Db, Error, IssuePatch, NewIssue, RelationKind, bulk, history, issues,
    labels, relations, undo,
};

type R = Result<(), Box<dyn std::error::Error>>;
const UI: ChangeSource = ChangeSource::Ui;

async fn seed(db: &Db, t: &str) -> Result<taskapp_core::IssueId, Error> {
    let n = NewIssue {
        title: t.into(),
        ..NewIssue::default()
    };
    Ok(issues::create(db, n, UI).await?.id)
}

#[tokio::test]
async fn a_bulk_update_is_one_undo_step_for_every_kind_of_change() -> R {
    let db = Db::open_in_memory().await?;
    let (a, b) = (seed(&db, "ABC a").await?, seed(&db, "ABC b").await?);
    let keep = labels::create(&db, "keep", "blue", None, UI).await?.id;
    let drop = labels::create(&db, "drop", "red", None, UI).await?.id;
    labels::add_label(&db, b, drop, UI).await?;
    let patch = BulkPatch {
        patch: IssuePatch {
            priority: Some(2),
            ..IssuePatch::default()
        },
        add_labels: vec![keep],
        remove_labels: vec![drop],
        ..BulkPatch::default()
    };
    let ids = [a, b];
    history::step("Bulk", bulk::update(&db, &ids, patch, UI)).await?;
    for id in ids {
        let i = issues::get(&db, id).await?;
        assert_eq!((i.priority, i.label_ids), (2, vec![keep]));
    }
    undo::undo(&db).await?;
    assert_eq!(issues::get(&db, a).await?.priority, 0);
    assert_eq!(issues::get(&db, b).await?.label_ids, vec![drop]);
    assert!(undo::undo(&db).await.is_err(), "one step only");
    Ok(())
}

#[tokio::test]
async fn a_bulk_update_adds_a_relation_from_each_issue() -> R {
    let db = Db::open_in_memory().await?;
    let (a, b, c) = (
        seed(&db, "ABC a").await?,
        seed(&db, "ABC b").await?,
        seed(&db, "ABC c").await?,
    );
    let patch = BulkPatch {
        add_relation: Some((RelationKind::Related, c)),
        ..BulkPatch::default()
    };
    bulk::update(&db, &[a, b], patch, UI).await?;
    assert_eq!(relations::for_issue(&db, c).await?.len(), 2);
    Ok(())
}

#[tokio::test]
async fn a_failing_item_applies_nothing() -> R {
    let db = Db::open_in_memory().await?;
    let a = seed(&db, "ABC a").await?;
    let patch = BulkPatch {
        patch: IssuePatch {
            priority: Some(2),
            ..IssuePatch::default()
        },
        add_relation: Some((RelationKind::Related, a)),
        ..BulkPatch::default()
    };
    let b = seed(&db, "ABC b").await?;
    // a relating to itself is invalid, after b was already edited
    assert!(bulk::update(&db, &[b, a], patch, UI).await.is_err());
    assert_eq!(issues::get(&db, b).await?.priority, 0);
    Ok(())
}

#[tokio::test]
async fn a_bulk_archive_is_one_undo_step() -> R {
    let db = Db::open_in_memory().await?;
    let (a, b) = (seed(&db, "ABC a").await?, seed(&db, "ABC b").await?);
    let patch = BulkPatch {
        archive: Some(true),
        ..BulkPatch::default()
    };
    history::step("Archive", bulk::update(&db, &[a, b], patch, UI)).await?;
    assert!(issues::get(&db, a).await?.archived_at.is_some());
    undo::undo(&db).await?;
    for id in [a, b] {
        assert!(issues::get(&db, id).await?.archived_at.is_none());
    }
    assert!(undo::undo(&db).await.is_err(), "one step only");
    Ok(())
}

#[tokio::test]
async fn a_bulk_duplicate_copies_each_issue_in_one_undo_step() -> R {
    let db = Db::open_in_memory().await?;
    let label = labels::create(&db, "keep", "blue", None, UI).await?.id;
    let (a, b) = (seed(&db, "ABC a").await?, seed(&db, "ABC b").await?);
    labels::add_label(&db, a, label, UI).await?;
    let copies = history::step("Duplicate", bulk::duplicate(&db, &[a, b], UI)).await?;
    assert_eq!(copies.len(), 2);
    assert_eq!(copies[0].title, "ABC a");
    assert_eq!(copies[0].label_ids, vec![label]);
    assert_ne!(copies[0].id, a);
    undo::undo(&db).await?;
    for c in &copies {
        assert!(issues::get(&db, c.id).await?.archived_at.is_some());
    }
    assert!(undo::undo(&db).await.is_err(), "one step only");
    Ok(())
}

#[tokio::test]
async fn a_bulk_duplicate_with_an_unknown_issue_copies_nothing() -> R {
    let db = Db::open_in_memory().await?;
    let a = seed(&db, "ABC a").await?;
    assert!(
        bulk::duplicate(&db, &[a, taskapp_core::IssueId(999)], UI)
            .await
            .is_err()
    );
    assert!(
        issues::get(&db, taskapp_core::IssueId(2)).await.is_err(),
        "no copy was made"
    );
    Ok(())
}
