use taskapp_core::{
    ChangeSource::Ui, Db, Error, FieldRef, Filter, FixedField, IssuePatch, Op, label_groups,
    labels, undo, views,
};

type R = Result<(), Box<dyn std::error::Error>>;

async fn issue(db: &Db) -> Result<taskapp_core::IssueId, Box<dyn std::error::Error>> {
    let n = taskapp_core::NewIssue {
        title: "t".into(),
        ..taskapp_core::NewIssue::default()
    };
    Ok(taskapp_core::issues::create(db, n, Ui).await?.id)
}

#[tokio::test]
async fn group_holds_labels_and_survives_group_delete() -> R {
    let db = Db::open_in_memory().await?;
    let g = label_groups::create(&db, "Context", Ui).await?;
    assert!(!g.exclusive);
    let mut home = labels::create(&db, "@home", "blue", None, Ui).await?;
    assert_eq!(home.group_id, None);
    home.group_id = Some(g.id);
    labels::update(&db, home.clone(), Ui).await?;
    assert_eq!(labels::list(&db).await?[0].group_id, Some(g.id));
    assert!(label_groups::create(&db, "Context", Ui).await.is_err());
    taskapp_core::history::step("Delete", label_groups::delete(&db, g.id, Ui)).await?;
    assert_eq!(labels::list(&db).await?[0].group_id, None);
    undo::undo(&db).await?;
    assert_eq!(label_groups::list(&db).await?.len(), 1);
    Ok(())
}

#[tokio::test]
async fn exclusive_group_allows_one_label() -> R {
    let db = Db::open_in_memory().await?;
    let g = label_groups::create(&db, "Context", Ui).await?;
    label_groups::set_exclusive(&db, g.id, true, Ui).await?;
    let mut ids = vec![];
    for n in ["@home", "@phone"] {
        let mut l = labels::create(&db, n, "blue", None, Ui).await?;
        l.group_id = Some(g.id);
        ids.push(labels::update(&db, l, Ui).await?.id);
    }
    let i = issue(&db).await?;
    labels::add_label(&db, i, ids[0], Ui).await?;
    // Adding a second one swaps it in, and one undo puts the first back.
    taskapp_core::history::step("Add", labels::add_label(&db, i, ids[1], Ui)).await?;
    assert_eq!(
        taskapp_core::issues::get(&db, i).await?.label_ids,
        vec![ids[1]]
    );
    undo::undo(&db).await?;
    assert_eq!(
        taskapp_core::issues::get(&db, i).await?.label_ids,
        vec![ids[0]]
    );
    undo::redo(&db).await?;
    assert_eq!(
        taskapp_core::issues::get(&db, i).await?.label_ids,
        vec![ids[1]]
    );
    // A new issue with two is refused too.
    let new = taskapp_core::NewIssue {
        title: "n".into(),
        label_ids: Some(ids.clone()),
        ..taskapp_core::NewIssue::default()
    };
    assert!(taskapp_core::issues::create(&db, new, Ui).await.is_err());
    // A whole set with two is refused.
    let patch = IssuePatch {
        label_ids: Some(ids.clone()),
        ..IssuePatch::default()
    };
    let err = taskapp_core::issues::update(&db, i, patch, Ui)
        .await
        .expect_err("two labels of an exclusive group");
    assert!(matches!(err, Error::Invalid(_)), "{err:?}");
    Ok(())
}

#[tokio::test]
async fn filter_by_label_group() -> R {
    let db = Db::open_in_memory().await?;
    let g = label_groups::create(&db, "Context", Ui).await?;
    let mut l = labels::create(&db, "@home", "blue", None, Ui).await?;
    l.group_id = Some(g.id);
    let l = labels::update(&db, l, Ui).await?;
    let (a, _b) = (issue(&db).await?, issue(&db).await?);
    labels::add_label(&db, a, l.id, Ui).await?;
    let f = |op| Filter::Cond {
        field: FieldRef::Fixed(FixedField::LabelGroup),
        op,
        value: serde_json::json!(g.id),
    };
    let def = |op| taskapp_core::ViewDef {
        filter: Some(f(op)),
        ..taskapp_core::ViewDef::default()
    };
    let has = views::query(&db, &def(Op::Eq)).await?;
    assert_eq!(has.len(), 1);
    assert_eq!(has[0].id, a);
    assert_eq!(views::query(&db, &def(Op::Neq)).await?.len(), 1);
    Ok(())
}
