use std::collections::BTreeSet;

use chrono::{Datelike, Duration, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BusinessDayRule {
    Exact,
    PriorBusinessDay,
    NextBusinessDay,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentWindowRule {
    /// The bill may be paid any time from the planning date through the due date.
    Anytime,
    /// Example: earliest_days_before=10 and latest_days_before=0 means the
    /// payment may occur from ten days before the due date through the due date.
    DaysBeforeDue {
        earliest_days_before: u32,
        latest_days_before: u32,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum PayScheduleRule {
    Weekly { anchor: NaiveDate },
    Biweekly { anchor: NaiveDate },
    SemiMonthly { first_day: u32, second_day: u32 },
    Monthly { day: u32 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct GeneratedBillDate {
    pub due_date: NaiveDate,
    pub earliest_payment_date: NaiveDate,
    pub latest_payment_date: NaiveDate,
}

pub fn is_business_day(date: NaiveDate, holidays: &BTreeSet<NaiveDate>) -> bool {
    !matches!(date.weekday(), Weekday::Sat | Weekday::Sun) && !holidays.contains(&date)
}

pub fn adjust_business_day(
    mut date: NaiveDate,
    rule: BusinessDayRule,
    holidays: &BTreeSet<NaiveDate>,
) -> NaiveDate {
    match rule {
        BusinessDayRule::Exact => date,
        BusinessDayRule::PriorBusinessDay => {
            while !is_business_day(date, holidays) {
                date -= Duration::days(1);
            }
            date
        }
        BusinessDayRule::NextBusinessDay => {
            while !is_business_day(date, holidays) {
                date += Duration::days(1);
            }
            date
        }
    }
}

pub fn prior_business_day(date: NaiveDate) -> NaiveDate {
    adjust_business_day(date, BusinessDayRule::PriorBusinessDay, &BTreeSet::new())
}

pub fn clamped_month_date(year: i32, month: u32, requested_day: u32) -> NaiveDate {
    let requested_day = requested_day.max(1);
    if let Some(date) = NaiveDate::from_ymd_opt(year, month, requested_day) {
        return date;
    }

    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let first_next = NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .expect("valid first day of next month");
    first_next - Duration::days(1)
}

pub fn payment_window(
    planning_date: NaiveDate,
    due_date: NaiveDate,
    rule: PaymentWindowRule,
) -> (NaiveDate, NaiveDate) {
    match rule {
        PaymentWindowRule::Anytime => (planning_date.min(due_date), due_date),
        PaymentWindowRule::DaysBeforeDue {
            earliest_days_before,
            latest_days_before,
        } => {
            let earliest_days_before = earliest_days_before.max(latest_days_before);
            let earliest = due_date - Duration::days(i64::from(earliest_days_before));
            let latest = due_date - Duration::days(i64::from(latest_days_before));
            (earliest, latest)
        }
    }
}

pub fn generate_monthly_bill_dates(
    planning_date: NaiveDate,
    through_date: NaiveDate,
    day_of_month: u32,
    business_day_rule: BusinessDayRule,
    payment_window_rule: PaymentWindowRule,
    holidays: &BTreeSet<NaiveDate>,
) -> Vec<GeneratedBillDate> {
    if through_date < planning_date {
        return Vec::new();
    }

    let mut year = planning_date.year();
    let mut month = planning_date.month();
    let mut result = Vec::new();

    loop {
        let raw_due = clamped_month_date(year, month, day_of_month);
        let pay_by_date = adjust_business_day(raw_due, business_day_rule, holidays);
        // Keep the contractual due date separate from the conservative planning
        // deadline. A bill due Sunday Sep 20 is still due Sep 20; the scheduler
        // may plan to have it funded/paid by Friday Sep 18.
        if raw_due >= planning_date && raw_due <= through_date {
            let (earliest, _) = payment_window(planning_date, pay_by_date, payment_window_rule);
            result.push(GeneratedBillDate {
                due_date: raw_due,
                earliest_payment_date: earliest,
                latest_payment_date: pay_by_date,
            });
        }

        if year > through_date.year()
            || (year == through_date.year() && month >= through_date.month())
        {
            break;
        }

        if month == 12 {
            year += 1;
            month = 1;
        } else {
            month += 1;
        }
    }

    result
}

fn generate_anchored_dates(
    anchor: NaiveDate,
    step_days: i64,
    window_start: NaiveDate,
    window_end: NaiveDate,
    business_day_rule: BusinessDayRule,
    holidays: &BTreeSet<NaiveDate>,
) -> Vec<NaiveDate> {
    if window_end < window_start {
        return Vec::new();
    }

    let mut date = anchor;
    while date < window_start {
        date += Duration::days(step_days);
    }

    let mut dates = Vec::new();
    while date <= window_end {
        let adjusted = adjust_business_day(date, business_day_rule, holidays);
        if adjusted >= window_start && adjusted <= window_end {
            dates.push(adjusted);
        }
        date += Duration::days(step_days);
    }
    dates.sort_unstable();
    dates.dedup();
    dates
}

pub fn generate_pay_dates(
    rule: PayScheduleRule,
    window_start: NaiveDate,
    window_end: NaiveDate,
    business_day_rule: BusinessDayRule,
    holidays: &BTreeSet<NaiveDate>,
) -> Vec<NaiveDate> {
    match rule {
        PayScheduleRule::Weekly { anchor } => generate_anchored_dates(
            anchor,
            7,
            window_start,
            window_end,
            business_day_rule,
            holidays,
        ),
        PayScheduleRule::Biweekly { anchor } => generate_anchored_dates(
            anchor,
            14,
            window_start,
            window_end,
            business_day_rule,
            holidays,
        ),
        PayScheduleRule::Monthly { day } => {
            generate_monthly_like_dates(day, None, window_start, window_end, business_day_rule, holidays)
        }
        PayScheduleRule::SemiMonthly {
            first_day,
            second_day,
        } => generate_monthly_like_dates(
            first_day,
            Some(second_day),
            window_start,
            window_end,
            business_day_rule,
            holidays,
        ),
    }
}

fn generate_monthly_like_dates(
    first_day: u32,
    second_day: Option<u32>,
    window_start: NaiveDate,
    window_end: NaiveDate,
    business_day_rule: BusinessDayRule,
    holidays: &BTreeSet<NaiveDate>,
) -> Vec<NaiveDate> {
    if window_end < window_start {
        return Vec::new();
    }

    let mut year = window_start.year();
    let mut month = window_start.month();
    let mut result = Vec::new();

    loop {
        for day in [Some(first_day), second_day].into_iter().flatten() {
            let raw = clamped_month_date(year, month, day);
            let adjusted = adjust_business_day(raw, business_day_rule, holidays);
            if adjusted >= window_start && adjusted <= window_end {
                result.push(adjusted);
            }
        }

        if year > window_end.year() || (year == window_end.year() && month >= window_end.month()) {
            break;
        }
        if month == 12 {
            year += 1;
            month = 1;
        } else {
            month += 1;
        }
    }

    result.sort_unstable();
    result.dedup();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(value: &str) -> NaiveDate {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn weekend_date_moves_to_prior_business_day() {
        assert_eq!(prior_business_day(d("2026-08-16")), d("2026-08-14"));
    }

    #[test]
    fn configured_holiday_is_skipped_too() {
        let holidays = BTreeSet::from([d("2026-12-25")]);
        assert_eq!(
            adjust_business_day(
                d("2026-12-25"),
                BusinessDayRule::PriorBusinessDay,
                &holidays
            ),
            d("2026-12-24")
        );
    }

    #[test]
    fn day_31_clamps_to_end_of_short_month() {
        assert_eq!(clamped_month_date(2026, 2, 31), d("2026-02-28"));
    }

    #[test]
    fn monthly_occurrence_override_window_is_deterministic() {
        let dates = generate_monthly_bill_dates(
            d("2026-08-01"),
            d("2026-09-30"),
            18,
            BusinessDayRule::PriorBusinessDay,
            PaymentWindowRule::DaysBeforeDue {
                earliest_days_before: 10,
                latest_days_before: 0,
            },
            &BTreeSet::new(),
        );
        assert_eq!(dates.len(), 2);
        assert_eq!(dates[0].due_date, d("2026-08-18"));
        assert_eq!(dates[0].earliest_payment_date, d("2026-08-08"));
    }


    #[test]
    fn monthly_bill_keeps_contract_due_date_when_weekend_moves_pay_by_date() {
        let dates = generate_monthly_bill_dates(
            d("2026-09-01"),
            d("2026-09-30"),
            20,
            BusinessDayRule::PriorBusinessDay,
            PaymentWindowRule::DaysBeforeDue {
                earliest_days_before: 31,
                latest_days_before: 0,
            },
            &BTreeSet::new(),
        );
        assert_eq!(dates.len(), 1);
        assert_eq!(dates[0].due_date, d("2026-09-20"));
        assert_eq!(dates[0].latest_payment_date, d("2026-09-18"));
    }

    #[test]
    fn biweekly_paycheck_generation_uses_anchor() {
        let dates = generate_pay_dates(
            PayScheduleRule::Biweekly {
                anchor: d("2026-08-14"),
            },
            d("2026-08-01"),
            d("2026-09-15"),
            BusinessDayRule::Exact,
            &BTreeSet::new(),
        );
        assert_eq!(dates, vec![d("2026-08-14"), d("2026-08-28"), d("2026-09-11")]);
    }

    #[test]
    fn semi_monthly_schedule_clamps_and_adjusts() {
        let dates = generate_pay_dates(
            PayScheduleRule::SemiMonthly {
                first_day: 15,
                second_day: 31,
            },
            d("2026-02-01"),
            d("2026-02-28"),
            BusinessDayRule::PriorBusinessDay,
            &BTreeSet::new(),
        );
        assert_eq!(dates, vec![d("2026-02-13"), d("2026-02-27")]);
    }
}
