use taskapp_core::{
    ChangeSource, Db, Error, IssueId, Relation, RelationKind, RelationKindDef, history,
    relation_kinds, relations, undo,
};

type R = Result<(), Box<dyn std::error::Error>>;

async fn db_with_issues(n: i64) -> Result<Db, Box<dyn std::error::Error>> {
    let db = Db::open_in_memory().await?;
    for i in 1..=n {
        sqlx::query(
            "INSERT INTO issues (id, number, title, status_id, created_at, updated_at) VALUES (?1, ?1, 't', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
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
    rel(from, to, RelationKind::new("blocks"))
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
    for kind in ["blocks", "related", "duplicate"].map(RelationKind::new) {
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
    relations::add(
        &db,
        rel(1, 2, RelationKind::new("related")),
        ChangeSource::Ui,
    )
    .await?;
    relations::add(
        &db,
        rel(2, 1, RelationKind::new("related")),
        ChangeSource::Ui,
    )
    .await?;
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

#[tokio::test]
async fn six_built_in_kinds_are_seeded_and_old_names_still_work() -> R {
    let db = db_with_issues(2).await?;
    let kinds = relation_kinds::list(&db).await?;
    let ids: Vec<_> = kinds.iter().map(|k| k.id.0.as_str()).collect();
    assert_eq!(
        ids,
        [
            "blocks",
            "related",
            "duplicate",
            "follows",
            "caused_by",
            "part_of"
        ]
    );
    assert!(kinds.iter().all(|k| k.built_in));
    // The old wire spelling and a display name both resolve to the stored id.
    relations::add(
        &db,
        rel(1, 2, RelationKind::new("Blocks")),
        ChangeSource::Ui,
    )
    .await?;
    relations::add(
        &db,
        rel(2, 1, RelationKind::new("caused by")),
        ChangeSource::Ui,
    )
    .await?;
    let stored: Vec<_> = relations::for_issue(&db, IssueId(1))
        .await?
        .into_iter()
        .map(|r| r.kind.0)
        .collect();
    assert_eq!(stored, ["blocks", "caused_by"]);
    let err = relations::add(&db, rel(1, 2, RelationKind::new("nope")), ChangeSource::Ui).await;
    assert!(matches!(err, Err(Error::Invalid(_))));
    Ok(())
}

#[tokio::test]
async fn custom_kinds_create_rename_delete_and_undo() -> R {
    let db = db_with_issues(2).await?;
    let ui = ChangeSource::Ui;
    let k = history::step(
        "c",
        relation_kinds::create(&db, "Mentors", "mentors", "mentored by", false, ui),
    )
    .await?;
    assert_eq!(
        (k.id.0.as_str(), k.built_in, k.position),
        ("mentors", false, 6)
    );
    let dup = relation_kinds::create(&db, "mentors", "x", "y", false, ui).await;
    assert!(matches!(dup, Err(Error::Conflict(_))));
    history::step("r", relations::add(&db, rel(1, 2, k.id.clone()), ui)).await?;
    let edited = RelationKindDef {
        name: "Coaches".into(),
        forward: "coaches".into(),
        ..k.clone()
    };
    let renamed = history::step("e", relation_kinds::update(&db, edited, ui)).await?;
    assert_eq!(
        (renamed.id.clone(), renamed.name.as_str()),
        (k.id.clone(), "Coaches")
    );
    assert_eq!(relations::for_issue(&db, IssueId(1)).await?.len(), 1);
    // In use: refused. Built-in: refused.
    assert!(matches!(
        relation_kinds::delete(&db, &k.id, ui).await,
        Err(Error::Conflict(_))
    ));
    let blocks_id = RelationKind::new("blocks");
    assert!(matches!(
        relation_kinds::delete(&db, &blocks_id, ui).await,
        Err(Error::Invalid(_))
    ));
    history::step("u", relations::remove(&db, rel(1, 2, k.id.clone()), ui)).await?;
    history::step("d", relation_kinds::delete(&db, &k.id, ui)).await?;
    assert_eq!(relation_kinds::list(&db).await?.len(), 6);
    undo::undo(&db).await?; // delete
    assert_eq!(relation_kinds::list(&db).await?.len(), 7);
    undo::undo(&db).await?; // remove relation
    assert_eq!(relations::for_issue(&db, IssueId(1)).await?.len(), 1);
    undo::undo(&db).await?; // rename
    assert_eq!(relation_kinds::list(&db).await?[6].name, "Mentors");
    undo::undo(&db).await?; // add relation
    undo::undo(&db).await?; // create
    assert_eq!(relation_kinds::list(&db).await?.len(), 6);
    undo::redo(&db).await?;
    assert_eq!(relation_kinds::list(&db).await?.len(), 7);
    Ok(())
}

#[tokio::test]
async fn filter_has_relation_of_kind() -> R {
    use taskapp_core::{FieldRef, Filter, FixedField, Op, ViewDef, views};
    let db = db_with_issues(3).await?;
    relations::add(
        &db,
        rel(1, 2, RelationKind::new("follows")),
        ChangeSource::Ui,
    )
    .await?;
    let keys = |op, value: serde_json::Value| {
        let def = ViewDef {
            filter: Some(Filter::Cond {
                field: FieldRef::Fixed(FixedField::RelationKind),
                op,
                value,
            }),
            ..ViewDef::default()
        };
        let db = db.clone();
        async move {
            views::query(&db, &def)
                .await
                .map(|v| v.iter().map(|i| i.id.0).collect::<Vec<_>>())
        }
    };
    assert_eq!(
        keys(Op::In, serde_json::json!(["follows", "part_of"])).await?,
        [1, 2]
    );
    assert_eq!(
        keys(Op::Eq, serde_json::json!("blocks")).await?,
        Vec::<i64>::new()
    );
    assert_eq!(keys(Op::IsEmpty, serde_json::Value::Null).await?, [3]);
    Ok(())
}
