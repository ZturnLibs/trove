use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Weekday};
use serde::{Deserialize, Serialize};

use super::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecurrenceFrequency {
    Daily,
    Weekdays,
    Weekly,
    Monthly,
    EveryNDays,
    EveryNWeeks,
}

impl RecurrenceFrequency {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekdays => "weekdays",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
            Self::EveryNDays => "everyNDays",
            Self::EveryNWeeks => "everyNWeeks",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceRule {
    pub version: u32,
    pub frequency: RecurrenceFrequency,
    pub interval: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weekdays: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monthday: Option<u8>,
    pub timezone: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_at: Option<String>,
}

impl RecurrenceRule {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.version != 1 {
            return Err(DomainError::Validation(
                "unsupported recurrence version".into(),
            ));
        }
        if self.interval == 0 {
            return Err(DomainError::Validation("interval must be >= 1".into()));
        }
        match self.frequency {
            RecurrenceFrequency::Weekly | RecurrenceFrequency::EveryNWeeks => {
                let days = self.weekdays.as_ref().ok_or_else(|| {
                    DomainError::Validation("weekly recurrence requires weekdays".into())
                })?;
                if days.is_empty() {
                    return Err(DomainError::Validation("weekdays cannot be empty".into()));
                }
                if days.iter().any(|d| *d < 1 || *d > 7) {
                    return Err(DomainError::Validation("weekday must be 1..=7".into()));
                }
            }
            RecurrenceFrequency::Monthly => {
                if let Some(day) = self.monthday {
                    if !(1..=31).contains(&day) {
                        return Err(DomainError::Validation("monthday must be 1..=31".into()));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, DomainError> {
        serde_json::to_string(self).map_err(|e| DomainError::Internal(e.to_string()))
    }

    pub fn from_json(raw: &str) -> Result<Self, DomainError> {
        let rule: Self = serde_json::from_str(raw)
            .map_err(|e| DomainError::Validation(format!("invalid recurrence json: {e}")))?;
        rule.validate()?;
        Ok(rule)
    }
}

/// Compute the next fire datetime after `after` (exclusive), keeping the same local time-of-day.
pub fn next_after(
    rule: &RecurrenceRule,
    after: NaiveDateTime,
) -> Result<Option<NaiveDateTime>, DomainError> {
    rule.validate()?;
    let time = after.time();
    let end = rule
        .end_at
        .as_ref()
        .map(|s| parse_end_date(s))
        .transpose()?;

    let mut cursor = after.date();
    for _ in 0..400 {
        let candidate_date = match rule.frequency {
            RecurrenceFrequency::Daily | RecurrenceFrequency::EveryNDays => {
                cursor + Duration::days(rule.interval.max(1) as i64)
            }
            RecurrenceFrequency::Weekdays => next_weekday(cursor),
            RecurrenceFrequency::Weekly | RecurrenceFrequency::EveryNWeeks => {
                next_weekly(cursor, rule)?
            }
            RecurrenceFrequency::Monthly => next_monthly(cursor, rule)?,
        };

        if candidate_date <= after.date() && rule.frequency != RecurrenceFrequency::Weekdays {
            cursor = candidate_date;
            continue;
        }

        let candidate = NaiveDateTime::new(candidate_date, time);
        if candidate <= after {
            cursor = candidate_date;
            continue;
        }
        if let Some(end_date) = end {
            if candidate_date > end_date {
                return Ok(None);
            }
        }
        return Ok(Some(candidate));
    }
    Err(DomainError::Internal(
        "failed to compute next recurrence".into(),
    ))
}

fn next_weekday(from: NaiveDate) -> NaiveDate {
    let mut date = from + Duration::days(1);
    loop {
        match date.weekday() {
            Weekday::Sat | Weekday::Sun => date += Duration::days(1),
            _ => return date,
        }
    }
}

fn next_weekly(from: NaiveDate, rule: &RecurrenceRule) -> Result<NaiveDate, DomainError> {
    let weekdays = rule.weekdays.as_ref().unwrap();
    let interval = rule.interval.max(1) as i64;
    let mut date = from + Duration::days(1);
    for _ in 0..400 {
        if weekdays.contains(&weekday_number(date.weekday())) {
            let weeks = (date - from).num_days() / 7;
            if interval == 1 || weeks % interval == 0 {
                return Ok(date);
            }
        }
        date += Duration::days(1);
    }
    Err(DomainError::Internal("weekly recurrence overflow".into()))
}

fn next_monthly(from: NaiveDate, rule: &RecurrenceRule) -> Result<NaiveDate, DomainError> {
    let day = rule.monthday.unwrap_or(from.day() as u8) as u32;
    let interval = rule.interval.max(1) as i32;
    let mut year = from.year();
    let mut month = from.month() as i32 + interval;
    while month > 12 {
        month -= 12;
        year += 1;
    }
    let max_day = last_day_of_month(year, month as u32);
    let use_day = day.min(max_day);
    NaiveDate::from_ymd_opt(year, month as u32, use_day)
        .ok_or_else(|| DomainError::Internal("invalid monthly date".into()))
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    let first_next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .unwrap();
    (first_next - Duration::days(1)).day()
}

fn weekday_number(weekday: Weekday) -> u8 {
    match weekday {
        Weekday::Mon => 1,
        Weekday::Tue => 2,
        Weekday::Wed => 3,
        Weekday::Thu => 4,
        Weekday::Fri => 5,
        Weekday::Sat => 6,
        Weekday::Sun => 7,
    }
}

fn parse_end_date(value: &str) -> Result<NaiveDate, DomainError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| DomainError::Validation("endAt must be YYYY-MM-DD".into()))
}

pub fn parse_local_datetime(value: &str) -> Result<NaiveDateTime, DomainError> {
    if let Ok(dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S") {
        return Ok(dt);
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M") {
        return Ok(dt);
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        return Ok(dt);
    }
    NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M")
        .map_err(|_| DomainError::Validation("datetime must be YYYY-MM-DDTHH:MM".into()))
}

pub fn format_local_datetime(dt: NaiveDateTime) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S").to_string()
}

pub fn local_now_naive() -> NaiveDateTime {
    let now = Local::now();
    NaiveDateTime::new(
        now.date_naive(),
        NaiveTime::from_hms_opt(now.hour(), now.minute(), now.second()).unwrap(),
    )
}

pub fn combine_date_time(date: &str, time: &str) -> Result<NaiveDateTime, DomainError> {
    let d = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| DomainError::Validation("invalid date".into()))?;
    let t = if time.len() == 5 {
        NaiveTime::parse_from_str(time, "%H:%M")
    } else {
        NaiveTime::parse_from_str(time, "%H:%M:%S")
    }
    .map_err(|_| DomainError::Validation("invalid time".into()))?;
    Ok(NaiveDateTime::new(d, t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daily_next() {
        let rule = RecurrenceRule {
            version: 1,
            frequency: RecurrenceFrequency::Daily,
            interval: 1,
            weekdays: None,
            monthday: None,
            timezone: "Asia/Shanghai".into(),
            end_at: None,
        };
        let after =
            NaiveDateTime::parse_from_str("2026-07-29T09:00:00", "%Y-%m-%dT%H:%M:%S").unwrap();
        let next = next_after(&rule, after).unwrap().unwrap();
        assert_eq!(next.to_string(), "2026-07-30 09:00:00");
    }

    #[test]
    fn weekdays_skips_weekend() {
        let rule = RecurrenceRule {
            version: 1,
            frequency: RecurrenceFrequency::Weekdays,
            interval: 1,
            weekdays: None,
            monthday: None,
            timezone: "Asia/Shanghai".into(),
            end_at: None,
        };
        // Friday
        let after =
            NaiveDateTime::parse_from_str("2026-07-31T09:00:00", "%Y-%m-%dT%H:%M:%S").unwrap();
        let next = next_after(&rule, after).unwrap().unwrap();
        assert_eq!(next.date().weekday(), Weekday::Mon);
        assert_eq!(next.to_string(), "2026-08-03 09:00:00");
    }

    #[test]
    fn monthly_clamps_day() {
        let rule = RecurrenceRule {
            version: 1,
            frequency: RecurrenceFrequency::Monthly,
            interval: 1,
            weekdays: None,
            monthday: Some(31),
            timezone: "Asia/Shanghai".into(),
            end_at: None,
        };
        let after =
            NaiveDateTime::parse_from_str("2026-01-31T08:00:00", "%Y-%m-%dT%H:%M:%S").unwrap();
        let next = next_after(&rule, after).unwrap().unwrap();
        assert_eq!(next.to_string(), "2026-02-28 08:00:00");
    }

    #[test]
    fn respects_end_at() {
        let rule = RecurrenceRule {
            version: 1,
            frequency: RecurrenceFrequency::Daily,
            interval: 1,
            weekdays: None,
            monthday: None,
            timezone: "Asia/Shanghai".into(),
            end_at: Some("2026-07-29".into()),
        };
        let after =
            NaiveDateTime::parse_from_str("2026-07-29T09:00:00", "%Y-%m-%dT%H:%M:%S").unwrap();
        assert!(next_after(&rule, after).unwrap().is_none());
    }
}
