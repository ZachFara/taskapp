use chrono::NaiveDate;
use taskapp_core::{checklist::reset, recur::Rule};

fn d(s: &str) -> NaiveDate {
    s.parse().expect("date")
}

fn rule(s: &str) -> Rule {
    s.parse().expect("rule")
}

#[test]
fn text_forms_become_one_canonical_rule() {
    for (said, canon) in [
        ("daily", "every 1 day from due"),
        ("Every 3 days", "every 3 day from due"),
        ("weekdays", "every 1 week on mon,tue,wed,thu,fri from due"),
        ("every monday", "every 1 week on mon from due"),
        ("every thu and mon", "every 1 week on mon,thu from due"),
        ("every 2 weeks on tuesday", "every 2 week on tue from due"),
        ("weekly", "every 1 week from due"),
        ("monthly on the 15th", "every 1 month on 15 from due"),
        ("every month", "every 1 month from due"),
        ("every 2 weeks from completion", "every 2 week from done"),
        (
            "every 1 week on mon,thu from due",
            "every 1 week on mon,thu from due",
        ),
    ] {
        assert_eq!(rule(said).to_string(), canon, "{said}");
    }
}

#[test]
fn nonsense_is_refused() {
    for bad in [
        "",
        "sometimes",
        "every 0 days",
        "every 2 days on mon",
        "every week on 15",
        "every month on 32",
        "every 1000 days",
    ] {
        assert!(bad.parse::<Rule>().is_err(), "{bad}");
    }
}

#[test]
fn plain_words() {
    assert_eq!(rule("weekdays").words(), "Every weekday");
    assert_eq!(
        rule("every 2 weeks on mon,thu from completion").words(),
        "Every 2 weeks on Mon and Thu, from completion"
    );
    assert_eq!(rule("monthly on 31").words(), "Every month on day 31");
    assert_eq!(rule("daily").words(), "Every day");
}

#[test]
fn days_and_weeks_step_from_the_due_date() {
    let today = d("2026-01-01");
    assert_eq!(rule("daily").next(d("2026-02-10"), today), d("2026-02-11"));
    assert_eq!(
        rule("every 3 days").next(d("2026-02-27"), today),
        d("2026-03-02")
    );
    assert_eq!(rule("weekly").next(d("2026-12-30"), today), d("2027-01-06"));
}

#[test]
fn weekdays_skip_the_weekend() {
    let today = d("2026-01-01");
    // 2026-02-06 is a Friday.
    assert_eq!(
        rule("weekdays").next(d("2026-02-06"), today),
        d("2026-02-09")
    );
    assert_eq!(
        rule("weekdays").next(d("2026-02-04"), today),
        d("2026-02-05")
    );
}

#[test]
fn chosen_days_and_every_other_week() {
    let today = d("2026-01-01");
    let r = rule("every monday,thursday");
    assert_eq!(r.next(d("2026-02-02"), today), d("2026-02-05")); // Mon -> Thu
    assert_eq!(r.next(d("2026-02-05"), today), d("2026-02-09")); // Thu -> next Mon
    let r = rule("every 2 weeks on mon,thu");
    assert_eq!(r.next(d("2026-02-02"), today), d("2026-02-05"));
    assert_eq!(r.next(d("2026-02-05"), today), d("2026-02-16")); // skips a week
}

#[test]
fn months_clamp_at_the_month_end() {
    let today = d("2020-01-01");
    assert_eq!(
        rule("monthly").next(d("2026-01-31"), today),
        d("2026-02-28")
    );
    assert_eq!(
        rule("monthly").next(d("2024-01-31"), today),
        d("2024-02-29")
    );
    assert_eq!(
        rule("every 2 months").next(d("2026-11-15"), today),
        d("2027-01-15")
    );
    let on31 = rule("monthly on 31");
    assert_eq!(on31.next(d("2026-02-28"), today), d("2026-03-31"));
    assert_eq!(on31.next(d("2026-03-31"), today), d("2026-04-30"));
    assert_eq!(
        rule("monthly on 15").next(d("2026-12-15"), today),
        d("2027-01-15")
    );
}

#[test]
fn from_due_never_lands_on_or_before_today() {
    // Weekly, last due three weeks ago: the next one is the coming Monday slot, not an old date.
    let r = rule("every monday");
    assert_eq!(r.next(d("2026-02-02"), d("2026-02-25")), d("2026-03-02"));
    assert_eq!(
        rule("daily").next(d("2026-02-10"), d("2026-02-10")),
        d("2026-02-11")
    );
}

#[test]
fn from_completion_counts_from_today() {
    let r = rule("every 2 weeks from completion");
    assert_eq!(r.next(d("2025-06-01"), d("2026-02-25")), d("2026-03-11"));
    assert_eq!(
        rule("every 3 days from done").next(d("2020-01-01"), d("2026-02-26")),
        d("2026-03-01")
    );
}

#[test]
fn reset_unticks_task_boxes_outside_code() {
    let text = "intro [x]\n- [x] a\n  * [X] b\n- [ ] c\n```\n- [x] code\n```";
    assert_eq!(
        reset(text),
        "intro [x]\n- [ ] a\n  * [ ] b\n- [ ] c\n```\n- [x] code\n```"
    );
}
