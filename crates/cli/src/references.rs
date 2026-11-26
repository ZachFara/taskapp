use taskapp_core::{
    CycleId, Db, Error, LabelId, ProjectId, Result, StatusId, cycles, labels, projects, statuses,
};

fn resolve(kind: &str, input: &str, matches: impl Iterator<Item = i64>) -> Result<i64> {
    if let Ok(id) = input.parse::<i64>() {
        return Ok(id);
    }
    let mut matches = matches;
    let id = matches
        .next()
        .ok_or_else(|| Error::NotFound(format!("{kind} {input}")))?;
    if matches.any(|other| other != id) {
        return Err(Error::Conflict(format!(
            "ambiguous {kind} {input}; use an id"
        )));
    }
    Ok(id)
}

pub async fn status(db: &Db, input: Option<&str>) -> Result<Option<StatusId>> {
    let Some(input) = input else { return Ok(None) };
    let all = statuses::list(db).await?;
    resolve(
        "status",
        input,
        all.iter()
            .filter(|s| s.name.eq_ignore_ascii_case(input))
            .map(|s| s.id.0),
    )
    .map(StatusId)
    .map(Some)
}

pub async fn project(db: &Db, input: Option<&str>) -> Result<Option<ProjectId>> {
    let Some(input) = input else { return Ok(None) };
    let all = projects::list(db).await?;
    resolve(
        "project",
        input,
        all.iter()
            .filter(|p| p.name.eq_ignore_ascii_case(input) || p.slug.eq_ignore_ascii_case(input))
            .map(|p| p.id.0),
    )
    .map(ProjectId)
    .map(Some)
}

pub async fn cycle(db: &Db, input: Option<&str>) -> Result<Option<CycleId>> {
    let Some(input) = input else { return Ok(None) };
    let all = cycles::list(db).await?;
    resolve(
        "cycle",
        input,
        all.iter()
            .filter(|c| {
                c.name
                    .as_deref()
                    .is_some_and(|name| name.eq_ignore_ascii_case(input))
            })
            .map(|c| c.id.0),
    )
    .map(CycleId)
    .map(Some)
}

pub async fn labels(db: &Db, inputs: &[String]) -> Result<Vec<LabelId>> {
    if inputs.is_empty() {
        return Ok(Vec::new());
    }
    let all = labels::list(db).await?;
    inputs
        .iter()
        .map(|input| {
            resolve(
                "label",
                input,
                all.iter()
                    .filter(|l| l.name.eq_ignore_ascii_case(input))
                    .map(|l| l.id.0),
            )
            .map(LabelId)
        })
        .collect()
}
