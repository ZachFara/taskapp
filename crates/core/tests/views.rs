use serde_json::{Value, json};
use taskapp_core::{
    Db, Error, FieldRef, Filter, FixedField, Issue, Layout, Op, PropertyDef, PropertyKind,
    SelectOption, Sort, ViewDef, search, views,
};

type R = Result<(), Box<dyn std::error::Error>>;

fn def(filter: Option<Filter>) -> ViewDef {
    ViewDef {
        filter,
        sorts: vec![],
        group_by: None,
        layout: Layout::List,
        columns: vec![],
        include_archived: false,
    }
}

fn pdef(key: &str, kind: PropertyKind) -> PropertyDef {
    PropertyDef {
        key: key.into(),
        name: key.into(),
        kind,
        options: Vec::<SelectOption>::new(),
        project_id: None,
        position: 0,
    }
}

fn cond(field: FieldRef, op: Op, value: Value) -> Filter {
    Filter::Cond { field, op, value }
}

fn prop(key: &str) -> FieldRef {
    FieldRef::Property(key.into())
}

async fn add(db: &Db, n: i64, title: &str, props: Value, status: i64, archived: bool) {
    let arch = archived.then_some("2026-01-01T00:00:00Z");
    sqlx::query(
        "INSERT INTO issues (id, number, title, status_id, properties, created_at, updated_at, archived_at)
         VALUES (?1, ?1, ?2, ?3, ?4, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', ?5)",
    )
    .bind(n)
    .bind(title)
    .bind(status)
    .bind(props.to_string())
    .bind(arch)
    .execute(&db.pool)
    .await
    .expect("insert issue");
}

async fn run(db: &Db, d: &ViewDef) -> Result<Vec<String>, Error> {
    let defs = vec![
        pdef("size", PropertyKind::Number),
        pdef("tags", PropertyKind::MultiSelect),
    ];
    sqlx::query("DELETE FROM property_defs")
        .execute(&db.pool)
        .await?;
    for p in &defs {
        let kind = serde_json::to_string(&p.kind).unwrap_or_default();
        sqlx::query("INSERT INTO property_defs (key, name, kind, position) VALUES (?, ?, ?, 0)")
            .bind(&p.key)
            .bind(&p.name)
            .bind(kind)
            .execute(&db.pool)
            .await?;
    }
    let issues: Vec<Issue> = views::query(db, d).await?;
    Ok(issues.into_iter().map(|i| i.title).collect())
}

async fn seeded() -> Result<Db, Box<dyn std::error::Error>> {
    let db = Db::open_in_memory().await?;
    add(
        &db,
        1,
        "a",
        json!({"size": 3, "tags": ["x", "y"]}),
        2,
        false,
    )
    .await;
    add(&db, 2, "b", json!({"size": 1, "tags": ["y"]}), 4, false).await;
    add(&db, 3, "c", json!({}), 3, false).await;
    add(&db, 4, "d", json!({"size": 2}), 2, true).await;
    Ok(db)
}

#[tokio::test]
async fn filter_fixed_and_property() -> R {
    let db = seeded().await?;
    let f = cond(
        FieldRef::Fixed(FixedField::StatusCategory),
        Op::In,
        json!(["started"]),
    );
    assert_eq!(run(&db, &def(Some(f))).await?, ["c"]);
    let f = cond(prop("size"), Op::Gte, json!(2));
    assert_eq!(run(&db, &def(Some(f))).await?, ["a"]);
    let f = cond(FieldRef::Fixed(FixedField::Title), Op::Eq, json!("b"));
    assert_eq!(run(&db, &def(Some(f))).await?, ["b"]);
    Ok(())
}

#[tokio::test]
async fn multiselect_contains() -> R {
    let db = seeded().await?;
    let f = cond(prop("tags"), Op::Contains, json!("y"));
    assert_eq!(run(&db, &def(Some(f))).await?, ["a", "b"]);
    let f = cond(prop("tags"), Op::Contains, json!("x"));
    assert_eq!(run(&db, &def(Some(f))).await?, ["a"]);
    Ok(())
}

#[tokio::test]
async fn nested_and_or_not() -> R {
    let db = seeded().await?;
    let small = cond(prop("size"), Op::Lt, json!(2));
    let y = cond(prop("tags"), Op::Contains, json!("y"));
    let f = Filter::And(vec![y.clone(), Filter::Not(Box::new(small.clone()))]);
    assert_eq!(run(&db, &def(Some(f))).await?, ["a"]);
    let f = Filter::Or(vec![small, cond(prop("size"), Op::IsEmpty, json!(null))]);
    assert_eq!(run(&db, &def(Some(f))).await?, ["b", "c"]);
    Ok(())
}

#[tokio::test]
async fn unknown_property_is_invalid() -> R {
    let f = cond(prop("nope"), Op::Eq, json!(1));
    let e = views::compile(&def(Some(f)), &[]).expect_err("must fail");
    assert!(matches!(e, Error::Invalid(_)));
    Ok(())
}

#[tokio::test]
async fn property_key_never_inlined() -> R {
    let defs = [pdef("size", PropertyKind::Number)];
    let mut d = def(Some(cond(prop("size"), Op::Eq, json!(1))));
    d.sorts = vec![Sort {
        field: prop("size"),
        descending: false,
    }];
    let (sql, binds) = views::compile(&d, &defs)?;
    assert!(!sql.contains("size"), "{sql}");
    assert!(
        binds
            .iter()
            .any(|b| b.as_str().is_some_and(|s| s.contains("size")))
    );
    Ok(())
}

#[tokio::test]
async fn hostile_property_key_is_invalid() -> R {
    let defs = [pdef("size", PropertyKind::Number)];
    let key = "') OR 1=1 --";
    let f = cond(prop(key), Op::Eq, json!(1));
    assert!(matches!(
        views::compile(&def(Some(f)), &defs),
        Err(Error::Invalid(_))
    ));
    let mut d = def(None);
    d.sorts = vec![Sort {
        field: prop(key),
        descending: false,
    }];
    assert!(matches!(views::compile(&d, &defs), Err(Error::Invalid(_))));
    Ok(())
}

#[tokio::test]
async fn in_with_empty_list_matches_nothing() -> R {
    let db = seeded().await?;
    let f = cond(FieldRef::Fixed(FixedField::Status), Op::In, json!([]));
    assert_eq!(run(&db, &def(Some(f))).await?.len(), 0);
    let f = cond(FieldRef::Fixed(FixedField::Status), Op::NotIn, json!([]));
    assert_eq!(run(&db, &def(Some(f))).await?.len(), 3);
    Ok(())
}

#[tokio::test]
async fn sort_property_nulls_last() -> R {
    let db = seeded().await?;
    let mut d = def(None);
    d.sorts = vec![Sort {
        field: prop("size"),
        descending: false,
    }];
    assert_eq!(run(&db, &d).await?, ["b", "a", "c"]);
    d.sorts[0].descending = true;
    assert_eq!(run(&db, &d).await?, ["a", "b", "c"]);
    Ok(())
}

#[tokio::test]
async fn archived_excluded_by_default() -> R {
    let db = seeded().await?;
    assert_eq!(run(&db, &def(None)).await?, ["a", "b", "c"]);
    let mut d = def(None);
    d.include_archived = true;
    assert_eq!(run(&db, &d).await?.len(), 4);
    Ok(())
}

#[tokio::test]
async fn saved_view_roundtrip() -> R {
    let db = Db::open_in_memory().await?;
    let seeded = views::list(&db).await?.len();
    let d = def(Some(cond(
        FieldRef::Fixed(FixedField::Priority),
        Op::Eq,
        json!(1),
    )));
    let v = views::save(&db, None, "Mine", d.clone()).await?;
    assert_eq!(views::list(&db).await?.len(), seeded + 1);
    let v2 = views::save(&db, Some(v.id), "Renamed", def(None)).await?;
    assert_eq!((v2.id, v2.name.as_str()), (v.id, "Renamed"));
    assert_eq!(v.def, d);
    views::delete(&db, v.id).await?;
    assert_eq!(views::list(&db).await?.len(), seeded);
    assert!(matches!(
        views::delete(&db, v.id).await,
        Err(Error::NotFound(_))
    ));
    Ok(())
}

#[tokio::test]
async fn search_finds_comment_text() -> R {
    let db = seeded().await?;
    sqlx::query(
        "INSERT INTO comments (issue_id, body, created_at, updated_at)
         VALUES (2, 'zebra crossing', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
    )
    .execute(&db.pool)
    .await?;
    for id in 1..=4 {
        search::reindex(&db, taskapp_core::IssueId(id)).await?;
    }
    let hits = search::search(&db, "zebra", 10).await?;
    assert_eq!(
        hits.iter().map(|i| i.title.as_str()).collect::<Vec<_>>(),
        ["b"]
    );
    // Reindexing again must not leave the old row behind.
    search::reindex(&db, taskapp_core::IssueId(2)).await?;
    assert_eq!(search::search(&db, "zebra", 10).await?.len(), 1);
    Ok(())
}

#[tokio::test]
async fn search_tolerates_punctuation() -> R {
    let db = seeded().await?;
    search::reindex(&db, taskapp_core::IssueId(1)).await?;
    search::search(&db, "ZF-1: fix 'x'", 10).await?;
    search::search(&db, "\"unbalanced AND (", 10).await?;
    assert_eq!(search::search(&db, "   ", 10).await?.len(), 0);
    assert_eq!(search::search(&db, "", 10).await?.len(), 0);
    Ok(())
}

#[tokio::test]
async fn reorder_writes_positions_and_list_follows() -> R {
    let db = Db::open_in_memory().await?;
    let mut ids: Vec<_> = views::list(&db).await?.into_iter().map(|v| v.id).collect();
    ids.reverse();
    views::reorder(&db, &ids).await?;
    let now: Vec<_> = views::list(&db).await?;
    assert_eq!(now.iter().map(|v| v.id).collect::<Vec<_>>(), ids);
    assert_eq!(
        now.iter().map(|v| v.position).collect::<Vec<_>>(),
        [0, 1, 2]
    );
    Ok(())
}

#[tokio::test]
async fn reorder_missing_or_duplicate_id_is_invalid() -> R {
    let db = Db::open_in_memory().await?;
    let before = views::list(&db).await?;
    let ids: Vec<_> = before.iter().map(|v| v.id).collect();
    for bad in [ids[..2].to_vec(), vec![ids[0], ids[0], ids[1]]] {
        let e = views::reorder(&db, &bad).await;
        assert!(matches!(e, Err(Error::Invalid(_))), "{e:?}");
    }
    assert_eq!(views::list(&db).await?, before);
    Ok(())
}
