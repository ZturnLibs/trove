use chrono::{Datelike, Duration, Local, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

use super::{RecurrenceFrequency, RecurrenceRule, TaskPriority};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ParsedCapture {
    pub title: String,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub priority: TaskPriority,
    pub recurrence: Option<RecurrenceRule>,
    /// Fields inferred with lower confidence; UI should highlight for confirmation.
    pub ambiguous_fields: Vec<String>,
    pub raw: String,
}

/// Deterministic quick-capture parser. Does not silently invent ambiguous dates/times.
pub fn parse_capture(input: &str) -> ParsedCapture {
    let raw = input.trim().to_string();
    let mut remaining = raw.clone();
    let mut due_date: Option<String> = None;
    let mut due_time: Option<String> = None;
    let mut priority = TaskPriority::None;
    let mut recurrence: Option<RecurrenceRule> = None;
    let mut ambiguous_fields = Vec::new();
    let timezone = "Asia/Shanghai".to_string();

    for (pat, pri) in [
        ("优先级高", TaskPriority::High),
        ("优先级中", TaskPriority::Medium),
        ("优先级低", TaskPriority::Low),
        ("!高", TaskPriority::High),
        ("!中", TaskPriority::Medium),
        ("!低", TaskPriority::Low),
        ("p1", TaskPriority::High),
        ("p2", TaskPriority::Medium),
        ("p3", TaskPriority::Low),
    ] {
        if let Some(next) = strip_token(&remaining, pat) {
            remaining = next;
            priority = pri;
            break;
        }
    }

    if let Some(next) = strip_token(&remaining, "每天") {
        remaining = next;
        recurrence = Some(daily_rule(&timezone));
    } else if let Some(next) = strip_token(&remaining, "工作日") {
        remaining = next;
        recurrence = Some(RecurrenceRule {
            version: 1,
            frequency: RecurrenceFrequency::Weekdays,
            interval: 1,
            weekdays: None,
            monthday: None,
            timezone: timezone.clone(),
            end_at: None,
        });
    } else if let Some((next, weekday)) = take_weekly(&remaining) {
        remaining = next;
        recurrence = Some(RecurrenceRule {
            version: 1,
            frequency: RecurrenceFrequency::Weekly,
            interval: 1,
            weekdays: Some(vec![weekday]),
            monthday: None,
            timezone: timezone.clone(),
            end_at: None,
        });
    } else if let Some((next, day)) = take_monthly(&remaining) {
        remaining = next;
        recurrence = Some(RecurrenceRule {
            version: 1,
            frequency: RecurrenceFrequency::Monthly,
            interval: 1,
            weekdays: None,
            monthday: Some(day),
            timezone: timezone.clone(),
            end_at: None,
        });
    }

    if let Some((next, date, amb)) = take_date(&remaining) {
        remaining = next;
        due_date = Some(date);
        if amb {
            ambiguous_fields.push("dueDate".into());
        }
    }

    if let Some((next, time, amb)) = take_time(&remaining) {
        remaining = next;
        due_time = Some(time);
        if amb {
            ambiguous_fields.push("dueTime".into());
        }
    }

    if recurrence.is_some() && due_date.is_none() {
        due_date = Some(Local::now().date_naive().format("%Y-%m-%d").to_string());
    }

    let title = cleanup_title(&remaining);
    ParsedCapture {
        title: if title.is_empty() { raw.clone() } else { title },
        due_date,
        due_time,
        priority,
        recurrence,
        ambiguous_fields,
        raw,
    }
}

fn daily_rule(timezone: &str) -> RecurrenceRule {
    RecurrenceRule {
        version: 1,
        frequency: RecurrenceFrequency::Daily,
        interval: 1,
        weekdays: None,
        monthday: None,
        timezone: timezone.into(),
        end_at: None,
    }
}

fn cleanup_title(input: &str) -> String {
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .trim_matches(|c| "，,。.!！".contains(c))
        .to_string()
}

fn strip_token(input: &str, token: &str) -> Option<String> {
    input.find(token).map(|idx| {
        let mut out = String::new();
        out.push_str(&input[..idx]);
        out.push_str(&input[idx + token.len()..]);
        out
    })
}

fn take_date(input: &str) -> Option<(String, String, bool)> {
    let today = Local::now().date_naive();
    for (token, date, amb) in [
        ("今天", today, false),
        ("明天", today + Duration::days(1), false),
        ("后天", today + Duration::days(2), false),
    ] {
        if let Some(next) = strip_token(input, token) {
            return Some((next, date.format("%Y-%m-%d").to_string(), amb));
        }
    }

    for (token, wd) in [
        ("下周一", Weekday::Mon),
        ("下周二", Weekday::Tue),
        ("下周三", Weekday::Wed),
        ("下周四", Weekday::Thu),
        ("下周五", Weekday::Fri),
        ("下周六", Weekday::Sat),
        ("下周日", Weekday::Sun),
        ("下星期天", Weekday::Sun),
    ] {
        if let Some(next) = strip_token(input, token) {
            let date = next_weekday(today, wd, true);
            return Some((next, date.format("%Y-%m-%d").to_string(), false));
        }
    }

    for (token, wd) in [
        ("星期一", Weekday::Mon),
        ("星期二", Weekday::Tue),
        ("星期三", Weekday::Wed),
        ("星期四", Weekday::Thu),
        ("星期五", Weekday::Fri),
        ("星期六", Weekday::Sat),
        ("星期天", Weekday::Sun),
        ("周一", Weekday::Mon),
        ("周二", Weekday::Tue),
        ("周三", Weekday::Wed),
        ("周四", Weekday::Thu),
        ("周五", Weekday::Fri),
        ("周六", Weekday::Sat),
        ("周日", Weekday::Sun),
    ] {
        if let Some(next) = strip_token(input, token) {
            let date = next_weekday(today, wd, false);
            return Some((next, date.format("%Y-%m-%d").to_string(), true));
        }
    }

    None
}

fn next_weekday(from: NaiveDate, target: Weekday, force_next_week: bool) -> NaiveDate {
    if force_next_week {
        let days_until_next_monday = {
            let from_mon = from.weekday().num_days_from_monday() as i64;
            if from_mon == 0 {
                7
            } else {
                7 - from_mon
            }
        };
        let next_monday = from + Duration::days(days_until_next_monday);
        let mut d = next_monday;
        for _ in 0..7 {
            if d.weekday() == target {
                return d;
            }
            d += Duration::days(1);
        }
        return next_monday;
    }

    let mut d = from;
    for _ in 0..7 {
        if d.weekday() == target {
            return d;
        }
        d += Duration::days(1);
    }
    from + Duration::days(7)
}

fn take_time(input: &str) -> Option<(String, String, bool)> {
    if let Some((start, end)) = find_hm(input) {
        let token = &input[start..end];
        let parts: Vec<_> = token.split(':').collect();
        if parts.len() == 2 {
            if let (Ok(h), Ok(m)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                if h < 24 && m < 60 {
                    let mut next = String::new();
                    next.push_str(&input[..start]);
                    next.push_str(&input[end..]);
                    return Some((next, format!("{h:02}:{m:02}"), false));
                }
            }
        }
    }

    for (period, add12) in [
        ("下午", true),
        ("晚上", true),
        ("傍晚", true),
        ("上午", false),
        ("早上", false),
        ("中午", false),
        ("凌晨", false),
    ] {
        if let Some(idx) = input.find(period) {
            let after = &input[idx + period.len()..];
            if let Some((consumed, mut hour, minute)) = parse_chinese_clock(after) {
                if add12 && hour < 12 {
                    hour += 12;
                } else if period == "中午" && hour < 12 && hour != 12 {
                    hour = 12;
                }
                let mut next = String::new();
                next.push_str(&input[..idx]);
                next.push_str(&after[consumed..]);
                return Some((next, format!("{hour:02}:{minute:02}"), false));
            }
        }
    }

    None
}

fn find_hm(input: &str) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    for i in 0..bytes.len() {
        if !bytes[i].is_ascii_digit() {
            continue;
        }
        let mut j = i;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j >= bytes.len() || bytes[j] != b':' {
            continue;
        }
        let mut k = j + 1;
        while k < bytes.len() && bytes[k].is_ascii_digit() {
            k += 1;
        }
        if k > j + 1 {
            return Some((i, k));
        }
    }
    None
}

fn parse_chinese_clock(after: &str) -> Option<(usize, u32, u32)> {
    let patterns = [
        ("十二点半", 12, 30),
        ("十一点半", 11, 30),
        ("十点半", 10, 30),
        ("九点半", 9, 30),
        ("八点半", 8, 30),
        ("七点半", 7, 30),
        ("六点半", 6, 30),
        ("五点半", 5, 30),
        ("四点半", 4, 30),
        ("三点半", 3, 30),
        ("两点半", 2, 30),
        ("二点半", 2, 30),
        ("一点半", 1, 30),
        ("十二点", 12, 0),
        ("十一点", 11, 0),
        ("十点", 10, 0),
        ("九点", 9, 0),
        ("八点", 8, 0),
        ("七点", 7, 0),
        ("六点", 6, 0),
        ("五点", 5, 0),
        ("四点", 4, 0),
        ("三点", 3, 0),
        ("两点", 2, 0),
        ("二点", 2, 0),
        ("一点", 1, 0),
    ];
    for (pat, h, m) in patterns {
        if after.starts_with(pat) {
            return Some((pat.len(), h, m));
        }
    }
    None
}

fn take_weekly(input: &str) -> Option<(String, u8)> {
    // RecurrenceRule weekdays use 1..=7 (Mon..Sun).
    for (token, wd) in [
        ("每周一", 1u8),
        ("每周二", 2),
        ("每周三", 3),
        ("每周四", 4),
        ("每周五", 5),
        ("每周六", 6),
        ("每周日", 7),
        ("每周天", 7),
    ] {
        if let Some(next) = strip_token(input, token) {
            return Some((next, wd));
        }
    }
    None
}

fn take_monthly(input: &str) -> Option<(String, u8)> {
    let Some(idx) = input.find("每月") else {
        return None;
    };
    let after = &input[idx + "每月".len()..];
    let mut digits = String::new();
    for ch in after.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else {
            break;
        }
    }
    if digits.is_empty() {
        return None;
    }
    let rest = &after[digits.len()..];
    let suffix = rest.chars().next()?;
    if suffix != '日' && suffix != '号' {
        return None;
    }
    let day: u8 = digits.parse().ok()?;
    if !(1..=31).contains(&day) {
        return None;
    }
    let consume = "每月".len() + digits.len() + suffix.len_utf8();
    let mut next = String::new();
    next.push_str(&input[..idx]);
    next.push_str(&input[idx + consume..]);
    Some((next, day))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tomorrow_afternoon() {
        let p = parse_capture("明天下午三点回复客户");
        assert!(p.title.contains("回复客户"));
        assert!(p.due_date.is_some());
        assert_eq!(p.due_time.as_deref(), Some("15:00"));
        assert!(p.ambiguous_fields.is_empty());
    }

    #[test]
    fn bare_weekday_is_ambiguous() {
        let p = parse_capture("周五交周报");
        assert!(p.due_date.is_some());
        assert!(p.ambiguous_fields.iter().any(|f| f == "dueDate"));
        assert!(p.title.contains("交周报"));
    }

    #[test]
    fn daily_recurrence() {
        let p = parse_capture("每天吃药");
        assert!(p.recurrence.is_some());
        assert_eq!(
            p.recurrence.as_ref().unwrap().frequency,
            RecurrenceFrequency::Daily
        );
        assert!(p.title.contains("吃药"));
    }

    #[test]
    fn priority_marker() {
        let p = parse_capture("明天 !高 提交方案");
        assert_eq!(p.priority, TaskPriority::High);
        assert!(p.title.contains("提交方案"));
    }

    #[test]
    fn hhmm_time() {
        let p = parse_capture("明天 15:30 开会");
        assert_eq!(p.due_time.as_deref(), Some("15:30"));
        assert!(p.title.contains("开会"));
    }
}
