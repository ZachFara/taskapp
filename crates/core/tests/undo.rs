use taskapp_core::{
    ChangeSource, Db, Error, Issue, IssuePatch, NewIssue, StatusCategory, StatusId, history,
    issues, labels, statuses, undo,
};

type R = Result<(), Box<dyn std::error::Error>>;
const UI: ChangeSource = ChangeSource::Ui;

fn new(title: &str) -> NewIssue {
    NewIssue {
        title: title.into(),
        ..NewIssue::default()
    }
}

fn title(t: &str) -> IssuePatch {
    IssuePatch {
        title: Some(t.into()),
        ..IssuePatch::default()
    }
}

/// A made-up issue created outside any step, so it does not count as an undoable step.
async fn seed(db: &Db, t: &str) -> Result<Issue, Error> {
    issues::create(db, new(t), UI).await
}

async fn edit(db: &Db, label: &str, i: &Issue, patch: IssuePatch) -> Result<(), Error> {
    history::step(label, async { issues::update(db, i.id, patch, UI).await }).await?;
    Ok(())
}

#[tokio::test]
async fn undo_reverts_one_edit_and_redo_reapplies_it() -> R {
    let db = Db::open_in_memory().await?;
    let a = seed(&db, "ABC one").await?;
    edit(&db, "Rename", &a, title("ABC renamed")).await?;
    assert_eq!(
        undo::undo_status(&db).await?.undo.as_deref(),
        Some("Rename")
    );
    undo::undo(&db).await?;
    assert_eq!(issues::get(&db, a.id).await?.title, "ABC one");
    let s = undo::undo_status(&db).await?;
    assert_eq!((s.undo, s.redo.as_deref()), (None, Some("Rename")));
    undo::redo(&db).await?;
    assert_eq!(issues::get(&db, a.id).await?.title, "ABC renamed");
    Ok(())
}

#[tokio::test]
async fn a_bulk_edit_is_one_step() -> R {
    let db = Db::open_in_memory().await?;
    let mut all = Vec::new();
    for t in ["ABC a", "ABC b", "ABC c"] {
        all.push(seed(&db, t).await?);
    }
    history::step("Set priority on 3 issues", async {
        for i in &all {
            let p = IssuePatch {
                priority: Some(2),
                ..IssuePatch::default()
            };
            issues::update(&db, i.id, p, UI).await?;
        }
        Ok(())
    })
    .await?;
    undo::undo(&db).await?;
    for i in &all {
        assert_eq!(issues::get(&db, i.id).await?.priority, 0);
    }
    assert!(undo::undo(&db).await.is_err(), "nothing else to undo");
    undo::redo(&db).await?;
    for i in &all {
        assert_eq!(issues::get(&db, i.id).await?.priority, 2);
    }
    Ok(())
}

#[tokio::test]
async fn undo_of_create_archives_and_redo_restores() -> R {
    let db = Db::open_in_memory().await?;
    let a = history::step("Create issue", async {
        issues::create(&db, new("ABC made"), UI).await
    })
    .await?;
    undo::undo(&db).await?;
    assert!(issues::get(&db, a.id).await?.archived_at.is_some());
    undo::redo(&db).await?;
    let back = issues::get(&db, a.id).await?;
    assert!(back.archived_at.is_none());
    assert_eq!((back.id, back.key), (a.id, a.key));
    Ok(())
}

#[tokio::test]
async fn undo_of_archive_restores_the_issue() -> R {
    let db = Db::open_in_memory().await?;
    let a = seed(&db, "ABC keep").await?;
    history::step("Archive issue", async {
        issues::archive(&db, a.id, UI).await
    })
    .await?;
    assert!(issues::get(&db, a.id).await?.archived_at.is_some());
    undo::undo(&db).await?;
    assert!(issues::get(&db, a.id).await?.archived_at.is_none());
    undo::redo(&db).await?;
    assert!(issues::get(&db, a.id).await?.archived_at.is_some());
    Ok(())
}

#[tokio::test]
async fn a_new_change_clears_the_redo_stack() -> R {
    let db = Db::open_in_memory().await?;
    let a = seed(&db, "ABC one").await?;
    edit(&db, "First", &a, title("ABC two")).await?;
    undo::undo(&db).await?;
    edit(&db, "Second", &a, title("ABC three")).await?;
    assert_eq!(undo::undo_status(&db).await?.redo, None);
    assert!(undo::redo(&db).await.is_err());
    Ok(())
}

#[tokio::test]
async fn undo_refuses_when_the_field_changed_since_and_applies_nothing() -> R {
    let db = Db::open_in_memory().await?;
    let a = seed(&db, "ABC one").await?;
    let both = IssuePatch {
        title: Some("ABC two".into()),
        priority: Some(3),
        ..IssuePatch::default()
    };
    edit(&db, "Edit", &a, both).await?;
    // Changed outside any step, as an import would.
    let p = IssuePatch {
        priority: Some(1),
        ..IssuePatch::default()
    };
    issues::update(&db, a.id, p, ChangeSource::System).await?;
    let err = undo::undo(&db).await.expect_err("conflict");
    assert!(matches!(err, Error::Conflict(_)), "got {err:?}");
    assert!(err.to_string().contains("priority"), "got {err}");
    let now = issues::get(&db, a.id).await?;
    assert_eq!((now.title.as_str(), now.priority), ("ABC two", 1));
    Ok(())
}

#[tokio::test]
async fn undo_and_redo_are_recorded_with_their_own_source_and_never_loop() -> R {
    let db = Db::open_in_memory().await?;
    let a = seed(&db, "ABC one").await?;
    edit(&db, "Rename", &a, title("ABC two")).await?;
    undo::undo(&db).await?;
    let sources: Vec<_> = history::list_for(&db, "issue", &a.id.0.to_string())
        .await?
        .into_iter()
        .map(|c| c.source)
        .collect();
    assert_eq!(sources.last(), Some(&ChangeSource::Undo));
    assert_eq!(undo::undo_status(&db).await?.undo, None);
    Ok(())
}

#[tokio::test]
async fn undo_of_a_label_create_removes_it() -> R {
    let db = Db::open_in_memory().await?;
    history::step("Create label", async {
        labels::create(&db, "ABC bug", "red", None, UI).await
    })
    .await?;
    undo::undo(&db).await?;
    assert_eq!(labels::list(&db).await?.len(), 0);
    Ok(())
}

#[tokio::test]
async fn undo_of_a_status_delete_moves_the_issues_back() -> R {
    let db = Db::open_in_memory().await?;
    let extra =
        statuses::create(&db, "ABC extra", StatusCategory::Started, "blue", None, UI).await?;
    let a = issues::create(
        &db,
        NewIssue {
            status_id: Some(extra.id),
            ..new("ABC one")
        },
        UI,
    )
    .await?;
    history::step("Delete status", async {
        statuses::delete(&db, extra.id, StatusId(1)).await
    })
    .await?;
    undo::undo(&db).await?;
    assert_eq!(issues::get(&db, a.id).await?.status_id, extra.id);
    Ok(())
}
