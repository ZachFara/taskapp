//! The same fixtures as `ui/src/lib/api.mock.ts`, created through core so the real and mock modes match.

use chrono::{Days, NaiveDate, Utc};
use serde_json::{Map, Value, json};
use taskapp_core::{
    ChangeSource::Ui, Db, Due, Error, IssueId, LabelId, MilestoneId, NewIssue, ProjectId,
    PropertyDef, PropertyKind, Relation, RelationKind, Result, SelectOption, StatusId,
    TemplateInput, ViewDef, ViewId, cycles, issues, labels, palette::Look, projects, properties,
    relations, templates, views, workspace,
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

/// One issue of the sample workspace; dates are days from today, `minutes` the estimate.
struct Row {
    title: &'static str,
    description: &'static str,
    status: i64,
    priority: u8,
    project: i64,
    start: Option<i64>,
    due: Option<i64>,
    minutes: u32,
    labels: &'static [i64],
    parent: i64,
    cycle: i64,
    milestone: i64,
    area: &'static str,
    effort: u32,
}

/// The same workspace as `ui/src/lib/api.mock.ts`, which the contract tests compare with this one.
const E: Row = Row {
    title: "",
    description: "",
    status: 1,
    priority: 0,
    project: 0,
    start: None,
    due: None,
    minutes: 0,
    labels: &[],
    parent: 0,
    cycle: 0,
    milestone: 0,
    area: "",
    effort: 0,
};

static ROWS: [Row; 24] = [
    Row {
        title: "Redesign homepage hero",
        description: "Replace the stock photo with the new brand illustration and tighten the headline.",
        status: 3,
        priority: 3,
        project: 1,
        start: Some(-5),
        due: Some(2),
        minutes: 960,
        labels: &[3],
        cycle: 1,
        milestone: 1,
        area: "work",
        effort: 13,
        ..E
    },
    Row {
        title: "Write launch announcement",
        status: 2,
        priority: 2,
        project: 1,
        start: Some(0),
        due: Some(5),
        minutes: 120,
        labels: &[5],
        milestone: 1,
        ..E
    },
    Row {
        title: "Fix broken links in footer",
        status: 4,
        priority: 1,
        project: 1,
        due: Some(-3),
        minutes: 60,
        labels: &[1],
        ..E
    },
    Row {
        title: "Migrate blog to new CMS",
        description: "Export posts, redirect the old URLs and check every embedded image.",
        status: 3,
        priority: 4,
        project: 1,
        start: Some(-10),
        due: Some(-2),
        minutes: 2400,
        labels: &[6],
        cycle: 1,
        area: "work",
        effort: 21,
        ..E
    },
    Row {
        title: "Set up analytics dashboard",
        status: 1,
        project: 1,
        minutes: 240,
        labels: &[2],
        ..E
    },
    Row {
        title: "Audit page speed",
        status: 2,
        priority: 2,
        project: 1,
        start: Some(1),
        due: Some(4),
        minutes: 180,
        labels: &[4],
        effort: 8,
        ..E
    },
    Row {
        title: "Mobile onboarding flow",
        description: "Three screens: welcome, permissions and account setup. Skippable at every step.",
        status: 3,
        priority: 3,
        project: 2,
        start: Some(-3),
        due: Some(6),
        minutes: 1440,
        labels: &[2, 3],
        cycle: 1,
        milestone: 2,
        area: "work",
        effort: 34,
        ..E
    },
    Row {
        title: "Welcome screen illustrations",
        status: 3,
        priority: 2,
        project: 2,
        start: Some(-3),
        due: Some(1),
        minutes: 480,
        labels: &[3],
        parent: 7,
        cycle: 1,
        ..E
    },
    Row {
        title: "Permission prompts copy",
        status: 2,
        priority: 1,
        project: 2,
        start: Some(2),
        due: Some(5),
        minutes: 60,
        labels: &[5],
        parent: 7,
        ..E
    },
    Row {
        title: "Crash on launch for Android 9",
        status: 3,
        priority: 4,
        project: 2,
        due: Some(-1),
        minutes: 240,
        labels: &[1],
        cycle: 1,
        ..E
    },
    Row {
        title: "Push notification settings",
        status: 2,
        priority: 2,
        project: 2,
        start: Some(3),
        due: Some(9),
        minutes: 960,
        labels: &[2],
        cycle: 2,
        ..E
    },
    Row {
        title: "Offline mode research",
        description: "Compare local-first sync options and write up the trade-offs.",
        status: 1,
        priority: 1,
        project: 2,
        minutes: 480,
        labels: &[4],
        ..E
    },
    Row {
        title: "Dark mode icons",
        status: 4,
        priority: 2,
        project: 2,
        start: Some(-8),
        due: Some(-4),
        minutes: 240,
        labels: &[3],
        ..E
    },
    Row {
        title: "Update app store listing",
        status: 2,
        priority: 2,
        project: 2,
        due: Some(3),
        minutes: 60,
        labels: &[5],
        parent: 7,
        cycle: 2,
        ..E
    },
    Row {
        title: "Confirm new office lease",
        status: 4,
        priority: 3,
        project: 3,
        start: Some(-14),
        due: Some(-9),
        ..E
    },
    Row {
        title: "Order standing desks",
        status: 3,
        priority: 2,
        project: 3,
        start: Some(-2),
        due: Some(4),
        minutes: 120,
        labels: &[6],
        cycle: 1,
        area: "home",
        ..E
    },
    Row {
        title: "Book movers",
        status: 2,
        priority: 3,
        project: 3,
        start: Some(2),
        due: Some(11),
        minutes: 60,
        area: "home",
        ..E
    },
    Row {
        title: "Plan seating chart",
        status: 2,
        priority: 2,
        project: 3,
        start: Some(4),
        due: Some(10),
        minutes: 240,
        labels: &[3],
        ..E
    },
    Row {
        title: "Set up network and Wi-Fi",
        description: "Two access points per floor, a guest network and a printer VLAN.",
        status: 1,
        priority: 3,
        project: 3,
        due: Some(14),
        minutes: 960,
        labels: &[6],
        ..E
    },
    Row {
        title: "Order access points",
        status: 1,
        priority: 2,
        project: 3,
        minutes: 480,
        labels: &[6],
        parent: 19,
        ..E
    },
    Row {
        title: "Send address change notices",
        status: 2,
        priority: 2,
        project: 3,
        due: Some(7),
        minutes: 60,
        labels: &[5],
        ..E
    },
    Row {
        title: "Cancel old storage unit",
        status: 5,
        project: 3,
        ..E
    },
    Row {
        title: "Rename Contact page",
        status: 2,
        priority: 1,
        minutes: 30,
        ..E
    },
    Row {
        title: "Team retro notes",
        status: 1,
        labels: &[4],
        ..E
    },
];

/// Fills an empty database with a believable workspace: three projects, six labels, two cycles, two milestones,
/// two templates, four views, three properties, 24 issues. Statuses and the first three views come from the migration.
///
/// # Errors
/// Fails when a core call fails, for example on a database that already holds these rows.
pub async fn run(db: &Db) -> Result<()> {
    reference_data(db).await?;
    let today = Utc::now().date_naive();
    for (k, row) in ROWS.iter().enumerate() {
        issue(db, today, row, k).await?;
    }
    for (from, to, kind) in [
        (4, 6, RelationKind::Blocks),
        (16, 18, RelationKind::Blocks),
        (10, 11, RelationKind::Related),
    ] {
        let (from, to) = (IssueId(from), IssueId(to));
        relations::add(db, Relation { from, to, kind }, Ui).await?;
    }
    seed_views(db, today).await
}

fn bad(e: &serde_json::Error) -> Error {
    Error::Invalid(e.to_string())
}

fn at(today: NaiveDate, days: i64) -> NaiveDate {
    let d = Days::new(days.unsigned_abs());
    (if days < 0 {
        today.checked_sub_days(d)
    } else {
        today.checked_add_days(d)
    })
    .unwrap_or(today)
}

async fn reference_data(db: &Db) -> Result<()> {
    workspace::setup(db, "Cairn", "ABC", Some("UTC")).await?;
    for (slug, name) in [
        ("website", "Website relaunch"),
        ("mobile", "Mobile app"),
        ("office", "Office move"),
    ] {
        projects::create(db, slug, name, None, None, Ui).await?;
    }
    for (name, color, icon) in [
        ("Bug", "red", "flag"),
        ("Feature", "blue", "bolt"),
        ("Design", "purple", "star"),
        ("Research", "teal", "target"),
        ("Docs", "green", "bookmark"),
        ("Ops", "orange", "tag"),
    ] {
        labels::create(db, name, color, Some(format!("icon:{icon}")), Ui).await?;
    }
    let today = Utc::now().date_naive();
    for (name, from, to) in [("Sprint 1", -6, 7), ("Sprint 2", 8, 21)] {
        let (from, to) = (at(today, from), at(today, to));
        cycles::create(db, Some(name.into()), from, to, Look::default(), Ui).await?;
    }
    for (project, name, days) in [(1, "Soft launch", 21), (2, "Store release", 30)] {
        let target = Some(at(today, days));
        projects::create_milestone(db, ProjectId(project), name, target, Ui).await?;
    }
    let area = [("home", "#4caf50"), ("work", "#2196f3")];
    for d in [
        def("area", "Area", PropertyKind::Select, &area, 0),
        def("effort", "Effort", PropertyKind::Number, &[], 1),
        def("review_on", "Review on", PropertyKind::Date, &[], 2),
    ] {
        properties::create_def(db, d, Ui).await?;
    }
    for t in [
        json!({"name": "Bug", "slug": "bug", "icon": "icon:flag", "color": "red", "body": {
            "title_prefix": "Bug: ",
            "fields": {"description": "Steps to reproduce:\n\nExpected:\n\nActual:", "priority": 2, "label_ids": [1]},
            "sub_issues": [{"title": "Reproduce", "fields": {"priority": 2}}]}}),
        json!({"name": "Research idea", "slug": "research", "icon": "icon:target", "color": "teal", "body": {
            "title_prefix": "Research: ",
            "fields": {"description": "Question:\n\nWhy it matters:", "label_ids": [4], "estimate_minutes": 240},
            "sub_issues": []}}),
    ] {
        let input: TemplateInput = serde_json::from_value(t).map_err(|e| bad(&e))?;
        templates::create(db, input, Ui).await?;
    }
    Ok(())
}

async fn issue(db: &Db, today: NaiveDate, r: &Row, k: usize) -> Result<()> {
    let mut props = Map::new();
    if !r.area.is_empty() {
        props.insert("area".into(), json!(r.area));
    }
    if r.effort > 0 {
        props.insert("effort".into(), json!(r.effort));
    }
    let opt = |id: i64| (id > 0).then_some(id);
    let created = Utc::now() - chrono::Duration::days(30 - i64::try_from(k + 1).unwrap_or(0));
    issues::create(
        db,
        NewIssue {
            title: r.title.into(),
            description: Some(r.description.into()),
            status_id: Some(StatusId(r.status)),
            priority: Some(r.priority),
            project_id: opt(r.project).map(ProjectId),
            milestone_id: opt(r.milestone).map(MilestoneId),
            parent_id: opt(r.parent).map(IssueId),
            cycle_id: opt(r.cycle).map(taskapp_core::CycleId),
            label_ids: Some(r.labels.iter().map(|&l| LabelId(l)).collect()),
            start_date: r.start.map(|d| at(today, d)),
            due: r.due.map(|d| Due::Date(at(today, d))),
            estimate_minutes: (r.minutes > 0).then_some(r.minutes),
            properties: Some(props),
            created_at: Some(created),
            ..NewIssue::default()
        },
        Ui,
    )
    .await?;
    Ok(())
}

/// Active and Board group by status, the timeline by project, and "This week" is added; the first three get icons.
async fn seed_views(db: &Db, today: NaiveDate) -> Result<()> {
    let open = json!({"Cond": {"field": {"Fixed": "StatusCategory"}, "op": "In", "value": ["unstarted", "started"]}});
    let def = |filter: Value, group: Value, layout: &str| {
        let v = json!({"filter": filter, "sorts": [], "group_by": group, "layout": layout, "columns": [], "include_archived": false});
        serde_json::from_value::<ViewDef>(v).map_err(|e| bad(&e))
    };
    let status = json!({"Fixed": "Status"});
    let week = json!({"And": [open.clone(), {"Cond": {"field": {"Fixed": "DueDate"}, "op": "Lte", "value": at(today, 7).to_string()}}]});
    let not_done =
        json!({"Cond": {"field": {"Fixed": "StatusCategory"}, "op": "Neq", "value": "completed"}});
    let all = [
        (
            Some(1),
            "Active",
            def(open.clone(), status.clone(), "List")?,
            "list",
        ),
        (Some(2), "Board", def(open, status, "Board")?, "board"),
        (
            Some(3),
            "Timeline: all projects",
            def(not_done, json!({"Fixed": "Project"}), "Gantt")?,
            "timeline",
        ),
        (
            None,
            "This week",
            def(week, Value::Null, "List")?,
            "calendar",
        ),
    ];
    for (id, name, d, icon) in all {
        let look = Look {
            icon: Some(format!("icon:{icon}")),
            color: None,
        };
        views::save(db, id.map(ViewId), name, d, look, Ui).await?;
    }
    Ok(())
}
