//! When will the open issues be done? A Monte Carlo over the recent daily completions: each simulated day
//! draws one of the last [`WINDOW`] days at random (quiet days included) and finishes that many issues.

use chrono::{Duration, NaiveDate};

use crate::{
    CycleId, Db, Error, Forecast, ForecastQuery, Result, StatusCategory,
    metrics::{self, Facts},
};

const WINDOW: i64 = 56;
const RUNS: usize = 1000;
/// Fewer finished issues than this in the window is too little to forecast from.
const MIN_DONE: u32 = 10;
/// A run that has not finished in ten years is cut off (cannot happen with `MIN_DONE` completions).
const MAX_DAYS: u32 = 3650;
const SEED: u32 = 0x00C0_FFEE;

/// mulberry32; the UI mock carries the same code so the contract cases agree.
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_add(0x6D2B_79F5);
        let mut t = self.0;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 0x3D));
        t ^ (t >> 14)
    }

    /// An index below `n` (at most 56, so the product cannot overflow).
    fn below(&mut self, n: usize) -> usize {
        usize::try_from((u64::from(self.next()) * n as u64) >> 32).unwrap_or(0)
    }
}

/// Days until `open` issues are done, one entry per run, sorted.
fn simulate(per_day: &[u32], open: u32) -> Vec<u32> {
    let mut rng = Rng(SEED);
    let mut runs: Vec<u32> = (0..RUNS)
        .map(|_| {
            let (mut left, mut days) = (open, 0);
            while left > 0 && days < MAX_DAYS {
                left = left.saturating_sub(per_day[rng.below(per_day.len())]);
                days += 1;
            }
            days
        })
        .collect();
    runs.sort_unstable();
    runs
}

fn forecast(all: &[Facts], cycle: Option<CycleId>, today: NaiveDate) -> Forecast {
    let mut per_day = vec![0u32; usize::try_from(WINDOW).unwrap_or(0)];
    for d in all.iter().filter_map(|f| f.done) {
        let age = (today - d).num_days();
        if (0..WINDOW).contains(&age) {
            per_day[usize::try_from(age).unwrap_or(0)] += 1;
        }
    }
    let completions = per_day.iter().sum();
    let open = u32::try_from(
        all.iter()
            .filter(|f| cycle.is_none_or(|c| f.issue.cycle_id == Some(c)))
            .filter(|f| {
                matches!(
                    f.at(today),
                    Some((
                        StatusCategory::Backlog
                            | StatusCategory::Unstarted
                            | StatusCategory::Started,
                        _
                    ))
                )
            })
            .count(),
    )
    .unwrap_or(u32::MAX);
    let enough = completions >= MIN_DONE;
    let runs = if enough && open > 0 {
        simulate(&per_day, open)
    } else {
        vec![]
    };
    // Nearest rank, like the metrics percentiles.
    let on = |pct: usize| {
        runs.get((runs.len() * pct).div_ceil(100).max(1) - 1)
            .map(|d| today + Duration::days(i64::from(*d)))
    };
    Forecast {
        open,
        completions,
        window_days: u32::try_from(WINDOW).unwrap_or(0),
        enough,
        p50: on(50),
        p85: on(85),
    }
}

/// [`forecast_for`] with the clock given, so a test can fix "today".
///
/// # Errors
/// `NotFound` for an unknown cycle.
pub async fn forecast_on(db: &Db, q: ForecastQuery, today: NaiveDate) -> Result<Forecast> {
    if let Some(id) = q.cycle
        && !crate::cycles::list(db).await?.iter().any(|c| c.id == id)
    {
        return Err(Error::NotFound(format!("cycle {}", id.0)));
    }
    let all = metrics::facts_for(db, q.filter).await?;
    Ok(forecast(&all, q.cycle, today))
}

/// The forecast for `q`, as of now.
///
/// # Errors
/// As [`forecast_on`].
pub async fn forecast_for(db: &Db, q: ForecastQuery) -> Result<Forecast> {
    forecast_on(db, q, metrics::today(db).await?).await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The same three numbers come out of the UI mock's JavaScript copy (checked by hand with node).
    #[test]
    fn rng_matches_the_mock() {
        let mut r = Rng(SEED);
        assert_eq!(
            [r.next(), r.next(), r.next()],
            [90_802_087, 2_860_920_651, 3_349_951_739]
        );
    }

    #[test]
    fn one_issue_a_day_finishes_in_a_day_per_issue() {
        let runs = simulate(&[1; 56], 3);
        assert!(runs.iter().all(|d| *d == 3));
    }
}
