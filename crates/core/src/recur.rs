//! Repeat rules for recurring issues. A rule is stored as its canonical text
//! (`every 1 week on mon,thu from due`), so the column stays readable and the CLI can print it.

use std::{fmt, str::FromStr};

use chrono::{Datelike, Days, Months, NaiveDate, Weekday};

use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unit {
    Day,
    Week,
    Month,
}

/// What the next date counts from: the old due date, or the day it was completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum From {
    Due,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    n: u32,
    unit: Unit,
    /// Weekly rules only; empty means the weekday of the base date.
    days: Vec<Weekday>,
    /// Monthly rules only.
    month_day: Option<u32>,
    from: From,
}

const NAMES: [(&str, Weekday); 7] = [
    ("mon", Weekday::Mon),
    ("tue", Weekday::Tue),
    ("wed", Weekday::Wed),
    ("thu", Weekday::Thu),
    ("fri", Weekday::Fri),
    ("sat", Weekday::Sat),
    ("sun", Weekday::Sun),
];
const WEEKDAYS: [Weekday; 5] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
];

fn day_name(w: Weekday) -> &'static str {
    NAMES[w.num_days_from_monday() as usize].0
}

/// A weekday from its first three letters ("monday", "Mon").
fn weekday(t: &str) -> Option<Weekday> {
    let head = t.get(..3)?;
    NAMES.iter().find(|(n, _)| *n == head).map(|(_, w)| *w)
}

fn month_end(y: i32, m: u32) -> u32 {
    let first = NaiveDate::from_ymd_opt(y, m, 1).unwrap_or(NaiveDate::MIN);
    (first + Months::new(1)).pred_opt().map_or(28, |d| d.day())
}

/// The words of a rule without its "from due" or "from completion" tail, and which one it named (due by default).
fn split_from(lower: &str) -> Option<(Vec<&str>, From)> {
    let mut toks: Vec<&str> = lower
        .split_whitespace()
        .filter(|t| !matches!(*t, "and" | "the"))
        .collect();
    let mut from = From::Due;
    if let Some(i) = toks.iter().position(|t| matches!(*t, "from" | "after")) {
        from = match toks.get(i + 1).copied()? {
            "due" => From::Due,
            "completion" | "complete" | "completed" | "done" => From::Done,
            _ => return None,
        };
        toks.drain(i..i + 2);
    }
    Some((toks, from))
}

/// A month day written "15", "15th" or "2nd".
fn day_number(t: &str) -> Option<u32> {
    t.trim_end_matches(|c: char| c.is_ascii_alphabetic())
        .parse()
        .ok()
}

impl FromStr for Rule {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        let bad = || {
            Error::Invalid(format!(
                "`{s}` is not a repeat rule (try: every monday, every 2 weeks, monthly on 15, weekdays)"
            ))
        };
        let lower = s.to_lowercase().replace(',', " ");
        let (toks, from) = split_from(&lower).ok_or_else(bad)?;
        let mut rest = toks.into_iter().peekable();
        rest.next_if_eq(&"every");
        let unit_of = |t: &str| match t {
            "day" | "days" | "daily" => Some(Unit::Day),
            "week" | "weeks" | "weekly" | "weekdays" | "weekday" => Some(Unit::Week),
            "month" | "months" | "monthly" => Some(Unit::Month),
            _ => None,
        };
        let (mut n, mut days, mut month_day) = (1u32, vec![], None);
        let head = rest.peek().copied().ok_or_else(bad)?;
        if head.starts_with("weekday") {
            days = WEEKDAYS.to_vec();
        }
        let unit = if let Ok(k) = head.parse::<u32>() {
            n = k;
            rest.next();
            rest.next().and_then(unit_of).ok_or_else(bad)?
        } else if let Some(u) = unit_of(head) {
            rest.next();
            u
        } else if weekday(head).is_some() {
            Unit::Week
        } else {
            return Err(bad());
        };
        rest.next_if_eq(&"on");
        rest.next_if_eq(&"day");
        for t in rest {
            if let Some(w) = weekday(t) {
                days.push(w);
            } else {
                month_day = Some(day_number(t).ok_or_else(bad)?);
            }
        }
        days.sort_by_key(Weekday::num_days_from_monday);
        days.dedup();
        let fits = match unit {
            Unit::Day => days.is_empty() && month_day.is_none(),
            Unit::Week => month_day.is_none(),
            Unit::Month => days.is_empty() && month_day.is_none_or(|d| (1..=31).contains(&d)),
        };
        if !fits || !(1..=365).contains(&n) {
            return Err(bad());
        }
        Ok(Rule {
            n,
            unit,
            days,
            month_day,
            from,
        })
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unit = match self.unit {
            Unit::Day => "day",
            Unit::Week => "week",
            Unit::Month => "month",
        };
        write!(f, "every {} {unit}", self.n)?;
        if !self.days.is_empty() {
            let names: Vec<_> = self.days.iter().map(|w| day_name(*w)).collect();
            write!(f, " on {}", names.join(","))?;
        }
        if let Some(d) = self.month_day {
            write!(f, " on {d}")?;
        }
        write!(
            f,
            " from {}",
            if self.from == From::Due {
                "due"
            } else {
                "done"
            }
        )
    }
}

impl Rule {
    /// The rule in plain words, for the CLI and the app.
    #[must_use]
    pub fn words(&self) -> String {
        let (n, unit) = (self.n, ["day", "week", "month"][self.unit as usize]);
        let mut out = match (n, self.days.as_slice()) {
            (1, d) if d == WEEKDAYS => "Every weekday".to_string(),
            (1, _) => format!("Every {unit}"),
            _ => format!("Every {n} {unit}s"),
        };
        if self.days != WEEKDAYS && !self.days.is_empty() {
            let names: Vec<String> = self.days.iter().map(|w| format!("{w}")).collect();
            out = format!("{out} on {}", names.join(" and "));
        }
        if let Some(d) = self.month_day {
            out = format!("{out} on day {d}");
        }
        if self.from == From::Done {
            out += ", from completion";
        }
        out
    }

    #[must_use]
    pub fn from_completion(&self) -> bool {
        self.from == From::Done
    }

    /// One occurrence after `d`.
    fn step(&self, d: NaiveDate) -> NaiveDate {
        let n = self.n;
        match self.unit {
            Unit::Day => d + Days::new(n.into()),
            Unit::Week => {
                let w = d.weekday().num_days_from_monday();
                match self
                    .days
                    .iter()
                    .map(Weekday::num_days_from_monday)
                    .find(|x| *x > w)
                {
                    _ if self.days.is_empty() => d + Days::new(u64::from(7 * n)),
                    Some(x) => d + Days::new(u64::from(x - w)),
                    None => {
                        d - Days::new(u64::from(w))
                            + Days::new(u64::from(7 * n + self.days[0].num_days_from_monday()))
                    }
                }
            }
            Unit::Month => {
                let at = |m: NaiveDate, day: u32| {
                    m.with_day(day.min(month_end(m.year(), m.month())))
                        .unwrap_or(m)
                };
                match self.month_day {
                    Some(day) => {
                        let this = at(d, day);
                        if this > d {
                            this
                        } else {
                            at(d.with_day(1).unwrap_or(d) + Months::new(n), day)
                        }
                    }
                    None => d + Months::new(n),
                }
            }
        }
    }

    /// The next due date. `base` is the old due date (today when there was none); `today` is the completion day.
    /// Counting from the due date never lands on or before today, so a late completion skips the missed slots.
    #[must_use]
    pub fn next(&self, base: NaiveDate, today: NaiveDate) -> NaiveDate {
        if self.from == From::Done {
            return self.step(today);
        }
        let mut d = self.step(base);
        while d <= today {
            d = self.step(d);
        }
        d
    }
}
