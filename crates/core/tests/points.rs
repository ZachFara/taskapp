use chrono::NaiveDate;
use serde_json::json;
use taskapp_core::{
    BulkPatch, ChangeSource, Db, Error, FieldRef, FixedField, Issue, IssueId, IssuePatch, NewIssue,
    Op, TemplateInput, ViewDef, bulk, history, import::csv, issues, quickadd, settings, templates,
    undo, views,
};

type R = Result<(), Box<dyn std::error::Error>>;
const UI: ChangeSource = ChangeSource::Ui;

fn new(title: &str, points: Option<f64>) -> NewIssue {
    NewIssue {
        title: title.into(),
        points,
        ..NewIssue::default()
    }
}

fn set(points: Option<f64>) -> IssuePatch {
    IssuePatch {
        points: Some(points),
        ..IssuePatch::default()
    }
}

async fn make(db: &Db, title: &str, points: Option<f64>) -> Result<Issue, Error> {
    issues::create(db, new(title, points), UI).await
}

#[tokio::test]
async fn points_are_stored_with_one_decimal_and_cleared_with_null() -> R {
    let db = Db::open_in_memory().await?;
    let a = make(&db, "ABC a", Some(0.5)).await?;
    assert_eq!(issues::get(&db, a.id).await?.points, Some(0.5));
    let b = issues::update(&db, a.id, set(Some(13.0)), UI).await?;
    assert_eq!(b.points, Some(13.0));
    let c = issues::update(&db, a.id, set(None), UI).await?;
    assert_eq!(c.points, None);
    // Absent in a patch leaves it alone.
    issues::update(&db, a.id, set(Some(2.0)), UI).await?;
    let d = issues::update(&db, a.id, IssuePatch::default(), UI).await?;
    assert_eq!(d.points, Some(2.0));
    Ok(())
}

#[tokio::test]
async fn bad_points_are_refused() -> R {
    let db = Db::open_in_memory().await?;
    for bad in [-1.0, 0.25, f64::NAN, f64::INFINITY] {
        let e = issues::create(&db, new("ABC x", Some(bad)), UI).await;
        assert!(matches!(e, Err(Error::Invalid(_))), "{bad}: {e:?}");
    }
    let a = make(&db, "ABC a", Some(1.0)).await?;
    let e = issues::update(&db, a.id, set(Some(-0.1)), UI).await;
    assert!(matches!(e, Err(Error::Invalid(_))), "{e:?}");
    assert_eq!(issues::get(&db, a.id).await?.points, Some(1.0));
    Ok(())
}

#[tokio::test]
async fn a_points_change_is_recorded_and_undone_and_redone() -> R {
    let db = Db::open_in_memory().await?;
    let a = make(&db, "ABC a", Some(3.0)).await?;
    history::step("Set points", async {
        issues::update(&db, a.id, set(Some(8.0)), UI).await
    })
    .await?;
    let rows = history::list_for(&db, "issue", &a.id.0.to_string()).await?;
    let last = rows.last().ok_or("no history")?;
    assert_eq!(
        (last.field.as_str(), &last.old_value, &last.new_value),
        ("points", &Some(json!(3.0)), &Some(json!(8.0)))
    );
    undo::undo(&db).await?;
    assert_eq!(issues::get(&db, a.id).await?.points, Some(3.0));
    undo::redo(&db).await?;
    assert_eq!(issues::get(&db, a.id).await?.points, Some(8.0));
    // Clearing is undone too.
    history::step("Clear points", async {
        issues::update(&db, a.id, set(None), UI).await
    })
    .await?;
    undo::undo(&db).await?;
    assert_eq!(issues::get(&db, a.id).await?.points, Some(8.0));
    Ok(())
}

#[tokio::test]
async fn bulk_points_are_one_undo_step() -> R {
    let db = Db::open_in_memory().await?;
    let ids: Vec<IssueId> = vec![
        make(&db, "ABC a", Some(1.0)).await?.id,
        make(&db, "ABC b", None).await?.id,
    ];
    let patch = BulkPatch {
        patch: set(Some(5.0)),
        ..BulkPatch::default()
    };
    history::step("Bulk", bulk::update(&db, &ids, patch, UI)).await?;
    for id in &ids {
        assert_eq!(issues::get(&db, *id).await?.points, Some(5.0));
    }
    undo::undo(&db).await?;
    assert_eq!(issues::get(&db, ids[0]).await?.points, Some(1.0));
    assert_eq!(issues::get(&db, ids[1]).await?.points, None);
    Ok(())
}

#[tokio::test]
async fn duplicate_keeps_points() -> R {
    let db = Db::open_in_memory().await?;
    let a = make(&db, "ABC a", Some(2.0)).await?;
    let copies = bulk::duplicate(&db, &[a.id], UI).await?;
    assert_eq!(copies[0].points, Some(2.0));
    Ok(())
}

#[tokio::test]
async fn views_filter_sort_and_group_by_points() -> R {
    let db = Db::open_in_memory().await?;
    for (t, p) in [("ABC a", Some(5.0)), ("ABC b", None), ("ABC c", Some(1.5))] {
        make(&db, t, p).await?;
    }
    let field = FieldRef::Fixed(FixedField::Points);
    let sorted = ViewDef {
        sorts: vec![taskapp_core::Sort {
            field: field.clone(),
            descending: false,
        }],
        ..ViewDef::default()
    };
    let titles: Vec<_> = views::query(&db, &sorted)
        .await?
        .into_iter()
        .map(|i| i.title)
        .collect();
    assert_eq!(titles, ["ABC c", "ABC a", "ABC b"], "nulls last");
    let big = ViewDef {
        filter: Some(taskapp_core::Filter::Cond {
            field: field.clone(),
            op: Op::Gte,
            value: json!(2),
        }),
        ..ViewDef::default()
    };
    assert_eq!(views::query(&db, &big).await?.len(), 1);
    let none = ViewDef {
        filter: Some(taskapp_core::Filter::Cond {
            field,
            op: Op::IsEmpty,
            value: json!(null),
        }),
        ..ViewDef::default()
    };
    assert_eq!(views::query(&db, &none).await?[0].title, "ABC b");
    Ok(())
}

#[test]
fn quick_add_reads_points() {
    let today = NaiveDate::from_ymd_opt(2026, 12, 1).unwrap_or_default();
    let p = |s: &str| quickadd::parse(s, today, "UTC");
    assert_eq!(p("fix it 3pt").points, Some(3.0));
    assert_eq!(p("fix it 3pt").title, "fix it");
    assert_eq!(p("0.5pt tidy").points, Some(0.5));
    assert_eq!(p("pt3 tidy").points, Some(3.0));
    assert_eq!(
        p("tidy 0.25pt").points,
        None,
        "two decimals stay in the title"
    );
    assert_eq!(p("tidy pt").points, None);
    assert_eq!(p("tidy ~2h p2").estimate_minutes, Some(120));
}

#[tokio::test]
async fn estimation_settings_default_and_are_validated() -> R {
    let db = Db::open_in_memory().await?;
    assert_eq!(
        settings::get::<String>(&db, "estimation").await?.as_deref(),
        Some("both")
    );
    assert_eq!(
        settings::get::<String>(&db, "point_scale")
            .await?
            .as_deref(),
        Some("fibonacci")
    );
    settings::set(&db, "estimation", &"none").await?;
    assert_eq!(
        settings::get::<String>(&db, "estimation").await?.as_deref(),
        Some("none")
    );
    // Hiding points hides nothing in the backend.
    let a = make(&db, "ABC a", Some(2.0)).await?;
    assert_eq!(issues::get(&db, a.id).await?.points, Some(2.0));
    for (k, v) in [("estimation", "everything"), ("point_scale", "primes")] {
        let e = settings::set(&db, k, &v).await;
        assert!(matches!(e, Err(Error::Invalid(_))), "{k}: {e:?}");
    }
    Ok(())
}

#[tokio::test]
async fn csv_points_column_maps_to_points() -> R {
    let db = Db::open_in_memory().await?;
    let plan = csv::plan(&db, "Title,Points,Estimate\nA,3,90\nB,0.5,\n", None).await?;
    assert_eq!(plan.skipped, []);
    assert_eq!(plan.issues[0].points, Some(3.0));
    assert_eq!(plan.issues[0].estimate_minutes, Some(90));
    assert_eq!(plan.issues[1].points, Some(0.5));
    let bad = csv::plan(&db, "Title,Points\nA,lots\n", None).await?;
    assert_eq!(bad.skipped.len(), 1);
    taskapp_core::import::apply(&db, &plan).await?;
    let a = issues::get_by_key(&db, "ABC-1").await?;
    assert_eq!(a.points, Some(3.0));
    Ok(())
}

#[tokio::test]
async fn templates_prefill_points() -> R {
    let db = Db::open_in_memory().await?;
    let input: TemplateInput = serde_json::from_value(json!({
        "name": "Spike", "slug": "spike", "icon": null, "color": null,
        "body": {"title_prefix": "", "fields": {"points": 3.0}, "sub_issues": []}
    }))?;
    let t = templates::create(&db, input, UI).await?;
    let done = templates::apply(&db, t.id, new("ABC spike", None), UI).await?;
    assert_eq!(done.issue.points, Some(3.0));
    let over = templates::apply(&db, t.id, new("ABC spike", Some(5.0)), UI).await?;
    assert_eq!(over.issue.points, Some(5.0));
    Ok(())
}
