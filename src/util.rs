pub fn ts_to_string(ts: i64) -> String {
    if ts <= 0 {
        return "\u{2014}".to_string();
    }
    let secs = ts as u64;
    let days = secs / 86400;
    let time = secs % 86400;
    let hours = time / 3600;
    let minutes = (time % 3600) / 60;
    let seconds = time % 60;

    let y = 1970 + (days / 365) as i64;
    let remaining = days % 365;
    let month_days_list = if is_leap_year(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut day_of_year = remaining;
    let mut m = 0u32;
    for (i, &days_in_m) in month_days_list.iter().enumerate() {
        if day_of_year < days_in_m as u64 {
            m = (i + 1) as u32;
            break;
        }
        day_of_year -= days_in_m as u64;
    }
    let d = (day_of_year + 1) as u32;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        y, m, d, hours, minutes, seconds
    )
}

pub fn ts_to_date(ts: i64) -> String {
    if ts <= 0 {
        return "\u{2014}".to_string();
    }
    let days = (ts as u64) / 86400;
    let y = 1970 + (days / 365) as i64;
    let remaining = days % 365;
    let month_days_list = if is_leap_year(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut day_of_year = remaining;
    let mut m = 0u32;
    for (i, &days_in_m) in month_days_list.iter().enumerate() {
        if day_of_year < days_in_m as u64 {
            m = (i + 1) as u32;
            break;
        }
        day_of_year -= days_in_m as u64;
    }
    let d = (day_of_year + 1) as u32;

    format!("{:04}-{:02}-{:02}", y, m, d)
}

pub fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ts_to_string_epoch() {
        let s = ts_to_string(0);
        assert_eq!(s, "\u{2014}");
    }

    #[test]
    fn test_ts_to_string_known() {
        let s = ts_to_string(946684800);
        assert!(s.contains("2000"));
        assert!(s.contains("00:00:00"));
    }

    #[test]
    fn test_ts_to_date() {
        let d = ts_to_date(946684800);
        assert!(d.contains("2000"));
    }
}
