//! The same fixtures as `ui/src/lib/api.mock.ts`, created through core so the real and mock modes match.

use chrono::NaiveDate;
use serde_json::{Map, json};
use taskapp_core::{
    ChangeSource::Ui, Db, Due, NewIssue, ProjectId, PropertyDef, PropertyKind, Result,
    SelectOption, StatusId, cycles, issues, labels, palette::Look, projects, properties, settings,
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

/// Fills an empty database: three projects, three labels, a cycle, three properties, 20 issues. Statuses come from the migration.
///
/// # Errors
/// Fails when a core call fails, for example on a database that already holds these rows.
pub async fn run(db: &Db) -> Result<()> {
    reference_data(db).await?;
    for n in 1..=20_u32 {
        issue(db, n).await?;
    }
    Ok(())
}

async fn reference_data(db: &Db) -> Result<()> {
    settings::set(db, "issue_prefix", &"ABC").await?;
    for (slug, name) in [
        ("website", "Website relaunch"),
        ("mobile", "Mobile app"),
        ("office", "Office move"),
    ] {
        projects::create(db, slug, name, None, None, Ui).await?;
    }
    for (name, color) in [("Bug", "red"), ("Feature", "blue"), ("Design", "purple")] {
        labels::create(db, name, color, None, Ui).await?;
    }
    let day = |d: u32| NaiveDate::from_ymd_opt(2026, 12, d).unwrap_or_default();
    cycles::create(
        db,
        Some("Sprint 1".into()),
        day(1),
        day(14),
        Look::default(),
        Ui,
    )
    .await?;
    let area = [("home", "#4caf50"), ("work", "#2196f3")];
    for d in [
        def("area", "Area", PropertyKind::Select, &area, 0),
        def("effort", "Effort", PropertyKind::Number, &[], 1),
        def("review_on", "Review on", PropertyKind::Date, &[], 2),
    ] {
        properties::create_def(db, d, Ui).await?;
    }
    Ok(())
}

async fn issue(db: &Db, n: u32) -> Result<()> {
    {
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
                project_id: match n % 3 {
                    0 if n.is_multiple_of(6) => Some(ProjectId(3)),
                    0 => None,
                    _ => Some(ProjectId(i64::from(n % 2 + 1))),
                },
                start_date: if n.is_multiple_of(4) { dec(n) } else { None },
                due: if n.is_multiple_of(4) {
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
