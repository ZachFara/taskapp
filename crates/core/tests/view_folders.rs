use taskapp_core::history::step;
use taskapp_core::{
    ChangeSource, Db, Error, FolderSection as S, ViewDef, ViewFolderId, cycles, palette::Look,
    pins, projects, undo, view_folders as vf, views,
};

type R = Result<(), Box<dyn std::error::Error>>;
const UI: ChangeSource = ChangeSource::Ui;

async fn view(db: &Db, name: &str) -> Result<taskapp_core::SavedView, Error> {
    views::save(db, None, name, ViewDef::default(), Look::default(), UI).await
}

async fn folder(
    db: &Db,
    s: S,
    parent: Option<ViewFolderId>,
    name: &str,
) -> Result<ViewFolderId, Error> {
    Ok(step("s", vf::create(db, s, parent, name, UI)).await?.id)
}

async fn spot(db: &Db, s: S, id: i64) -> Result<(Option<ViewFolderId>, i32), Error> {
    let p = vf::placements(db).await?;
    let p = p
        .iter()
        .find(|p| p.section == s && p.id == id)
        .ok_or(Error::NotFound("item".into()))?;
    Ok((p.folder_id, p.position))
}

#[tokio::test]
async fn folders_hold_views_and_deleting_one_frees_them() -> R {
    let db = Db::open_in_memory().await?;
    let f = step("s", vf::create(&db, S::View, None, " Work ", UI)).await?;
    assert_eq!(f.name, "Work");
    let v = view(&db, "A").await?;
    step(
        "s",
        vf::place_item(&db, S::View, v.id.0, Some(f.id), None, UI),
    )
    .await?;
    assert_eq!(spot(&db, S::View, v.id.0).await?.0, Some(f.id));
    assert_eq!(vf::rename(&db, f.id, "Home", UI).await?.name, "Home");
    step("s", vf::delete(&db, f.id, UI)).await?;
    assert_eq!(vf::list(&db).await?.len(), 0);
    assert_eq!(spot(&db, S::View, v.id.0).await?.0, None);
    // undo brings back the folder and the view's place in it
    undo::undo(&db).await?;
    assert_eq!(vf::list(&db).await?.len(), 1);
    assert_eq!(spot(&db, S::View, v.id.0).await?.0, Some(f.id));
    Ok(())
}

#[tokio::test]
async fn every_section_takes_its_own_items_only() -> R {
    let db = Db::open_in_memory().await?;
    let p = projects::create(&db, "alpha", "Alpha", None, None, UI).await?;
    let c = cycles::create(
        &db,
        None,
        "2026-01-01".parse()?,
        "2026-01-14".parse()?,
        Look::default(),
        UI,
    )
    .await?;
    let pin = pins::pin(&db, taskapp_core::PinKind::Url, "https://example.com", None).await?;
    for (s, id) in [(S::Project, p.id.0), (S::Cycle, c.id.0), (S::Pin, pin.id.0)] {
        let f = folder(&db, s, None, "F").await?;
        step("s", vf::place_item(&db, s, id, Some(f), None, UI)).await?;
        assert_eq!(spot(&db, s, id).await?.0, Some(f));
        // a folder of another section refuses it
        let other = folder(&db, S::View, None, "V").await?;
        assert!(matches!(
            vf::place_item(&db, s, id, Some(other), None, UI).await,
            Err(Error::Invalid(_))
        ));
        // and deleting the folder frees the item
        step("s", vf::delete(&db, f, UI)).await?;
        assert_eq!(spot(&db, s, id).await?.0, None);
    }
    Ok(())
}

#[tokio::test]
async fn nesting_stops_at_three_levels_and_refuses_cycles() -> R {
    let db = Db::open_in_memory().await?;
    let a = folder(&db, S::View, None, "A").await?;
    let b = folder(&db, S::View, Some(a), "B").await?;
    let c = folder(&db, S::View, Some(b), "C").await?;
    assert!(matches!(
        vf::create(&db, S::View, Some(c), "D", UI).await,
        Err(Error::Invalid(_))
    ));
    let loose = folder(&db, S::View, None, "Loose").await?;
    let kid = folder(&db, S::View, Some(loose), "Kid").await?;
    // Loose > Kid is two deep: under C it would be five
    assert!(
        vf::place_folder(&db, loose, Some(c), None, UI)
            .await
            .is_err()
    );
    // Kid alone fits under B (A > B > Kid)
    step("s", vf::place_folder(&db, kid, Some(b), None, UI)).await?;
    // a folder cannot go inside itself or its own descendant
    assert!(vf::place_folder(&db, a, Some(a), None, UI).await.is_err());
    assert!(vf::place_folder(&db, a, Some(c), None, UI).await.is_err());
    // folders of another section are no parent
    assert!(vf::create(&db, S::Project, Some(a), "X", UI).await.is_err());
    // items go into the deepest folder
    let v = view(&db, "V").await?;
    step("s", vf::place_item(&db, S::View, v.id.0, Some(c), None, UI)).await?;
    // deleting a middle folder moves its folders and items up one level
    step("s", vf::delete(&db, c, UI)).await?;
    assert_eq!(spot(&db, S::View, v.id.0).await?.0, Some(b));
    step("s", vf::delete(&db, b, UI)).await?;
    let left = vf::list(&db).await?;
    assert_eq!(
        left.iter().find(|f| f.id == kid).map(|f| f.parent_id),
        Some(Some(a))
    );
    assert_eq!(spot(&db, S::View, v.id.0).await?.0, Some(a));
    Ok(())
}

#[tokio::test]
async fn placing_orders_siblings_and_every_write_undoes() -> R {
    let db = Db::open_in_memory().await?;
    assert!(matches!(
        vf::create(&db, S::View, None, "  ", UI).await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        vf::rename(&db, ViewFolderId(9), "x", UI).await,
        Err(Error::NotFound(_))
    ));
    assert!(matches!(
        vf::delete(&db, ViewFolderId(9), UI).await,
        Err(Error::NotFound(_))
    ));
    let a = folder(&db, S::View, None, "A").await?;
    let b = folder(&db, S::View, None, "B").await?;
    step("s", vf::place_folder(&db, b, None, Some(0), UI)).await?;
    assert_eq!(vf::list(&db).await?[0].id, b);
    let (vx, vy, vz) = (
        view(&db, "X").await?.id.0,
        view(&db, "Y").await?.id.0,
        view(&db, "Z").await?.id.0,
    );
    assert!(
        vf::place_item(&db, S::View, vx, Some(ViewFolderId(99)), None, UI)
            .await
            .is_err()
    );
    assert!(matches!(
        vf::place_item(&db, S::View, 999, None, None, UI).await,
        Err(Error::NotFound(_))
    ));
    step("s", vf::place_item(&db, S::View, vz, Some(a), None, UI)).await?;
    step("s", vf::place_item(&db, S::View, vy, Some(a), Some(0), UI)).await?;
    step("s", vf::place_item(&db, S::View, vx, Some(a), Some(1), UI)).await?;
    let mut inside: Vec<_> = vf::placements(&db)
        .await?
        .into_iter()
        .filter(|p| p.folder_id == Some(a))
        .collect();
    inside.sort_by_key(|p| p.position);
    assert_eq!(
        inside.iter().map(|p| p.id).collect::<Vec<_>>(),
        [vy, vx, vz]
    );
    for _ in 0..6 {
        undo::undo(&db).await?; // three item moves, the folder move, then the two creates
    }
    assert_eq!(vf::list(&db).await?.len(), 0);
    assert_eq!(spot(&db, S::View, vx).await?.0, None);
    Ok(())
}
