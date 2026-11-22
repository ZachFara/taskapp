use chrono::NaiveDate;
use taskapp_core::{
    ChangeSource, Db, Error, LabelId, NewIssue, ProjectId, StatusId, history, labels, projects,
    quickadd,
    templates::{self, SubIssue, TemplateBody, TemplateFields, TemplateInput},
};

type R = Result<(), Box<dyn std::error::Error>>;

const SRC: ChangeSource = ChangeSource::Ui;

fn today() -> NaiveDate {
    "2026-11-22".parse().expect("date")
}

async fn first_status(db: &Db) -> StatusId {
    let id: i64 = sqlx::query_scalar("SELECT id FROM statuses ORDER BY position, id LIMIT 1")
        .fetch_one(&db.pool)
        .await
        .expect("status");
    StatusId(id)
}

fn bug(body: TemplateBody) -> TemplateInput {
    TemplateInput {
        name: "Bug".into(),
        slug: "bug".into(),
        icon: Some("bug".into()),
        color: None,
        body,
    }
}

fn body(prefix: &str, fields: TemplateFields, subs: &[&str]) -> TemplateBody {
    TemplateBody {
        title_prefix: prefix.into(),
        fields,
        sub_issues: subs
            .iter()
            .map(|t| SubIssue {
                title: (*t).into(),
                fields: TemplateFields::default(),
            })
            .collect(),
    }
}

#[tokio::test]
async fn crud_round_trip_records_history() -> R {
    let db = Db::open_in_memory().await?;
    let t = templates::create(&db, bug(TemplateBody::default()), SRC).await?;
    assert_eq!(templates::list(&db).await?.len(), 1);
    assert_eq!(templates::get(&db, t.id).await?.slug, "bug");

    let renamed = TemplateInput {
        name: "Defect".into(),
        ..bug(TemplateBody::default())
    };
    assert_eq!(
        templates::update(&db, t.id, renamed, SRC).await?.name,
        "Defect"
    );

    templates::delete(&db, t.id, SRC).await?;
    assert_eq!(templates::list(&db).await?.len(), 0);
    assert!(matches!(
        templates::get(&db, t.id).await,
        Err(Error::NotFound(_))
    ));
    let changes = history::list_for(&db, "template", &t.id.0.to_string()).await?;
    assert!(changes.len() >= 3, "create, update and delete are recorded");
    Ok(())
}

#[tokio::test]
async fn apply_creates_sub_issues_with_template_fields() -> R {
    let db = Db::open_in_memory().await?;
    let label = labels::create(&db, "ABC-urgent", "red", SRC).await?;
    let fields = TemplateFields {
        priority: Some(2),
        label_ids: vec![label.id],
        description: Some("Steps to reproduce".into()),
        ..TemplateFields::default()
    };
    let t = templates::create(&db, bug(body("Bug: ", fields, &["Repro", "Fix"])), SRC).await?;

    let applied = templates::apply(
        &db,
        t.id,
        NewIssue {
            title: "Login fails".into(),
            ..NewIssue::default()
        },
        SRC,
    )
    .await?;

    assert_eq!(applied.issue.title, "Bug: Login fails");
    assert_eq!(applied.issue.priority, 2);
    assert_eq!(applied.issue.label_ids, vec![label.id]);
    assert_eq!(applied.issue.description, "Steps to reproduce");
    let subs: Vec<_> = applied
        .sub_issues
        .iter()
        .map(|s| s.title.as_str())
        .collect();
    assert_eq!(subs, ["Repro", "Fix"]);
    assert!(
        applied
            .sub_issues
            .iter()
            .all(|s| s.parent_id == Some(applied.issue.id))
    );
    assert_eq!(applied.skipped.len(), 0);
    Ok(())
}

#[tokio::test]
async fn overrides_win_over_template() -> R {
    let db = Db::open_in_memory().await?;
    let label = labels::create(&db, "ABC-label", "blue", SRC).await?;
    let project = projects::create(&db, "abc", "ABC", SRC).await?;
    let fields = TemplateFields {
        priority: Some(2),
        label_ids: vec![label.id],
        ..TemplateFields::default()
    };
    let t = templates::create(&db, bug(body("", fields, &[])), SRC).await?;

    let overrides = NewIssue {
        title: "Crash".into(),
        priority: Some(4),
        project_id: Some(project.id),
        ..NewIssue::default()
    };
    let issue = templates::apply(&db, t.id, overrides, SRC).await?.issue;

    assert_eq!(issue.priority, 4);
    assert_eq!(issue.project_id, Some(project.id));
    assert_eq!(
        issue.label_ids,
        vec![label.id],
        "unset override keeps template labels"
    );
    Ok(())
}

#[tokio::test]
async fn missing_references_are_skipped_and_reported() -> R {
    let db = Db::open_in_memory().await?;
    let fields = TemplateFields {
        label_ids: vec![LabelId(999)],
        project_id: Some(ProjectId(999)),
        status_id: Some(StatusId(999)),
        ..TemplateFields::default()
    };
    let t = templates::create(&db, bug(body("", fields, &[])), SRC).await?;

    let applied = templates::apply(
        &db,
        t.id,
        NewIssue {
            title: "Stale".into(),
            ..NewIssue::default()
        },
        SRC,
    )
    .await?;

    assert_eq!(applied.issue.label_ids.len(), 0);
    assert_eq!(applied.issue.project_id, None);
    assert_eq!(applied.issue.status_id, first_status(&db).await);
    assert_eq!(
        applied.skipped.len(),
        3,
        "label, project and status: {:?}",
        applied.skipped
    );
    Ok(())
}

#[tokio::test]
async fn quickadd_slug_picks_template_and_text_overrides() -> R {
    let db = Db::open_in_memory().await?;
    let fields = TemplateFields {
        priority: Some(3),
        ..TemplateFields::default()
    };
    templates::create(&db, bug(body("Bug: ", fields, &["Repro"])), SRC).await?;

    let issue = quickadd::create_from(&db, "/bug Fix login p1", today(), "UTC", SRC).await?;

    assert_eq!(issue.title, "Bug: Fix login");
    assert_eq!(issue.priority, 1);
    assert_eq!(templates::list(&db).await?.len(), 1);
    Ok(())
}

#[tokio::test]
async fn unknown_slug_is_invalid_and_lists_known_slugs() -> R {
    let db = Db::open_in_memory().await?;
    templates::create(&db, bug(TemplateBody::default()), SRC).await?;

    let err = quickadd::create_from(&db, "/nope Fix login", today(), "UTC", SRC)
        .await
        .expect_err("unknown slug");

    let Error::Invalid(msg) = err else {
        return Err(format!("expected Invalid, got {err:?}").into());
    };
    assert!(msg.contains("bug"), "message lists known slugs: {msg}");
    Ok(())
}
