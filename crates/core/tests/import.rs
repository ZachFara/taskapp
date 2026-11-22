use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicUsize, Ordering},
};

use serde_json::{Value, json};
use taskapp_core::{
    Db, Error, PropertyKind, RelationKind, Result,
    import::{
        ImportPlan, apply,
        csv::{self, guess_mapping},
        linear::{self, LinearApi, Token},
    },
};

type R = std::result::Result<(), Box<dyn std::error::Error>>;

const CSV: &str = "Name,Notes,State,Prio,Project,Tags,Start,Due,Points,Area,Effort\n\
Write docs,\"Cover A, B\",Todo,high,Alpha,\"bug;ui\",2026-12-01,2026-12-05,90,home,3\n\
Fix login,,In Progress,1,Alpha,bug,,,,work,5\n\
Ship it,,Done,low,Beta,,,,,home,8\n";

fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(a, b)| ((*a).into(), (*b).into()))
        .collect()
}

fn full_map() -> BTreeMap<String, String> {
    map(&[
        ("Name", "title"),
        ("Notes", "description"),
        ("State", "status"),
        ("Prio", "priority"),
        ("Project", "project"),
        ("Tags", "labels"),
        ("Start", "start_date"),
        ("Due", "due_date"),
        ("Points", "estimate"),
        ("Area", "property:Area"),
        ("Effort", "property:Effort"),
    ])
}

async fn count(db: &Db) -> Result<Vec<i64>> {
    let mut out = vec![];
    for t in [
        "issues",
        "labels",
        "projects",
        "statuses",
        "cycles",
        "property_defs",
        "changes",
        "import_log",
        "relations",
    ] {
        out.push(
            sqlx::query_scalar(&format!("SELECT count(*) FROM {t}"))
                .fetch_one(&db.pool)
                .await?,
        );
    }
    Ok(out)
}

#[tokio::test]
async fn csv_happy_path() -> R {
    let db = Db::open_in_memory().await?;
    let plan = csv::plan(&db, CSV, Some(&full_map())).await?;
    assert_eq!(plan.issues.len(), 3);
    assert_eq!(plan.skipped, []);
    assert_eq!(plan.projects, ["Alpha", "Beta"]);
    assert_eq!(plan.labels, ["bug", "ui"]);
    let first = &plan.issues[0];
    assert_eq!(first.title, "Write docs");
    assert_eq!(first.description, "Cover A, B");
    assert_eq!(first.priority, 2);
    assert_eq!(first.estimate_minutes, Some(90));
    assert_eq!(first.labels, ["bug", "ui"]);
    Ok(())
}

#[tokio::test]
async fn header_guess() {
    let headers: Vec<String> = [
        "Title",
        "Description",
        "Status",
        "Priority",
        "Labels",
        "Due date",
        "Mystery",
    ]
    .map(String::from)
    .to_vec();
    let m = guess_mapping(&headers);
    assert_eq!(m.get("Title").map(String::as_str), Some("title"));
    assert_eq!(m.get("Due date").map(String::as_str), Some("due_date"));
    assert_eq!(m.get("Labels").map(String::as_str), Some("labels"));
    assert!(!m.contains_key("Mystery"));
}

#[tokio::test]
async fn default_mapping_lists_unmapped_columns() -> R {
    let db = Db::open_in_memory().await?;
    let plan = csv::plan(&db, "Title,Mystery\nA,x\n", None).await?;
    assert_eq!(plan.unmapped, ["Mystery"]);
    assert_eq!(plan.issues.len(), 1);
    Ok(())
}

#[tokio::test]
async fn bad_rows_skipped_with_row_number() -> R {
    let db = Db::open_in_memory().await?;
    let text = "Title,Due,Prio\nGood,2026-12-01,2\n,2026-12-01,2\nBad date,soon,2\nBad prio,2026-12-01,9\nAlso good,,\n";
    let m = map(&[
        ("Title", "title"),
        ("Due", "due_date"),
        ("Prio", "priority"),
    ]);
    let plan = csv::plan(&db, text, Some(&m)).await?;
    assert_eq!(plan.issues.len(), 2);
    let rows: Vec<u32> = plan.skipped.iter().map(|s| s.row).collect();
    assert_eq!(rows, [3, 4, 5]);
    assert!(plan.skipped.iter().all(|s| !s.reason.is_empty()));
    Ok(())
}

#[tokio::test]
async fn custom_property_created_with_inferred_type() -> R {
    let db = Db::open_in_memory().await?;
    let plan = csv::plan(&db, CSV, Some(&full_map())).await?;
    let kind = |k: &str| {
        plan.property_defs
            .iter()
            .find(|d| d.key == k)
            .map(|d| d.kind)
    };
    assert_eq!(kind("effort"), Some(PropertyKind::Number));
    assert_eq!(kind("area"), Some(PropertyKind::Text));
    apply(&db, &plan).await?;
    let defs = taskapp_core::properties::list_defs(&db).await?;
    assert_eq!(defs.len(), 2);
    let found = taskapp_core::search::search(&db, "docs", 5).await?;
    assert_eq!(found[0].properties.get("effort"), Some(&json!(3.0)));
    assert_eq!(found[0].properties.get("area"), Some(&json!("home")));
    Ok(())
}

#[tokio::test]
async fn dry_run_writes_nothing() -> R {
    let db = Db::open_in_memory().await?;
    let before = count(&db).await?;
    csv::plan(&db, CSV, Some(&full_map())).await?;
    assert_eq!(count(&db).await?, before);
    Ok(())
}

#[tokio::test]
async fn apply_equals_plan_and_labels_one_history_step() -> R {
    let db = Db::open_in_memory().await?;
    let plan = csv::plan(&db, CSV, Some(&full_map())).await?;
    let done = apply(&db, &plan).await?;
    assert_eq!(done.created, 3);
    let c = count(&db).await?;
    assert_eq!(c[0], 3);
    assert_eq!(c[1], 2);
    assert_eq!(c[2], 2);
    assert_eq!(c[7], 3);
    let steps: Vec<Option<String>> = sqlx::query_scalar("SELECT DISTINCT step_label FROM changes")
        .fetch_all(&db.pool)
        .await?;
    assert_eq!(
        steps,
        [Some("Import 3 issues".to_owned())],
        "one step for the whole import"
    );
    let titles: Vec<String> = sqlx::query_scalar("SELECT title FROM issues ORDER BY number")
        .fetch_all(&db.pool)
        .await?;
    assert_eq!(
        titles,
        plan.issues
            .iter()
            .map(|i| i.title.clone())
            .collect::<Vec<_>>()
    );
    Ok(())
}

#[tokio::test]
async fn reimport_does_not_duplicate() -> R {
    let db = Db::open_in_memory().await?;
    let plan = csv::plan(&db, CSV, Some(&full_map())).await?;
    apply(&db, &plan).await?;
    let after_first = count(&db).await?;
    let again = csv::plan(&db, CSV, Some(&full_map())).await?;
    assert_eq!(again.issues, []);
    assert_eq!(again.skipped.len(), 3);
    // A stale plan applied twice must not duplicate either.
    let done = apply(&db, &plan).await?;
    assert_eq!(done.created, 0);
    let after_second = count(&db).await?;
    assert_eq!(after_first[0], after_second[0]);
    assert_eq!(after_first[7], after_second[7]);
    Ok(())
}

struct Pages(Vec<Value>, AtomicUsize);

impl LinearApi for Pages {
    fn query(&self, _query: &str, vars: Value) -> impl Future<Output = Result<Value>> + Send {
        let i = self.1.fetch_add(1, Ordering::SeqCst);
        if i > 0 {
            assert_eq!(
                vars["after"], "cursor-1",
                "second page asks for the first page's end cursor"
            );
        }
        let page = self.0.get(i).cloned();
        async move { page.ok_or_else(|| Error::Invalid("no more pages".into())) }
    }
}

fn node(id: &str, title: &str, state: (&str, &str), extra: &Value) -> Value {
    let mut n = json!({
        "identifier": id, "title": title, "description": "body", "priority": 2, "estimate": 3.0,
        "dueDate": "2026-12-24", "state": {"name": state.0, "type": state.1},
        "project": {"name": "Roadmap"}, "labels": {"nodes": [{"name": "bug"}]},
        "cycle": {"number": 7, "name": "Sprint", "startsAt": "2026-12-01T00:00:00.000Z", "endsAt": "2026-12-14T00:00:00.000Z"},
        "parent": null, "relations": {"nodes": []}
    });
    for (k, v) in extra.as_object().into_iter().flatten() {
        n[k] = v.clone();
    }
    n
}

fn pages() -> Pages {
    let p1 = json!({"issues": {"nodes": [
        node("ENG-1", "Parent", ("Triage Me", "triage"), &json!({"relations": {"nodes": [
            {"type": "blocks", "relatedIssue": {"identifier": "ENG-2"}}]}})),
    ], "pageInfo": {"hasNextPage": true, "endCursor": "cursor-1"}}});
    let p2 = json!({"issues": {"nodes": [
        node("ENG-2", "Child", ("Shipped", "completed"), &json!({"parent": {"identifier": "ENG-1"}})),
    ], "pageInfo": {"hasNextPage": false, "endCursor": null}}});
    Pages(vec![p1, p2], AtomicUsize::new(0))
}

#[tokio::test]
async fn linear_fixture_maps_states_labels_relations_and_sub_issues() -> R {
    let db = Db::open_in_memory().await?;
    // Where the workspace counts time, one Linear point is one hour.
    taskapp_core::settings::set(&db, "estimation", &"time").await?;
    let plan: ImportPlan = linear::plan(&db, &pages()).await?;
    assert_eq!(plan.issues.len(), 2);
    let names: Vec<_> = plan.statuses.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Triage Me", "Shipped"]);
    assert_eq!(plan.labels, ["bug"]);
    assert_eq!(plan.projects, ["Roadmap"]);
    assert_eq!(plan.cycles.len(), 1);
    assert_eq!(plan.relations[0].kind, RelationKind::Blocks);
    assert_eq!(plan.issues[1].parent.as_deref(), Some("ENG-1"));
    assert_eq!(plan.issues[0].estimate_minutes, Some(180));
    apply(&db, &plan).await?;
    let c = count(&db).await?;
    assert_eq!((c[0], c[8]), (2, 1));
    let parent: Option<i64> =
        sqlx::query_scalar("SELECT parent_id FROM issues WHERE title = 'Child'")
            .fetch_one(&db.pool)
            .await?;
    assert!(parent.is_some());
    let cat: String = sqlx::query_scalar("SELECT category FROM statuses WHERE name = 'Triage Me'")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(cat, "backlog");
    let logged: String =
        sqlx::query_scalar("SELECT source FROM import_log WHERE external_id = 'ENG-1'")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(logged, "linear");
    // Second run: nothing new.
    let again = linear::plan(&db, &pages()).await?;
    assert_eq!(again.issues, []);
    Ok(())
}

#[tokio::test]
async fn linear_estimate_becomes_points_where_the_workspace_uses_points() -> R {
    let db = Db::open_in_memory().await?;
    let plan = linear::plan(&db, &pages()).await?;
    assert_eq!(
        (plan.issues[0].points, plan.issues[0].estimate_minutes),
        (Some(3.0), None)
    );
    apply(&db, &plan).await?;
    let got: Option<f64> = sqlx::query_scalar("SELECT points FROM issues WHERE title = 'Parent'")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(got, Some(3.0));
    Ok(())
}

#[tokio::test]
async fn token_never_in_debug_or_errors() {
    let token = Token::new("lin_api_SECRET123");
    assert!(!format!("{token:?}").contains("SECRET123"));
    let leaky = json!({"errors": [{"message": "bad key lin_api_SECRET123"}]});
    let err = linear::response_data(&leaky, &token)
        .err()
        .map_or_else(String::new, |e| e.to_string());
    assert!(err.contains("bad key"));
    assert!(!err.contains("SECRET123"));
    let client = linear::CurlLinear::new(token);
    assert!(!format!("{client:?}").contains("SECRET123"));
}

#[tokio::test]
async fn import_undoes_as_one_step_including_what_it_created() -> R {
    let db = Db::open_in_memory().await?;
    let plain = csv::plan(&db, "Title\nA\nB\n", None).await?;
    apply(&db, &plain).await?;
    taskapp_core::undo::undo(&db).await?;
    let live: i64 = sqlx::query_scalar("SELECT count(*) FROM issues WHERE archived_at IS NULL")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(live, 0);
    let rich = csv::plan(&db, "Title,Tags\nC,x\n", None).await?;
    let rich = ImportPlan {
        labels: vec!["x".into()],
        ..rich
    };
    apply(&db, &rich).await?;
    taskapp_core::undo::undo(&db).await?;
    let labels: i64 = sqlx::query_scalar("SELECT count(*) FROM labels WHERE name = 'x'")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(labels, 0);
    Ok(())
}
