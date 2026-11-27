use serde_json::{Value, json};
use taskapp_core::{
    ChangeSource, Db, FieldRef, Filter, FixedField, IssuePatch, NewIssue, Op, ProjectStatus, Sort,
    ViewDef, ViewEntity, issues, labels, palette::Look, projects, undo, views,
};

type R = Result<(), Box<dyn std::error::Error>>;
const UI: ChangeSource = ChangeSource::Ui;

fn cond(f: FixedField, op: Op, value: Value) -> Filter {
    Filter::Cond {
        field: FieldRef::Fixed(f),
        op,
        value,
    }
}

async fn names(db: &Db, def: &ViewDef) -> Result<Vec<String>, taskapp_core::Error> {
    Ok(views::projects_query(db, def)
        .await?
        .into_iter()
        .map(|r| r.project.name)
        .collect())
}

/// Alpha (status 2, high, target Jan, bug label, 1 of 2 done), Beta (status 1, no target, no issues),
/// Gamma (status 4, low, target Feb, archived).
async fn world(db: &Db) -> Result<Vec<ProjectStatus>, Box<dyn std::error::Error>> {
    let st = taskapp_core::project_statuses::list(db).await?;
    let bug = labels::create(db, "bug", "red", None, UI).await?;
    let mut a = projects::create(db, "alpha", "Alpha", None, None, UI).await?;
    a.status_id = st[1].id;
    a.priority = 2;
    a.target_date = chrono::NaiveDate::from_ymd_opt(2027, 1, 10);
    a.label_ids = vec![bug.id];
    projects::update(db, a.clone(), UI).await?;
    projects::create(db, "beta", "Beta", None, None, UI).await?;
    let mut g = projects::create(db, "gamma", "Gamma", None, None, UI).await?;
    g.status_id = st[3].id;
    g.priority = 4;
    g.target_date = chrono::NaiveDate::from_ymd_opt(2027, 2, 1);
    projects::update(db, g, UI).await?;
    sqlx::query("UPDATE projects SET archived_at = '2026-01-01T00:00:00Z' WHERE slug = 'gamma'")
        .execute(&db.pool)
        .await?;
    let statuses = taskapp_core::statuses::list(db).await?;
    let done = statuses
        .iter()
        .find(|s| s.category == taskapp_core::StatusCategory::Completed)
        .map(|s| s.id);
    for (title, status) in [("one", done), ("two", None)] {
        let i = issues::create(
            db,
            NewIssue {
                title: title.into(),
                project_id: Some(a.id),
                ..NewIssue::default()
            },
            UI,
        )
        .await?;
        if let Some(s) = status {
            let patch = IssuePatch {
                status_id: Some(s),
                ..IssuePatch::default()
            };
            issues::update(db, i.id, patch, UI).await?;
        }
    }
    Ok(st)
}

#[tokio::test]
async fn each_project_field_filters_and_archived_are_hidden() -> R {
    let db = Db::open_in_memory().await?;
    let st = world(&db).await?;
    assert_eq!(names(&db, &ViewDef::default()).await?, ["Alpha", "Beta"]);
    let all = ViewDef {
        include_archived: true,
        ..ViewDef::default()
    };
    assert_eq!(names(&db, &all).await?, ["Alpha", "Beta", "Gamma"]);
    let db = &db;
    let by = |f, op, v| ViewDef {
        filter: Some(cond(f, op, v)),
        ..ViewDef::default()
    };
    let d = |f, op, v| async move { names(db, &by(f, op, v)).await };
    assert_eq!(
        d(FixedField::Status, Op::In, json!([st[1].id])).await?,
        ["Alpha"]
    );
    assert_eq!(
        d(FixedField::StatusCategory, Op::Eq, json!("planned")).await?,
        ["Beta"]
    );
    assert_eq!(d(FixedField::Priority, Op::Eq, json!(2)).await?, ["Alpha"]);
    assert_eq!(
        d(FixedField::Title, Op::Contains, json!("bet")).await?,
        ["Beta"]
    );
    assert_eq!(
        d(FixedField::TargetDate, Op::IsEmpty, json!(null)).await?,
        ["Beta"]
    );
    assert_eq!(
        d(FixedField::TargetDate, Op::Lt, json!("2027-02-01")).await?,
        ["Alpha"]
    );
    assert_eq!(
        d(FixedField::IssueCount, Op::Eq, json!(2)).await?,
        ["Alpha"]
    );
    assert_eq!(
        d(FixedField::Progress, Op::Gte, json!(50)).await?,
        ["Alpha"]
    );
    assert_eq!(
        d(FixedField::Label, Op::IsNotEmpty, json!(null)).await?,
        ["Alpha"]
    );
    assert_eq!(
        d(FixedField::Archived, Op::Eq, json!(true)).await?,
        Vec::<String>::new()
    );
    // A field that is only an issue's is refused, not ignored.
    assert!(
        names(db, &by(FixedField::Cycle, Op::IsEmpty, json!(null)))
            .await
            .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn sorts_by_target_date_and_rows_carry_their_progress() -> R {
    let db = Db::open_in_memory().await?;
    world(&db).await?;
    let sorted = |descending| ViewDef {
        sorts: vec![Sort {
            field: FieldRef::Fixed(FixedField::TargetDate),
            descending,
        }],
        ..ViewDef::default()
    };
    // No target date sorts last either way.
    assert_eq!(names(&db, &sorted(false)).await?, ["Alpha", "Beta"]);
    assert_eq!(names(&db, &sorted(true)).await?, ["Alpha", "Beta"]);
    let rows = views::projects_query(&db, &ViewDef::default()).await?;
    assert_eq!((rows[0].issue_done, rows[0].issue_total), (1, 2));
    assert_eq!((rows[1].issue_done, rows[1].issue_total), (0, 0));
    Ok(())
}

#[tokio::test]
async fn an_issue_view_filters_by_its_projects_status() -> R {
    let db = Db::open_in_memory().await?;
    let st = world(&db).await?;
    let def = ViewDef {
        filter: Some(cond(FixedField::ProjectStatus, Op::In, json!([st[1].id]))),
        ..ViewDef::default()
    };
    assert_eq!(views::query(&db, &def).await?.len(), 2);
    let other = ViewDef {
        filter: Some(cond(
            FixedField::ProjectStatus,
            Op::NotIn,
            json!([st[1].id]),
        )),
        ..ViewDef::default()
    };
    assert_eq!(views::query(&db, &other).await?.len(), 0);
    Ok(())
}

#[tokio::test]
async fn a_view_has_an_entity_that_defaults_to_issue_and_undoes() -> R {
    let db = Db::open_in_memory().await?;
    let issue = views::save(&db, None, "Issues", ViewDef::default(), Look::default(), UI).await?;
    assert_eq!(issue.entity, ViewEntity::Issue);
    let proj = views::save_as(
        &db,
        None,
        ("P", ViewEntity::Project),
        ViewDef::default(),
        Look::default(),
        UI,
    )
    .await?;
    assert_eq!(proj.entity, ViewEntity::Project);
    // Saving again keeps the entity.
    let again = views::save(
        &db,
        Some(proj.id),
        "P2",
        ViewDef::default(),
        Look::default(),
        UI,
    )
    .await?;
    assert_eq!(again.entity, ViewEntity::Project);
    let made = taskapp_core::history::step("Saved view Q", async {
        views::save_as(
            &db,
            None,
            ("Q", ViewEntity::Project),
            ViewDef::default(),
            Look::default(),
            UI,
        )
        .await
    })
    .await?;
    undo::undo(&db).await?;
    assert!(views::list(&db).await?.iter().all(|v| v.id != made.id));
    undo::redo(&db).await?;
    let back = views::list(&db).await?;
    assert_eq!(
        back.iter().find(|v| v.id == made.id).map(|v| v.entity),
        Some(ViewEntity::Project)
    );
    Ok(())
}

#[tokio::test]
async fn projects_sort_and_filter_by_created_and_updated_time() -> R {
    let db = Db::open_in_memory().await?;
    world(&db).await?;
    let mut a = projects::list(&db).await?.remove(0);
    a.priority = 1;
    projects::update(&db, a, UI).await?;
    let sorted = |field, descending| ViewDef {
        sorts: vec![Sort {
            field: FieldRef::Fixed(field),
            descending,
        }],
        ..ViewDef::default()
    };
    // Alpha was made first but edited last (the archived Gamma is hidden).
    assert_eq!(
        names(&db, &sorted(FixedField::CreatedAt, true)).await?,
        ["Beta", "Alpha"]
    );
    assert_eq!(
        names(&db, &sorted(FixedField::UpdatedAt, true)).await?,
        ["Alpha", "Beta"]
    );
    let after = |f| ViewDef {
        filter: Some(cond(f, Op::Gt, json!("2000-01-01"))),
        ..ViewDef::default()
    };
    assert_eq!(names(&db, &after(FixedField::UpdatedAt)).await?.len(), 2);
    // Any write moves updated_at; an undo puts the old time back.
    let mut p = projects::list(&db).await?.remove(1);
    let (created, before) = (p.created_at, p.updated_at);
    p.name = "Renamed".into();
    taskapp_core::history::step("s", projects::update(&db, p, UI)).await?;
    let now = projects::list(&db).await?.remove(1);
    assert_eq!((now.created_at, now.updated_at > before), (created, true));
    undo::undo(&db).await?;
    assert_eq!(projects::list(&db).await?.remove(1).updated_at, before);
    Ok(())
}
