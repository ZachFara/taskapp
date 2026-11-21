use serde_json::{Value, json};
use taskapp_core::{
    ChangeSource::Ui, Db, Error, FieldRef, Filter, IssueId, Op, PropertyDef, PropertyKind,
    SelectOption, ViewDef, history, properties,
};

type R = Result<(), Box<dyn std::error::Error>>;

fn def(key: &str, kind: PropertyKind) -> PropertyDef {
    PropertyDef {
        key: key.into(),
        name: key.into(),
        kind,
        options: vec![],
        project_id: None,
        position: 0,
    }
}

fn opt(v: &str) -> SelectOption {
    SelectOption {
        value: v.into(),
        color: "#fff".into(),
    }
}

/// Issues are inserted directly: the issue module is not what is under test.
async fn issue(db: &Db, project: Option<i64>, props: Value) -> IssueId {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO issues (number, title, status_id, project_id, properties, created_at, updated_at)
         VALUES ((SELECT coalesce(max(number), 0) + 1 FROM issues), 't', 1, ?, ?, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')
         RETURNING id",
    )
    .bind(project)
    .bind(props.to_string())
    .fetch_one(&db.pool)
    .await
    .expect("insert issue");
    IssueId(id)
}

async fn all_props(db: &Db) -> Vec<String> {
    sqlx::query_scalar("SELECT properties FROM issues ORDER BY id")
        .fetch_all(&db.pool)
        .await
        .expect("props")
}

async fn view(db: &Db, def: &ViewDef) {
    sqlx::query("INSERT INTO views (name, definition, position) VALUES ('v', ?, 99)")
        .bind(serde_json::to_string(def).expect("json"))
        .execute(&db.pool)
        .await
        .expect("insert view");
}

async fn last_view(db: &Db) -> ViewDef {
    let s: String = sqlx::query_scalar("SELECT definition FROM views ORDER BY id DESC LIMIT 1")
        .fetch_one(&db.pool)
        .await
        .expect("view");
    serde_json::from_str(&s).expect("view def")
}

#[tokio::test]
async fn invalid_key_rejected() -> R {
    let db = Db::open_in_memory().await?;
    for k in ["Area", "1x", "", "a-b", &"a".repeat(41)] {
        let e = properties::create_def(&db, def(k, PropertyKind::Text), Ui).await;
        assert!(matches!(e, Err(Error::Invalid(_))), "{k}");
    }
    properties::create_def(&db, def("area_1", PropertyKind::Text), Ui).await?;
    Ok(())
}

#[tokio::test]
async fn duplicate_key_is_conflict() -> R {
    let db = Db::open_in_memory().await?;
    properties::create_def(&db, def("area", PropertyKind::Text), Ui).await?;
    let e = properties::create_def(&db, def("area", PropertyKind::Number), Ui).await;
    assert!(matches!(e, Err(Error::Conflict(_))));
    assert_eq!(properties::list_defs(&db).await?.len(), 1);
    Ok(())
}

#[tokio::test]
async fn validate_each_kind() {
    use PropertyKind::*;
    let mut sel = def("k", Select);
    sel.options = vec![opt("a"), opt("b")];
    let cases: Vec<(PropertyKind, Value, Value)> = vec![
        (Text, json!("x"), json!(1)),
        (Number, json!(1.5), json!("1")),
        (Date, json!("2026-02-03"), json!("2026-2-3")),
        (Date, json!("2026-02-03"), json!("2026-02-30")),
        (Select, json!("a"), json!("z")),
        (MultiSelect, json!(["a", "b"]), json!(["a", "a"])),
        (MultiSelect, json!([]), json!(["z"])),
        (Checkbox, json!(true), json!("true")),
        (Url, json!("https://x.io/p"), json!("ftp://x.io")),
        (Url, json!("HTTP://x.io"), json!("https://")),
        (NoteLink, json!("notes/a.md"), json!("notes/a.txt")),
        (NoteLink, json!("obsidian://open?vault=v"), json!(3)),
    ];
    for (kind, good, bad) in cases {
        let mut d = def("k", kind);
        d.options = sel.options.clone();
        assert!(properties::validate(&d, &good).is_ok(), "{kind:?} {good}");
        assert!(
            matches!(properties::validate(&d, &bad), Err(Error::Invalid(_))),
            "{kind:?} {bad}"
        );
    }
}

#[tokio::test]
async fn select_value_not_in_options_rejected() -> R {
    let db = Db::open_in_memory().await?;
    let mut d = def("size", PropertyKind::Select);
    d.options = vec![opt("s"), opt("m")];
    properties::create_def(&db, d, Ui).await?;
    let i = issue(&db, None, json!({})).await;
    let e = properties::set_value(&db, i, "size", Some(json!("xl")), Ui).await;
    assert!(matches!(e, Err(Error::Invalid(_))));
    assert_eq!(all_props(&db).await, ["{}"]);
    Ok(())
}

#[tokio::test]
async fn set_value_records_history() -> R {
    let db = Db::open_in_memory().await?;
    properties::create_def(&db, def("area", PropertyKind::Text), Ui).await?;
    let i = issue(&db, None, json!({})).await;
    let got = properties::set_value(&db, i, "area", Some(json!("ops")), Ui).await?;
    assert_eq!(got.properties["area"], json!("ops"));
    assert_eq!(got.key, "ZF-1");
    properties::set_value(&db, i, "area", None, Ui).await?;
    let h = history::list_for(&db, "issue", &i.0.to_string()).await?;
    assert_eq!(h.len(), 2);
    assert_eq!(h[0].field, "properties.area");
    assert_eq!(
        (h[0].old_value.clone(), h[0].new_value.clone()),
        (None, Some(json!("ops")))
    );
    assert_eq!(h[1].new_value, None);
    let e = properties::set_value(&db, i, "nope", Some(json!(1)), Ui).await;
    assert!(matches!(e, Err(Error::NotFound(_))));
    Ok(())
}

#[tokio::test]
async fn rename_keeps_values() -> R {
    let db = Db::open_in_memory().await?;
    properties::create_def(&db, def("area", PropertyKind::Text), Ui).await?;
    let i = issue(&db, None, json!({"area": "ops"})).await;
    let d = properties::rename_def(&db, "area", "Area of work", Ui).await?;
    assert_eq!((d.key.as_str(), d.name.as_str()), ("area", "Area of work"));
    assert_eq!(all_props(&db).await, [r#"{"area":"ops"}"#]);
    assert_eq!(properties::list_defs(&db).await?[0].name, "Area of work");
    let _ = i;
    Ok(())
}

#[tokio::test]
async fn change_kind_text_to_number_converts() -> R {
    let db = Db::open_in_memory().await?;
    properties::create_def(&db, def("n", PropertyKind::Text), Ui).await?;
    issue(&db, None, json!({"n": "3"})).await;
    issue(&db, None, json!({"n": "2.5"})).await;
    let d = properties::change_kind(&db, "n", PropertyKind::Number, vec![], Ui).await?;
    assert_eq!(d.kind, PropertyKind::Number);
    assert_eq!(all_props(&db).await, [r#"{"n":3}"#, r#"{"n":2.5}"#]);
    Ok(())
}

#[tokio::test]
async fn change_kind_select_multiselect_and_text() -> R {
    let db = Db::open_in_memory().await?;
    let mut d = def("s", PropertyKind::Select);
    d.options = vec![opt("a"), opt("b")];
    properties::create_def(&db, d, Ui).await?;
    issue(&db, None, json!({"s": "a"})).await;
    let opts = vec![opt("a"), opt("b")];
    properties::change_kind(&db, "s", PropertyKind::MultiSelect, opts.clone(), Ui).await?;
    assert_eq!(all_props(&db).await, [r#"{"s":["a"]}"#]);
    issue(&db, None, json!({"s": []})).await;
    properties::change_kind(&db, "s", PropertyKind::Select, opts, Ui).await?;
    assert_eq!(all_props(&db).await, [r#"{"s":"a"}"#, "{}"]);
    properties::change_kind(&db, "s", PropertyKind::Text, vec![], Ui).await?;
    assert_eq!(all_props(&db).await[0], r#"{"s":"a"}"#);
    Ok(())
}

#[tokio::test]
async fn change_kind_refuses_and_lists_keys() -> R {
    let db = Db::open_in_memory().await?;
    properties::create_def(&db, def("n", PropertyKind::Text), Ui).await?;
    issue(&db, None, json!({"n": "3"})).await;
    issue(&db, None, json!({"n": "abc"})).await;
    let before = all_props(&db).await;
    let updated: Vec<String> = sqlx::query_scalar("SELECT updated_at FROM issues")
        .fetch_all(&db.pool)
        .await?;
    let e = properties::change_kind(&db, "n", PropertyKind::Number, vec![], Ui).await;
    let Err(Error::Conflict(m)) = e else {
        panic!("expected Conflict, got {e:?}")
    };
    assert!(m.contains("ZF-2") && !m.contains("ZF-1"), "{m}");
    assert_eq!(all_props(&db).await, before);
    let after: Vec<String> = sqlx::query_scalar("SELECT updated_at FROM issues")
        .fetch_all(&db.pool)
        .await?;
    assert_eq!(after, updated);
    assert_eq!(
        properties::list_defs(&db).await?[0].kind,
        PropertyKind::Text
    );
    Ok(())
}

#[tokio::test]
async fn change_kind_lists_at_most_20_keys() -> R {
    let db = Db::open_in_memory().await?;
    properties::create_def(&db, def("n", PropertyKind::Text), Ui).await?;
    for _ in 0..25 {
        issue(&db, None, json!({"n": "x"})).await;
    }
    let Err(Error::Conflict(m)) =
        properties::change_kind(&db, "n", PropertyKind::Number, vec![], Ui).await
    else {
        panic!("expected Conflict")
    };
    assert_eq!(m.matches("ZF-").count(), 20, "{m}");
    assert!(m.contains("5 more"));
    Ok(())
}

fn prop(k: &str) -> FieldRef {
    FieldRef::Property(k.into())
}

fn cond(k: &str) -> Filter {
    Filter::Cond {
        field: prop(k),
        op: Op::Eq,
        value: json!("x"),
    }
}

#[tokio::test]
async fn delete_def_removes_values_and_cleans_views() -> R {
    let db = Db::open_in_memory().await?;
    properties::create_def(&db, def("area", PropertyKind::Text), Ui).await?;
    properties::create_def(&db, def("keep", PropertyKind::Text), Ui).await?;
    issue(&db, None, json!({"area": "ops", "keep": "k"})).await;
    issue(&db, None, json!({})).await;
    let v = ViewDef {
        filter: Some(Filter::And(vec![
            cond("area"),
            Filter::Or(vec![cond("area"), cond("keep")]),
            Filter::Not(Box::new(cond("area"))),
        ])),
        sorts: vec![taskapp_core::Sort {
            field: prop("area"),
            descending: false,
        }],
        group_by: Some(prop("area")),
        layout: taskapp_core::Layout::List,
        columns: vec![prop("area"), prop("keep")],
        include_archived: false,
    };
    view(&db, &v).await;
    properties::delete_def(&db, "area", Ui).await?;
    assert_eq!(all_props(&db).await, [r#"{"keep":"k"}"#, "{}"]);
    let got = last_view(&db).await;
    assert_eq!(
        got.filter,
        Some(Filter::And(vec![Filter::Or(vec![cond("keep")])]))
    );
    assert!(got.sorts.is_empty() && got.group_by.is_none());
    assert_eq!(got.columns, vec![prop("keep")]);
    assert_eq!(properties::list_defs(&db).await?.len(), 1);
    assert!(matches!(
        properties::delete_def(&db, "area", Ui).await,
        Err(Error::NotFound(_))
    ));
    Ok(())
}

#[tokio::test]
async fn delete_def_single_condition_view_has_no_filter() -> R {
    let db = Db::open_in_memory().await?;
    properties::create_def(&db, def("area", PropertyKind::Text), Ui).await?;
    for f in [cond("area"), Filter::And(vec![cond("area")])] {
        view(
            &db,
            &ViewDef {
                filter: Some(f),
                sorts: vec![],
                group_by: None,
                layout: taskapp_core::Layout::List,
                columns: vec![],
                include_archived: false,
            },
        )
        .await;
    }
    properties::delete_def(&db, "area", Ui).await?;
    assert_eq!(last_view(&db).await.filter, None);
    Ok(())
}

#[tokio::test]
async fn project_scoped_def_rejects_other_project() -> R {
    let db = Db::open_in_memory().await?;
    for (id, slug) in [(1, "a"), (2, "b")] {
        sqlx::query("INSERT INTO projects (id, slug, name, created_at) VALUES (?, ?, ?, '2026-01-01T00:00:00Z')")
            .bind(id).bind(slug).bind(slug).execute(&db.pool).await?;
    }
    let mut d = def("area", PropertyKind::Text);
    d.project_id = Some(taskapp_core::ProjectId(1));
    properties::create_def(&db, d, Ui).await?;
    let mine = issue(&db, Some(1), json!({})).await;
    let other = issue(&db, Some(2), json!({})).await;
    let none = issue(&db, None, json!({})).await;
    properties::set_value(&db, mine, "area", Some(json!("x")), Ui).await?;
    for i in [other, none] {
        let e = properties::set_value(&db, i, "area", Some(json!("x")), Ui).await;
        assert!(matches!(e, Err(Error::Invalid(_))));
    }
    Ok(())
}

async fn log(db: &Db, entity: &str, id: &str) -> Vec<(String, Option<Value>, Option<Value>)> {
    history::list_for(db, entity, id)
        .await
        .expect("history")
        .into_iter()
        .map(|c| (c.field, c.old_value, c.new_value))
        .collect()
}

#[tokio::test]
async fn def_changes_write_history() -> R {
    let db = Db::open_in_memory().await?;
    properties::create_def(&db, def("n", PropertyKind::Text), Ui).await?;
    properties::rename_def(&db, "n", "Number", Ui).await?;
    properties::change_kind(&db, "n", PropertyKind::Number, vec![], Ui).await?;
    properties::delete_def(&db, "n", Ui).await?;
    let fields: Vec<String> = log(&db, "property_def", "n")
        .await
        .into_iter()
        .map(|c| c.0)
        .collect();
    assert_eq!(fields, ["created", "name", "kind", "deleted"]);
    Ok(())
}

#[tokio::test]
async fn change_kind_writes_one_issue_row_per_converted_issue() -> R {
    let db = Db::open_in_memory().await?;
    properties::create_def(&db, def("n", PropertyKind::Text), Ui).await?;
    let a = issue(&db, None, json!({"n": "3"})).await;
    let none = issue(&db, None, json!({})).await;
    properties::change_kind(&db, "n", PropertyKind::Number, vec![], Ui).await?;
    assert_eq!(
        log(&db, "issue", &a.0.to_string()).await,
        [("properties.n".into(), Some(json!("3")), Some(json!(3)))]
    );
    assert_eq!(log(&db, "issue", &none.0.to_string()).await.len(), 0);
    Ok(())
}

#[tokio::test]
async fn refused_change_kind_writes_no_history() -> R {
    let db = Db::open_in_memory().await?;
    properties::create_def(&db, def("n", PropertyKind::Text), Ui).await?;
    let a = issue(&db, None, json!({"n": "3"})).await;
    issue(&db, None, json!({"n": "abc"})).await;
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM changes")
        .fetch_one(&db.pool)
        .await?;
    assert!(
        properties::change_kind(&db, "n", PropertyKind::Number, vec![], Ui)
            .await
            .is_err()
    );
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM changes")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(before, after);
    assert_eq!(log(&db, "issue", &a.0.to_string()).await.len(), 0);
    Ok(())
}

#[tokio::test]
async fn delete_def_writes_a_row_per_issue_that_lost_a_value() -> R {
    let db = Db::open_in_memory().await?;
    properties::create_def(&db, def("n", PropertyKind::Text), Ui).await?;
    let a = issue(&db, None, json!({"n": "x"})).await;
    let b = issue(&db, None, json!({})).await;
    properties::delete_def(&db, "n", Ui).await?;
    assert_eq!(
        log(&db, "issue", &a.0.to_string()).await,
        [("properties.n".into(), Some(json!("x")), None)]
    );
    assert_eq!(log(&db, "issue", &b.0.to_string()).await.len(), 0);
    Ok(())
}

async fn keys_in_order(db: &Db) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(properties::list_defs(db)
        .await?
        .into_iter()
        .map(|d| d.key)
        .collect())
}

#[tokio::test]
async fn reorder_sets_positions() -> R {
    let db = Db::open_in_memory().await?;
    for k in ["a", "b", "c"] {
        properties::create_def(&db, def(k, PropertyKind::Text), Ui).await?;
    }
    properties::reorder(&db, &["c".into(), "a".into(), "b".into()]).await?;
    assert_eq!(keys_in_order(&db).await?, ["c", "a", "b"]);
    Ok(())
}

#[tokio::test]
async fn reorder_missing_or_duplicate_key_is_invalid_and_changes_nothing() -> R {
    let db = Db::open_in_memory().await?;
    for k in ["a", "b"] {
        properties::create_def(&db, def(k, PropertyKind::Text), Ui).await?;
    }
    for keys in [
        vec!["b".to_string()],
        vec!["b".into(), "b".into()],
        vec!["b".into(), "a".into(), "zz".into()],
    ] {
        let e = properties::reorder(&db, &keys).await;
        assert!(matches!(e, Err(Error::Invalid(_))), "{keys:?}: {e:?}");
    }
    assert_eq!(keys_in_order(&db).await?, ["a", "b"]);
    Ok(())
}

async fn two_projects(db: &Db) -> Result<(), sqlx::Error> {
    for (id, slug) in [(1, "a"), (2, "b")] {
        sqlx::query("INSERT INTO projects (id, slug, name, created_at) VALUES (?, ?, ?, '2026-01-01T00:00:00Z')")
            .bind(id).bind(slug).bind(slug).execute(&db.pool).await?;
    }
    Ok(())
}

#[tokio::test]
async fn set_scope_moves_and_clears_scope() -> R {
    let db = Db::open_in_memory().await?;
    two_projects(&db).await?;
    properties::create_def(&db, def("area", PropertyKind::Text), Ui).await?;
    issue(&db, Some(1), json!({"area": "x"})).await;
    let d = properties::set_scope(&db, "area", Some(taskapp_core::ProjectId(1)), Ui).await?;
    assert_eq!(d.project_id, Some(taskapp_core::ProjectId(1)));
    let d = properties::set_scope(&db, "area", None, Ui).await?;
    assert_eq!(d.project_id, None);
    assert_eq!(properties::list_defs(&db).await?[0].project_id, None);
    Ok(())
}

#[tokio::test]
async fn set_scope_refuses_while_values_sit_on_other_projects() -> R {
    let db = Db::open_in_memory().await?;
    two_projects(&db).await?;
    properties::create_def(&db, def("area", PropertyKind::Text), Ui).await?;
    issue(&db, Some(1), json!({"area": "x"})).await;
    issue(&db, Some(2), json!({"area": "y"})).await;
    issue(&db, None, json!({})).await;
    let e = properties::set_scope(&db, "area", Some(taskapp_core::ProjectId(1)), Ui).await;
    let Err(Error::Conflict(m)) = e else {
        panic!("expected Conflict, got {e:?}")
    };
    assert!(m.contains("ZF-2") && !m.contains("ZF-1"), "{m}");
    assert_eq!(properties::list_defs(&db).await?[0].project_id, None);
    let e = properties::set_scope(&db, "nope", None, Ui).await;
    assert!(matches!(e, Err(Error::NotFound(_))));
    Ok(())
}

#[tokio::test]
async fn set_value_waits_for_a_second_writer_instead_of_failing() -> R {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("t.db");
    let (a, b) = (Db::open(&path).await?, Db::open(&path).await?);
    properties::create_def(&a, def("note", PropertyKind::Text), Ui).await?;
    let mut ids = vec![];
    for _ in 0..60 {
        ids.push(issue(&a, None, json!({})).await);
    }
    let run = |db: Db, ids: Vec<IssueId>| async move {
        let mut failed = 0;
        for id in ids {
            let v = Some(json!("x"));
            failed += i32::from(properties::set_value(&db, id, "note", v, Ui).await.is_err());
        }
        failed
    };
    let (x, y) = tokio::join!(
        tokio::spawn(run(a.clone(), ids.clone())),
        tokio::spawn(run(b.clone(), ids))
    );
    assert_eq!((x?, y?), (0, 0));
    Ok(())
}
