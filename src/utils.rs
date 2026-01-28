use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc, Datelike};

pub fn parse_shortcut_date(s: &str) -> Option<NaiveDate> {
    let s = s.to_lowercase();
    let now = Local::now().date_naive();
    
    match s.as_str() {
        "today" | "t" => return Some(now),
        "tomorrow" | "tm" => return Some(now + Duration::days(1)),
        _ => {}
    }

    // [N]d, [N]w
    if s.ends_with('d') {
        if let Ok(n) = s[..s.len()-1].parse::<i64>() {
            return Some(now + Duration::days(n));
        }
    }
    if s.ends_with('w') {
        if let Ok(n) = s[..s.len()-1].parse::<i64>() {
            return Some(now + Duration::days(n * 7));
        }
    }

    // mon, tue, wed, thu, fri, sat, sun
    let target_weekday = match s.as_str() {
        "mon" => Some(chrono::Weekday::Mon),
        "tue" => Some(chrono::Weekday::Tue),
        "wed" => Some(chrono::Weekday::Wed),
        "thu" => Some(chrono::Weekday::Thu),
        "fri" => Some(chrono::Weekday::Fri),
        "sat" => Some(chrono::Weekday::Sat),
        "sun" => Some(chrono::Weekday::Sun),
        _ => None,
    };

    if let Some(target) = target_weekday {
        let mut date = now + Duration::days(1);
        while date.weekday() != target {
            date += Duration::days(1);
        }
        return Some(date);
    }

    None
}

pub fn parse_shortcut_time(s: &str) -> Option<NaiveTime> {
    let s = s.to_lowercase();
    match s.as_str() {
        "last" => return Some(NaiveTime::from_hms_opt(23, 59, 0).unwrap()),
        "morning" => return Some(NaiveTime::from_hms_opt(9, 0, 0).unwrap()),
        "noon" => return Some(NaiveTime::from_hms_opt(12, 0, 0).unwrap()),
        "evening" => return Some(NaiveTime::from_hms_opt(18, 0, 0).unwrap()),
        "night" => return Some(NaiveTime::from_hms_opt(21, 0, 0).unwrap()),
        _ => {}
    }

    // [N]h
    if s.ends_with('h') {
        if let Ok(n) = s[..s.len()-1].parse::<i64>() {
            let now = Local::now();
            let target = now + Duration::hours(n);
            return Some(target.time());
        }
    }

    None
}

pub fn parse_full_date_time(date_str: &str, time_str: &str) -> Option<DateTime<Utc>> {
    if date_str.trim().is_empty() {
        return None;
    }

    let date = parse_shortcut_date(date_str)
        .or_else(|| NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok())?;

    let time = if time_str.trim().is_empty() {
        NaiveTime::from_hms_opt(23, 59, 0).unwrap()
    } else {
        parse_shortcut_time(time_str)
            .or_else(|| NaiveTime::parse_from_str(time_str, "%H:%M").ok())
            .unwrap_or_else(|| NaiveTime::from_hms_opt(23, 59, 0).unwrap())
    };

    let dt = NaiveDateTime::new(date, time);
    Local.from_local_datetime(&dt).single().map(|dt| dt.with_timezone(&Utc))
}
