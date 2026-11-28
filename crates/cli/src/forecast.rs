//! `taskapp forecast`: when the open issues of a project or cycle are likely done.

use serde_json::json;
use taskapp_core::{
    Db, Error, FieldRef, Filter, FixedField, Forecast, ForecastQuery, Op, Result, forecast,
};

use crate::references;

#[derive(clap::Args)]
pub struct ForecastArgs {
    /// Only issues of this project (name, slug or id).
    #[arg(long, conflicts_with = "cycle")]
    project: Option<String>,
    /// A cycle's name or id: its members.
    #[arg(long)]
    cycle: Option<String>,
}

/// The sentence the app shows too; `None` dates mean too little history or nothing open.
pub fn text(f: &Forecast) -> String {
    match (f.p50, f.p85) {
        _ if f.open == 0 => "Nothing is open.".into(),
        (Some(a), Some(b)) => format!(
            "{} open. At the current pace: likely by {a}, 85% by {b} ({} finished in the last {} days).",
            f.open, f.completions, f.window_days
        ),
        _ => format!(
            "{} open. Not enough history: {} finished in the last {} days (need 10).",
            f.open, f.completions, f.window_days
        ),
    }
}

pub async fn print(db: &Db, args: &ForecastArgs, json: bool) -> Result<()> {
    let project = references::project(db, args.project.as_deref()).await?;
    let cycle = references::cycle(db, args.cycle.as_deref()).await?;
    let q = ForecastQuery {
        filter: project.map(|p| Filter::Cond {
            field: FieldRef::Fixed(FixedField::Project),
            op: Op::In,
            value: json!([p.0]),
        }),
        cycle,
    };
    let f = forecast::forecast_for(db, q).await?;
    if json {
        let v = serde_json::to_value(&f).map_err(|e| Error::Invalid(e.to_string()))?;
        println!("{v}");
    } else {
        println!("{}", text(&f));
    }
    Ok(())
}
