//! The same fixtures as `ui/src/lib/api.mock.ts`, created through core so the real and mock modes match.

use chrono::NaiveDate;
use serde_json::{Map, json};
use taskapp_core::{
    ChangeSource::Ui, Db, Due, NewIssue, ProjectId, PropertyDef, PropertyKind, Result,
    SelectOption, StatusId, issues, projects, properties,
};

fn def(
    key: &str,
    name: &str,
    kind: PropertyKind,
    options: &[(&str, &str)],
    position: i32,
) -> PropertyDef {
    PropertyDef {
        key: key.into(),
        name: name.into(),
        kind,
        options: options
            .iter()
            .map(|(value, color)| SelectOption {
                value: (*value).into(),
                color: (*color).into(),
            })
            .collect(),
        project_id: None,
        position,
    }
}

/// Fills an empty database: two projects, three properties, 20 issues. Statuses come from the migration.
///
/// # Errors
/// Fails when a core call fails, for example on a database that already holds these rows.
pub async fn run(db: &Db) -> Result<()> {
    for (slug, name) in [("home", "Home"), ("work", "Work")] {
        projects::create(db, slug, name, Ui).await?;
    }
    let area = [("home", "#4caf50"), ("work", "#2196f3")];
    for d in [
        def("area", "Area", PropertyKind::Select, &area, 0),
        def("effort", "Effort", PropertyKind::Number, &[], 1),
        def("review_on", "Review on", PropertyKind::Date, &[], 2),
    ] {
        properties::create_def(db, d, Ui).await?;
    }
    for n in 1..=20_u32 {
        let mut props = Map::new();
        match n % 3 {
            1 => {
                props.insert("area".into(), json!("home"));
                props.insert("effort".into(), json!(n));
            }
            2 => {
                props.insert("area".into(), json!("work"));
            }
            _ => {}
        }
        let dec = |day: u32| NaiveDate::from_ymd_opt(2026, 12, day);
        issues::create(
            db,
            NewIssue {
                title: format!("Fixture task {n}"),
                description: Some(String::new()),
                status_id: Some(StatusId(i64::from((n - 1) % 5 + 1))),
                priority: u8::try_from(n % 5).ok(),
                project_id: (n % 3 != 0).then_some(ProjectId(i64::from(n % 2 + 1))),
                start_date: if n % 4 == 0 { dec(n) } else { None },
                due: if n % 4 == 0 {
                    dec(n + 2).map(Due::Date)
                } else {
                    None
                },
                estimate_minutes: (n % 2 == 1).then_some(480),
                properties: Some(props),
                ..NewIssue::default()
            },
            Ui,
        )
        .await?;
    }
    Ok(())
}
