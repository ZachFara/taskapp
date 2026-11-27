use chrono::{NaiveDate, TimeZone, Utc};
use serde_json::json;
use taskapp_core::{
    ChangeSource, Db, Error, Issue, IssuePatch, Layout, NewIssue, PinKind, PropertyDef,
    PropertyKind, Relation, RelationKind, StatusCategory, StatusId, ViewDef, blocks, comments,
    cycles, history,
    import::{ImportPlan, PlanIssue, apply},
    issues, labels,
    palette::Look,
    pins, projects, properties, relations, settings, statuses,
    templates::{self, TemplateBody, TemplateInput},
    undo, views,
};

type R = Result<(), Box<dyn std::error::Error>>;
const UI: ChangeSource = ChangeSource::Ui;

async fn seed(db: &Db, t: &str) -> Result<Issue, Error> {
    issues::create(
        db,
        NewIssue {
            title: t.into(),
            ..NewIssue::default()
        },
        UI,
    )
    .await
}

async fn n(db: &Db, sql: &str) -> i64 {
    sqlx::query_scalar(sql)
        .fetch_one(&db.pool)
        .await
        .expect("count")
}

fn date(d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 12, d).expect("date")
}

#[tokio::test]
async fn label_create_edit_delete_undo_and_redo() -> R {
    let db = Db::open_in_memory().await?;
    let a = seed(&db, "ABC one").await?;
    let l = history::step(
        "Create label",
        labels::create(&db, "ABC bug", "red", None, UI),
    )
    .await?;
    undo::undo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM labels").await, 0);
    undo::redo(&db).await?;
    assert_eq!(labels::list(&db).await?[0].id, l.id);

    let mut renamed = l.clone();
    renamed.name = "ABC defect".into();
    history::step("Edit label", labels::update(&db, renamed, UI)).await?;
    undo::undo(&db).await?;
    assert_eq!(labels::list(&db).await?[0].name, "ABC bug");
    undo::redo(&db).await?;
    assert_eq!(labels::list(&db).await?[0].name, "ABC defect");

    labels::add_label(&db, a.id, l.id, UI).await?;
    history::step("Delete label", labels::delete(&db, l.id, UI)).await?;
    assert_eq!(issues::get(&db, a.id).await?.label_ids.len(), 0);
    undo::undo(&db).await?;
    assert_eq!(labels::list(&db).await?[0].id, l.id);
    assert_eq!(issues::get(&db, a.id).await?.label_ids, vec![l.id]);
    undo::redo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM labels").await, 0);
    Ok(())
}

#[tokio::test]
async fn bulk_label_assign_over_three_issues_is_one_step() -> R {
    let db = Db::open_in_memory().await?;
    let l = labels::create(&db, "ABC bug", "red", None, UI).await?;
    let mut all = vec![];
    for t in ["ABC a", "ABC b", "ABC c"] {
        all.push(seed(&db, t).await?);
    }
    history::step("Label 3 issues", async {
        for i in &all {
            labels::add_label(&db, i.id, l.id, UI).await?;
        }
        Ok(())
    })
    .await?;
    undo::undo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM issue_labels").await, 0);
    undo::redo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM issue_labels").await, 3);
    Ok(())
}

#[tokio::test]
async fn comment_add_edit_delete() -> R {
    let db = Db::open_in_memory().await?;
    let a = seed(&db, "ABC one").await?;
    let c = history::step(
        "Comment",
        comments::add_comment(&db, a.id, "first", None, UI),
    )
    .await?;
    undo::undo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM comments").await, 0);
    undo::redo(&db).await?;
    history::step("Edit comment", comments::update(&db, c.id, "second", UI)).await?;
    undo::undo(&db).await?;
    assert_eq!(comments::list(&db, a.id).await?[0].body, "first");
    undo::redo(&db).await?;
    history::step("Delete comment", comments::delete(&db, c.id, UI)).await?;
    undo::undo(&db).await?;
    assert_eq!(comments::list(&db, a.id).await?[0].body, "second");
    undo::redo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM comments").await, 0);
    Ok(())
}

#[tokio::test]
async fn project_and_milestone_delete_restores_links() -> R {
    let db = Db::open_in_memory().await?;
    let p = history::step(
        "Create project",
        projects::create(&db, "abc-p", "ABC P", None, None, UI),
    )
    .await?;
    undo::undo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM projects").await, 0);
    undo::redo(&db).await?;
    let m = history::step(
        "Create milestone",
        projects::create_milestone(&db, p.id, "M1", Some(date(9)), UI),
    )
    .await?;
    let a = issues::create(
        &db,
        NewIssue {
            title: "ABC one".into(),
            project_id: Some(p.id),
            milestone_id: Some(m.id),
            ..NewIssue::default()
        },
        UI,
    )
    .await?;
    history::step("Delete project", projects::delete(&db, p.id, UI)).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM milestones").await, 0);
    undo::undo(&db).await?;
    let back = issues::get(&db, a.id).await?;
    assert_eq!(
        (back.project_id, back.milestone_id),
        (Some(p.id), Some(m.id))
    );
    assert_eq!(n(&db, "SELECT count(*) FROM milestones").await, 1);
    undo::redo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM projects").await, 0);
    undo::undo(&db).await?;

    let mut edited = p.clone();
    edited.name = "ABC renamed".into();
    history::step("Edit project", projects::update(&db, edited, UI)).await?;
    undo::undo(&db).await?;
    assert_eq!(projects::list(&db).await?[0].name, "ABC P");
    undo::redo(&db).await?;
    assert_eq!(projects::list(&db).await?[0].name, "ABC renamed");
    Ok(())
}

#[tokio::test]
async fn cycle_create_edit_delete() -> R {
    let db = Db::open_in_memory().await?;
    let c = history::step(
        "Create cycle",
        cycles::create(
            &db,
            Some("C1".into()),
            date(1),
            date(7),
            Look::default(),
            UI,
        ),
    )
    .await?;
    undo::undo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM cycles").await, 0);
    undo::redo(&db).await?;
    let mut e = c.clone();
    e.name = Some("C1b".into());
    history::step("Edit cycle", cycles::update(&db, e, UI)).await?;
    undo::undo(&db).await?;
    assert_eq!(cycles::list(&db).await?[0].name.as_deref(), Some("C1"));
    undo::redo(&db).await?;
    let a = issues::create(
        &db,
        NewIssue {
            title: "ABC one".into(),
            cycle_id: Some(c.id),
            ..NewIssue::default()
        },
        UI,
    )
    .await?;
    history::step("Delete cycle", cycles::delete(&db, c.id, UI)).await?;
    undo::undo(&db).await?;
    assert_eq!(issues::get(&db, a.id).await?.cycle_id, Some(c.id));
    undo::redo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM cycles").await, 0);
    Ok(())
}

#[tokio::test]
async fn property_def_delete_restores_values() -> R {
    let db = Db::open_in_memory().await?;
    let def = PropertyDef {
        key: "area".into(),
        name: "Area".into(),
        kind: PropertyKind::Text,
        options: vec![],
        project_id: None,
        position: 0,
        applies_to: taskapp_core::PropertyAppliesTo::Issues,
    };
    history::step("Create property", properties::create_def(&db, def, UI)).await?;
    undo::undo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM property_defs").await, 0);
    undo::redo(&db).await?;
    let a = seed(&db, "ABC one").await?;
    properties::set_value(&db, a.id, "area", Some(json!("ops")), UI).await?;
    history::step(
        "Rename property",
        properties::rename_def(&db, "area", "Zone", UI),
    )
    .await?;
    undo::undo(&db).await?;
    assert_eq!(properties::list_defs(&db).await?[0].name, "Area");
    undo::redo(&db).await?;
    undo::undo(&db).await?;
    history::step("Delete property", properties::delete_def(&db, "area", UI)).await?;
    assert_eq!(issues::get(&db, a.id).await?.properties.len(), 0);
    undo::undo(&db).await?;
    assert_eq!(
        issues::get(&db, a.id).await?.properties["area"],
        json!("ops")
    );
    assert_eq!(properties::list_defs(&db).await?.len(), 1);
    undo::redo(&db).await?;
    assert_eq!(properties::list_defs(&db).await?.len(), 0);
    Ok(())
}

#[tokio::test]
async fn relation_add_and_remove() -> R {
    let db = Db::open_in_memory().await?;
    let (a, b) = (seed(&db, "ABC a").await?, seed(&db, "ABC b").await?);
    let rel = Relation {
        from: a.id,
        to: b.id,
        kind: RelationKind::new("blocks"),
    };
    history::step("Relate", relations::add(&db, rel.clone(), UI)).await?;
    undo::undo(&db).await?;
    assert_eq!(relations::for_issue(&db, a.id).await?.len(), 0);
    undo::redo(&db).await?;
    assert_eq!(relations::for_issue(&db, a.id).await?.len(), 1);
    history::step("Unrelate", relations::remove(&db, rel, UI)).await?;
    undo::undo(&db).await?;
    assert_eq!(relations::for_issue(&db, a.id).await?.len(), 1);
    undo::redo(&db).await?;
    assert_eq!(relations::for_issue(&db, a.id).await?.len(), 0);
    Ok(())
}

#[tokio::test]
async fn status_create_edit_and_delete_with_move_round_trip() -> R {
    let db = Db::open_in_memory().await?;
    let extra = history::step(
        "Create status",
        statuses::create(&db, "ABC extra", StatusCategory::Started, "blue", None, UI),
    )
    .await?;
    undo::undo(&db).await?;
    assert_eq!(
        n(
            &db,
            "SELECT count(*) FROM statuses WHERE name = 'ABC extra'"
        )
        .await,
        0
    );
    undo::redo(&db).await?;
    history::step(
        "Edit status",
        statuses::update(
            &db,
            extra.id,
            Some("ABC more".into()),
            None,
            Look::default(),
            UI,
        ),
    )
    .await?;
    undo::undo(&db).await?;
    assert_eq!(
        statuses::list(&db).await?.last().expect("one").name,
        "ABC extra"
    );
    undo::redo(&db).await?;
    undo::undo(&db).await?;
    let a = issues::create(
        &db,
        NewIssue {
            title: "ABC one".into(),
            status_id: Some(extra.id),
            ..NewIssue::default()
        },
        UI,
    )
    .await?;
    history::step(
        "Delete status",
        statuses::delete(&db, extra.id, StatusId(1)),
    )
    .await?;
    assert_eq!(issues::get(&db, a.id).await?.status_id, StatusId(1));
    undo::undo(&db).await?;
    assert_eq!(issues::get(&db, a.id).await?.status_id, extra.id);
    undo::redo(&db).await?;
    assert_eq!(issues::get(&db, a.id).await?.status_id, StatusId(1));
    Ok(())
}

#[tokio::test]
async fn view_save_rename_delete() -> R {
    let db = Db::open_in_memory().await?;
    let def = ViewDef {
        filter: None,
        sorts: vec![],
        group_by: None,
        layout: Layout::List,
        columns: vec![],
        include_archived: false,
        ..ViewDef::default()
    };
    let base = views::list(&db).await?.len();
    let view = history::step(
        "Save view",
        views::save(&db, None, "ABC v", def.clone(), Look::default(), UI),
    )
    .await?;
    undo::undo(&db).await?;
    assert_eq!(views::list(&db).await?.len(), base);
    undo::redo(&db).await?;
    history::step(
        "Rename view",
        views::save(&db, Some(view.id), "ABC w", def, Look::default(), UI),
    )
    .await?;
    undo::undo(&db).await?;
    assert_eq!(views::list(&db).await?.last().expect("one").name, "ABC v");
    undo::redo(&db).await?;
    history::step("Delete view", views::delete(&db, view.id)).await?;
    undo::undo(&db).await?;
    assert_eq!(views::list(&db).await?.last().expect("one").name, "ABC w");
    undo::redo(&db).await?;
    assert_eq!(views::list(&db).await?.len(), base);
    Ok(())
}

#[tokio::test]
async fn pin_and_unpin() -> R {
    let db = Db::open_in_memory().await?;
    let pin = history::step(
        "Pin",
        pins::pin(&db, PinKind::Url, "https://example.com", None),
    )
    .await?;
    undo::undo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM pins").await, 0);
    undo::redo(&db).await?;
    history::step("Unpin", pins::unpin(&db, pin.id)).await?;
    undo::undo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM pins").await, 1);
    undo::redo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM pins").await, 0);
    Ok(())
}

#[tokio::test]
async fn template_create_and_delete() -> R {
    let db = Db::open_in_memory().await?;
    let input = TemplateInput {
        name: "Bug".into(),
        slug: "bug".into(),
        icon: None,
        color: None,
        body: TemplateBody::default(),
    };
    let tpl = history::step("Create template", templates::create(&db, input, UI)).await?;
    undo::undo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM issue_templates").await, 0);
    undo::redo(&db).await?;
    history::step("Delete template", templates::delete(&db, tpl.id, UI)).await?;
    undo::undo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM issue_templates").await, 1);
    undo::redo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM issue_templates").await, 0);
    Ok(())
}

#[tokio::test]
async fn work_block_create_move_delete() -> R {
    let db = Db::open_in_memory().await?;
    let issue = seed(&db, "ABC one").await?;
    let start = Utc
        .with_ymd_and_hms(2026, 12, 1, 9, 0, 0)
        .single()
        .expect("time");
    let end = start + chrono::Duration::hours(1);
    let block = history::step("Add block", blocks::create_block(&db, issue.id, start, end)).await?;
    undo::undo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM work_blocks").await, 0);
    undo::redo(&db).await?;
    let later = end + chrono::Duration::hours(1);
    history::step("Move block", blocks::move_block(&db, block.id, end, later)).await?;
    undo::undo(&db).await?;
    assert_eq!(blocks::blocks_between(&db, start, end).await?.len(), 1);
    undo::redo(&db).await?;
    history::step("Delete block", blocks::delete_block(&db, block.id)).await?;
    undo::undo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM work_blocks").await, 1);
    undo::redo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM work_blocks").await, 0);
    Ok(())
}

#[tokio::test]
async fn setting_set_and_change() -> R {
    let db = Db::open_in_memory().await?;
    history::step(
        "Set backup dir",
        settings::set(&db, "backup_dir", &"/tmp/x"),
    )
    .await?;
    undo::undo(&db).await?;
    assert_eq!(settings::get::<String>(&db, "backup_dir").await?, None);
    undo::redo(&db).await?;
    history::step(
        "Change backup dir",
        settings::set(&db, "backup_dir", &"/tmp/y"),
    )
    .await?;
    undo::undo(&db).await?;
    assert_eq!(
        settings::get::<String>(&db, "backup_dir").await?.as_deref(),
        Some("/tmp/x")
    );
    undo::redo(&db).await?;
    assert_eq!(
        settings::get::<String>(&db, "backup_dir").await?.as_deref(),
        Some("/tmp/y")
    );
    Ok(())
}

#[tokio::test]
async fn an_import_creating_a_label_and_a_project_is_undone_in_one_step() -> R {
    let db = Db::open_in_memory().await?;
    let plan = ImportPlan {
        source: "csv".into(),
        labels: vec!["ABC imported".into()],
        projects: vec!["ABC Import".into()],
        issues: vec![PlanIssue {
            external_id: "1".into(),
            title: "ABC from csv".into(),
            project: Some("ABC Import".into()),
            labels: vec!["ABC imported".into()],
            description: String::new(),
            status: None,
            priority: 0,
            start_date: None,
            due_date: None,
            estimate_minutes: None,
            points: None,
            properties: serde_json::Map::new(),
            parent: None,
            cycle: None,
        }],
        ..ImportPlan::default()
    };
    apply(&db, &plan).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM labels").await, 1);
    undo::undo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM labels").await, 0);
    assert_eq!(n(&db, "SELECT count(*) FROM projects").await, 0);
    assert_eq!(
        n(&db, "SELECT count(*) FROM issues WHERE archived_at IS NULL").await,
        0
    );
    assert!(undo::undo(&db).await.is_err(), "the import was one step");
    undo::redo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM labels").await, 1);
    assert_eq!(n(&db, "SELECT count(*) FROM projects").await, 1);
    assert_eq!(n(&db, "SELECT count(*) FROM issue_labels").await, 1);
    assert_eq!(
        n(
            &db,
            "SELECT count(*) FROM issues WHERE project_id IS NOT NULL AND archived_at IS NULL"
        )
        .await,
        1
    );
    Ok(())
}

#[tokio::test]
async fn row_conflict_is_refused_and_nothing_changes() -> R {
    let db = Db::open_in_memory().await?;
    let l = labels::create(&db, "ABC bug", "red", None, UI).await?;
    let mut e = l.clone();
    e.name = "ABC defect".into();
    history::step("Edit label", labels::update(&db, e.clone(), UI)).await?;
    e.name = "ABC other".into();
    labels::update(&db, e, ChangeSource::System).await?;
    let err = undo::undo(&db).await.expect_err("conflict");
    assert!(matches!(err, Error::Conflict(_)), "got {err:?}");
    assert_eq!(labels::list(&db).await?[0].name, "ABC other");
    Ok(())
}

#[tokio::test]
async fn a_rolled_back_first_write_still_clears_the_redo_stack() -> R {
    let db = Db::open_in_memory().await?;
    let a = seed(&db, "ABC one").await?;
    let p = projects::create(&db, "abc-p", "ABC P", None, None, UI).await?;
    projects::create(&db, "abc-q", "ABC Q", None, None, UI).await?;
    history::step(
        "Rename",
        issues::update(
            &db,
            a.id,
            IssuePatch {
                title: Some("ABC two".into()),
                ..IssuePatch::default()
            },
            UI,
        ),
    )
    .await?;
    undo::undo(&db).await?;
    assert!(undo::undo_status(&db).await?.redo.is_some());
    history::step("Second", async {
        // Records a name change, then fails on the duplicate slug and rolls back.
        let mut bad = p.clone();
        bad.name = "ABC changed".into();
        bad.slug = "abc-q".into();
        assert!(projects::update(&db, bad, UI).await.is_err());
        labels::create(&db, "ABC late", "red", None, UI).await
    })
    .await?;
    assert_eq!(undo::undo_status(&db).await?.redo, None);
    Ok(())
}
