use taskapp_core::{
    ChangeSource, Db, StatusCategory,
    import::{ImportPlan, PlanIssue, PlanStatus, apply},
    labels, palette, undo,
};

type R = Result<(), Box<dyn std::error::Error>>;

async fn n(db: &Db, sql: &str) -> i64 {
    sqlx::query_scalar(sql)
        .fetch_one(&db.pool)
        .await
        .expect("count")
}

fn plan() -> ImportPlan {
    ImportPlan {
        source: "csv".into(),
        labels: vec!["ABC imported".into()],
        statuses: vec![PlanStatus {
            name: "ABC Review".into(),
            category: StatusCategory::Started,
        }],
        issues: vec![PlanIssue {
            external_id: "1".into(),
            title: "ABC from csv".into(),
            labels: vec!["ABC imported".into()],
            status: Some("ABC Review".into()),
            description: String::new(),
            priority: 0,
            project: None,
            start_date: None,
            due_date: None,
            estimate_minutes: None,
            points: None,
            properties: serde_json::Map::new(),
            parent: None,
            cycle: None,
        }],
        ..ImportPlan::default()
    }
}

#[tokio::test]
async fn an_undone_import_can_be_imported_again() -> R {
    let db = Db::open_in_memory().await?;
    assert_eq!(apply(&db, &plan()).await?.created, 1);
    undo::undo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM import_log").await, 0);
    assert_eq!(apply(&db, &plan()).await?.created, 1);
    undo::undo(&db).await?;
    undo::redo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM import_log").await, 1);
    Ok(())
}

#[tokio::test]
async fn imported_labels_and_statuses_use_palette_keys_and_undo_after_reopen() -> R {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("a.db");
    let db = Db::open(&path).await?;
    apply(&db, &plan()).await?;
    let colors: Vec<String> = sqlx::query_scalar(
        "SELECT color FROM labels WHERE name = 'ABC imported' UNION ALL SELECT color FROM statuses WHERE name = 'ABC Review'",
    )
    .fetch_all(&db.pool)
    .await?;
    assert_eq!(colors, ["gray", "gray"]);
    drop(db);
    let db = Db::open(&path).await?;
    undo::undo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM labels").await, 0);
    Ok(())
}

#[tokio::test]
async fn the_legacy_colour_rewrite_does_not_make_undo_conflict() -> R {
    let db = Db::open_in_memory().await?;
    taskapp_core::history::step("Create label", async {
        labels::create(&db, "ABC old", "gray", None, ChangeSource::Ui).await
    })
    .await?;
    // What an older build left behind: the row and its recorded snapshot both hold the hex.
    sqlx::query("UPDATE labels SET color = '#95a2b3'")
        .execute(&db.pool)
        .await?;
    sqlx::query("UPDATE changes SET new_value = json_set(new_value, '$.color', '#95a2b3') WHERE entity = 'row:labels'")
        .execute(&db.pool)
        .await?;
    palette::map_legacy_colors(&db.pool).await?;
    undo::undo(&db).await?;
    assert_eq!(n(&db, "SELECT count(*) FROM labels").await, 0);
    Ok(())
}
