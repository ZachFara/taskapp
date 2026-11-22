use chrono::{Datelike, Duration, NaiveDate, NaiveTime, TimeZone, Utc, Weekday};
use chrono_tz::Tz;

use crate::{
    ChangeSource, Db, Due, Issue, NewIssue, ParsedQuickAdd, Result, issues, labels, projects,
    templates,
};

/// Pure: parses quick-add text relative to `today` in IANA zone `tz`.
#[must_use]
pub fn parse(input: &str, today: NaiveDate, tz: &str) -> ParsedQuickAdd {
    let tokens: Vec<&str> = input.split_whitespace().collect();
    let mut parts = Parts::default();
    let mut title: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        match take(&tokens, i, today, &mut parts) {
            0 => {
                title.push(tokens[i]);
                i += 1;
            }
            n => i += n,
        }
    }
    // A time with no date is today's time; an unusable zone or a local time that
    // does not exist keeps "at <time>" in the title so nothing is lost.
    let mut due = parts.due_date.map(Due::Date);
    if let Some((time, raw)) = parts.time {
        if let Some(d) = to_due(parts.due_date.unwrap_or(today), time, tz) {
            due = Some(d);
        } else {
            title.push("at");
            title.push(raw);
        }
    }
    // An input made only of tokens keeps its whole text as the title.
    let title = if title.is_empty() {
        tokens.join(" ")
    } else {
        title.join(" ")
    };
    ParsedQuickAdd {
        title,
        due,
        start_date: parts.start,
        priority: parts.priority,
        project_slug: parts.slug,
        label_names: parts.labels,
        estimate_minutes: parts.estimate,
    }
}

/// # Errors
///
/// Fails on a database error or invalid arguments.
pub async fn create_from(
    db: &Db,
    input: &str,
    today: NaiveDate,
    tz: &str,
    src: ChangeSource,
) -> Result<Issue> {
    if let Some((slug, rest)) = templates::split_slug(input) {
        let template = templates::by_slug(db, slug).await?;
        let overrides = new_issue(db, rest, today, tz).await?;
        return Ok(templates::apply(db, template.id, overrides, src)
            .await?
            .issue);
    }
    issues::create(db, new_issue(db, input, today, tz).await?, src).await
}

/// The issue [`create_from`] would create, with project and label names resolved.
///
/// # Errors
///
/// Fails on a database error.
pub async fn new_issue(db: &Db, input: &str, today: NaiveDate, tz: &str) -> Result<NewIssue> {
    let parsed = parse(input, today, tz);
    let mut title = parsed.title;
    // Unknown project or label: its token goes back into the title, nothing is created.
    let project_id = match parsed.project_slug {
        Some(slug) => {
            let found = projects::list(db)
                .await?
                .into_iter()
                .find(|p| p.slug == slug);
            if found.is_none() {
                title.push_str(" #");
                title.push_str(&slug);
            }
            found.map(|p| p.id)
        }
        None => None,
    };
    let all_labels = labels::list(db).await?;
    let mut label_ids = Vec::new();
    for name in parsed.label_names {
        if let Some(l) = all_labels
            .iter()
            .find(|l| l.name.eq_ignore_ascii_case(&name))
        {
            label_ids.push(l.id);
        } else {
            title.push_str(" @");
            title.push_str(&name);
        }
    }
    Ok(NewIssue {
        title,
        description: None,
        status_id: None,
        priority: parsed.priority,
        project_id,
        milestone_id: None,
        parent_id: None,
        cycle_id: None,
        // None, not an empty list, so a template's labels survive when the text names none.
        label_ids: (!label_ids.is_empty()).then_some(label_ids),
        start_date: parsed.start_date,
        due: parsed.due,
        estimate_minutes: parsed.estimate_minutes,
        properties: None,
        created_at: None,
        completed_at: None,
    })
}

#[derive(Default)]
struct Parts<'a> {
    due_date: Option<NaiveDate>,
    time: Option<(NaiveTime, &'a str)>,
    start: Option<NaiveDate>,
    priority: Option<u8>,
    slug: Option<String>,
    labels: Vec<String>,
    estimate: Option<u32>,
}

/// Consumes the token(s) at `i` into `p`; returns how many were consumed (0 = plain title word).
fn take<'a>(tokens: &[&'a str], i: usize, today: NaiveDate, p: &mut Parts<'a>) -> usize {
    let raw = tokens[i];
    let w = raw.to_ascii_lowercase();
    if let Some(prio) = priority(&w) {
        p.priority = Some(prio);
        return 1;
    }
    if let Some(slug) = w.strip_prefix('#').filter(|s| !s.is_empty()) {
        p.slug = Some(slug.to_string());
        return 1;
    }
    if let Some(label) = raw.strip_prefix('@').filter(|s| !s.is_empty()) {
        p.labels.push(label.to_string());
        return 1;
    }
    if let Some(minutes) = estimate(&w) {
        p.estimate = Some(minutes);
        return 1;
    }
    match w.as_str() {
        "by" | "due" => date_at(tokens, i + 1, today).map_or(0, |(d, n)| {
            p.due_date = Some(d);
            1 + n
        }),
        "start" => date_at(tokens, i + 1, today).map_or(0, |(d, n)| {
            p.start = Some(d);
            1 + n
        }),
        "at" => tokens
            .get(i + 1)
            .and_then(|t| time_at(t).map(|tm| (tm, *t)))
            .map_or(0, |found| {
                p.time = Some(found);
                2
            }),
        _ => date_at(tokens, i, today).map_or(0, |(d, n)| {
            p.due_date = Some(d);
            n
        }),
    }
}

fn priority(w: &str) -> Option<u8> {
    let digit = w.strip_prefix('p')?;
    if digit.len() != 1 {
        return None;
    }
    digit.parse::<u8>().ok().filter(|p| *p <= 4)
}

/// `~30m`, `~2h`, `~1d` (1d = 480 minutes). Zero or overflow stays in the title.
fn estimate(w: &str) -> Option<u32> {
    let body = w.strip_prefix('~')?;
    let unit = body.chars().last()?;
    let n: u32 = body[..body.len() - unit.len_utf8()].parse().ok()?;
    let per_unit = match unit {
        'm' => 1,
        'h' => 60,
        'd' => 480,
        _ => return None,
    };
    n.checked_mul(per_unit).filter(|m| *m > 0)
}

/// A date starting at token `i`; returns the date and how many tokens it used.
fn date_at(tokens: &[&str], i: usize, today: NaiveDate) -> Option<(NaiveDate, usize)> {
    let w = tokens.get(i)?.to_ascii_lowercase();
    match w.as_str() {
        "today" => Some((today, 1)),
        "tomorrow" => Some((today.checked_add_signed(Duration::days(1))?, 1)),
        "next" if word(tokens, i + 1).as_deref() == Some("week") => {
            Some((next_weekday(today, Weekday::Mon)?, 2))
        }
        "in" => {
            let n: u32 = tokens.get(i + 1)?.parse().ok()?;
            let days = match word(tokens, i + 2)?.as_str() {
                "day" | "days" => i64::from(n),
                "week" | "weeks" => i64::from(n) * 7,
                _ => return None,
            };
            Some((today.checked_add_signed(Duration::days(days))?, 3))
        }
        _ => weekday(&w)
            .and_then(|wd| next_weekday(today, wd))
            .map(|d| (d, 1))
            .or_else(|| iso_date(&w).map(|d| (d, 1)))
            .or_else(|| month_day(tokens, i, today)),
    }
}

fn word(tokens: &[&str], i: usize) -> Option<String> {
    tokens.get(i).map(|t| t.to_ascii_lowercase())
}

fn weekday(w: &str) -> Option<Weekday> {
    match w {
        "mon" | "monday" => Some(Weekday::Mon),
        "tue" | "tuesday" => Some(Weekday::Tue),
        "wed" | "wednesday" => Some(Weekday::Wed),
        "thu" | "thursday" => Some(Weekday::Thu),
        "fri" | "friday" => Some(Weekday::Fri),
        "sat" | "saturday" => Some(Weekday::Sat),
        "sun" | "sunday" => Some(Weekday::Sun),
        _ => None,
    }
}

/// Next occurrence after `today`; today's own weekday means seven days out.
fn next_weekday(today: NaiveDate, wd: Weekday) -> Option<NaiveDate> {
    let diff = (wd.num_days_from_monday() + 7 - today.weekday().num_days_from_monday()) % 7;
    let diff = if diff == 0 { 7 } else { diff };
    today.checked_add_signed(Duration::days(i64::from(diff)))
}

fn iso_date(w: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(w, "%Y-%m-%d").ok()
}

/// `dec 3` or `3 dec`; this year, or next year if this year's date is already past.
fn month_day(tokens: &[&str], i: usize, today: NaiveDate) -> Option<(NaiveDate, usize)> {
    const MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let first = word(tokens, i)?;
    let (month_name, day) = match first.parse::<u32>() {
        Ok(day) => (word(tokens, i + 1)?, day),
        Err(_) => (first, tokens.get(i + 1)?.parse().ok()?),
    };
    let month = u32::try_from(MONTHS.iter().position(|m| *m == month_name)? + 1).ok()?;
    let used = 2;
    let this_year = NaiveDate::from_ymd_opt(today.year(), month, day)?;
    let date = if this_year < today {
        NaiveDate::from_ymd_opt(today.year() + 1, month, day)?
    } else {
        this_year
    };
    Some((date, used))
}

/// `3pm`, `9:15am`, `15:30`. A bare hour needs `am`/`pm` or a colon.
fn time_at(token: &str) -> Option<NaiveTime> {
    let s = token.to_ascii_lowercase();
    let (body, meridiem) = if let Some(b) = s.strip_suffix("am") {
        (b, Some(false))
    } else if let Some(b) = s.strip_suffix("pm") {
        (b, Some(true))
    } else {
        (s.as_str(), None)
    };
    let (h, m) = match body.split_once(':') {
        Some((h, m)) => (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?),
        None if meridiem.is_some() => (body.parse::<u32>().ok()?, 0),
        None => return None,
    };
    let hour = match meridiem {
        Some(pm) => {
            if !(1..=12).contains(&h) {
                return None;
            }
            (h % 12) + if pm { 12 } else { 0 }
        }
        None if h <= 23 => h,
        None => return None,
    };
    NaiveTime::from_hms_opt(hour, m, 0)
}

fn to_due(date: NaiveDate, time: NaiveTime, tz: &str) -> Option<Due> {
    let zone: Tz = tz.parse().ok()?;
    let local = zone.from_local_datetime(&date.and_time(time)).earliest()?;
    Some(Due::DateTime {
        utc: local.with_timezone(&Utc),
        tz: tz.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone, Utc};

    use super::*;
    use crate::Due;

    const TZ: &str = "America/New_York";

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("valid date")
    }

    // 2026-11-20 is a Friday.
    fn parse_today(input: &str) -> ParsedQuickAdd {
        parse(input, date(2026, 11, 20), TZ)
    }

    #[test]
    fn full_example() {
        let q = parse_today("renew lease by fri p1 #ohio-move");
        assert_eq!(q.title, "renew lease");
        assert_eq!(q.due, Some(Due::Date(date(2026, 11, 27))));
        assert_eq!(q.priority, Some(1));
        assert_eq!(q.project_slug.as_deref(), Some("ohio-move"));
    }

    #[test]
    fn friday_on_friday_is_next_week() {
        let q = parse_today("file taxes by friday");
        assert_eq!(q.due, Some(Due::Date(date(2026, 11, 27))));
        assert_eq!(q.title, "file taxes");
    }

    #[test]
    fn time_makes_datetime() {
        let q = parse_today("call bank tomorrow at 3pm");
        let utc = Utc
            .with_ymd_and_hms(2026, 11, 21, 20, 0, 0)
            .single()
            .expect("valid utc");
        assert_eq!(
            q.due,
            Some(Due::DateTime {
                utc,
                tz: TZ.to_string()
            })
        );
        assert_eq!(q.title, "call bank");
    }

    #[test]
    fn estimate_units() {
        assert_eq!(parse_today("task ~30m").estimate_minutes, Some(30));
        assert_eq!(parse_today("task ~2h").estimate_minutes, Some(120));
        assert_eq!(parse_today("task ~1d").estimate_minutes, Some(480));
    }

    #[test]
    fn month_day_rolls_to_next_year() {
        assert_eq!(
            parse_today("file jan 5").due,
            Some(Due::Date(date(2027, 1, 5)))
        );
        assert_eq!(
            parse_today("file dec 3").due,
            Some(Due::Date(date(2026, 12, 3)))
        );
        assert_eq!(
            parse_today("file 3 dec").due,
            Some(Due::Date(date(2026, 12, 3)))
        );
    }

    #[test]
    fn labels_and_start() {
        let q = parse_today("write doc @work @home start tomorrow");
        assert_eq!(q.title, "write doc");
        assert_eq!(q.label_names, vec!["work", "home"]);
        assert_eq!(q.start_date, Some(date(2026, 11, 21)));
        assert_eq!(q.due, None);
    }

    #[test]
    fn no_tokens_whole_title() {
        let q = parse_today("buy   milk eggs");
        assert_eq!(q.title, "buy milk eggs");
        assert_eq!(q.due, None);
        assert_eq!(q.priority, None);
        assert_eq!(q.project_slug, None);
        assert_eq!(q.label_names, Vec::<String>::new());
        assert_eq!(q.estimate_minutes, None);
    }

    #[test]
    fn p5_and_bare_p_stay_in_title() {
        let q = parse_today("fix sink p5 p");
        assert_eq!(q.title, "fix sink p5 p");
        assert_eq!(q.priority, None);
    }

    #[test]
    fn bare_hash_stays_in_title() {
        let q = parse_today("issue # 4");
        assert_eq!(q.title, "issue # 4");
        assert_eq!(q.project_slug, None);
    }

    #[test]
    fn by_without_date_stays_in_title() {
        let q = parse_today("pay by");
        assert_eq!(q.title, "pay by");
        assert_eq!(q.due, None);
    }

    #[test]
    fn at_invalid_time_stays_in_title() {
        let q = parse_today("call at 25:00");
        assert_eq!(q.title, "call at 25:00");
        assert_eq!(q.due, None);
    }

    #[tokio::test]
    async fn unknown_project_kept_in_title() {
        let db = Db::open_in_memory().await.expect("in-memory db");
        let issue = create_from(
            &db,
            "fix sink #nope",
            date(2026, 11, 20),
            TZ,
            ChangeSource::Ui,
        )
        .await
        .expect("create_from");
        assert!(issue.title.contains("#nope"));
        assert_eq!(issue.project_id, None);
    }
}
