use taskapp_core::history::step;
use taskapp_core::{
    ChangeSource, Db, Error, ViewDef, ViewFolderId, palette::Look, undo, view_folders, views,
};

type R = Result<(), Box<dyn std::error::Error>>;
const UI: ChangeSource = ChangeSource::Ui;

async fn view(db: &Db, name: &str) -> Result<taskapp_core::SavedView, Error> {
    views::save(db, None, name, ViewDef::default(), Look::default(), UI).await
}

#[tokio::test]
async fn folders_hold_views_and_deleting_one_frees_them() -> R {
    let db = Db::open_in_memory().await?;
    let f = step("s", view_folders::create(&db, " Work ", UI)).await?;
    assert_eq!(f.name, "Work");
    let v = view(&db, "A").await?;
    step("s", view_folders::move_view(&db, v.id, Some(f.id), UI)).await?;
    let moved = views::list(&db).await?;
    assert_eq!(
        moved.iter().find(|x| x.id == v.id).map(|x| x.folder_id),
        Some(Some(f.id))
    );
    assert_eq!(
        view_folders::rename(&db, f.id, "Home", UI).await?.name,
        "Home"
    );
    step("s", view_folders::delete(&db, f.id, UI)).await?;
    assert_eq!(view_folders::list(&db).await?.len(), 0);
    let after = views::list(&db).await?;
    assert!(after.iter().any(|x| x.id == v.id && x.folder_id.is_none()));
    // undo brings back the folder and the view's place in it
    undo::undo(&db).await?;
    assert_eq!(view_folders::list(&db).await?.len(), 1);
    let back = views::list(&db).await?;
    assert_eq!(
        back.iter().find(|x| x.id == v.id).map(|x| x.folder_id),
        Some(Some(f.id))
    );
    Ok(())
}

#[tokio::test]
async fn bad_input_is_refused_and_every_write_undoes() -> R {
    let db = Db::open_in_memory().await?;
    assert!(matches!(
        view_folders::create(&db, "  ", UI).await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        view_folders::rename(&db, ViewFolderId(9), "x", UI).await,
        Err(Error::NotFound(_))
    ));
    assert!(matches!(
        view_folders::delete(&db, ViewFolderId(9), UI).await,
        Err(Error::NotFound(_))
    ));
    let a = step("s", view_folders::create(&db, "A", UI)).await?;
    let b = step("s", view_folders::create(&db, "B", UI)).await?;
    assert!(view_folders::reorder(&db, &[a.id], UI).await.is_err());
    step("s", view_folders::reorder(&db, &[b.id, a.id], UI)).await?;
    assert_eq!(view_folders::list(&db).await?[0].id, b.id);
    let v = view(&db, "V").await?;
    assert!(
        view_folders::move_view(&db, v.id, Some(ViewFolderId(99)), UI)
            .await
            .is_err()
    );
    step("s", view_folders::move_view(&db, v.id, Some(a.id), UI)).await?;
    for _ in 0..4 {
        undo::undo(&db).await?; // move, reorder, then the two creates
    }
    assert_eq!(view_folders::list(&db).await?.len(), 0);
    Ok(())
}
