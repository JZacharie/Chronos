use rusqlite::{Connection, Result, params};
use crate::util;

pub fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub fn day_start(ts: i64) -> i64 {
    ts - (ts % 86400)
}

pub fn week_start(ts: i64) -> i64 {
    let days_from_thu = ((ts / 86400) + 4) % 7;
    day_start(ts) - (days_from_thu * 86400)
}

pub fn month_start(ts: i64) -> i64 {
    let days = ts / 86400;
    let year_month_days = days_since_epoch_to_year_month(days);
    let month_start_days = days - (year_month_days.2 as i64);
    month_start_days * 86400
}

fn days_since_epoch_to_year_month(days: i64) -> (i64, i64, i64) {
    let mut y = 1970i64;
    let mut remaining = days;
    loop {
        let days_in_year = if util::is_leap_year(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let mut day_of_year = remaining;
    for (m, days_in_m) in month_days(y).iter().enumerate() {
        if day_of_year < *days_in_m {
            return (y, (m as i64) + 1, day_of_year);
        }
        day_of_year -= days_in_m;
    }
    (y, 12, day_of_year)
}

fn month_days(year: i64) -> [i64; 12] {
    if util::is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    }
}

pub fn get_periods_in_range(
    conn: &Connection,
    start_ts: i64,
    end_ts: i64,
) -> Result<Vec<(i64, i64, bool)>> {
    let mut stmt = conn.prepare(
        "SELECT task_id, duration_seconds, is_payable FROM time_periods
         WHERE begin_time >= ?1 AND begin_time < ?2",
    )?;
    let rows = stmt.query_map(params![start_ts, end_ts], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, bool>(2)?,
        ))
    })?;
    rows.collect()
}

pub fn get_total_in_range(conn: &Connection, start_ts: i64, end_ts: i64) -> Result<i64> {
    conn.query_row(
        "SELECT COALESCE(SUM(duration_seconds), 0) FROM time_periods
         WHERE begin_time >= ?1 AND begin_time < ?2",
        params![start_ts, end_ts],
        |row| row.get(0),
    )
}

pub fn get_billable_in_range(conn: &Connection, start_ts: i64, end_ts: i64) -> Result<i64> {
    conn.query_row(
        "SELECT COALESCE(SUM(duration_seconds), 0) FROM time_periods
         WHERE begin_time >= ?1 AND begin_time < ?2 AND is_payable = 1",
        params![start_ts, end_ts],
        |row| row.get(0),
    )
}

pub fn get_all_periods_in_range(
    conn: &Connection,
    start_ts: i64,
    end_ts: i64,
) -> Result<Vec<(i64, i64, i64, i64, bool)>> {
    let mut stmt = conn.prepare(
        "SELECT tp.task_id, tp.begin_time, tp.end_time, tp.duration_seconds, tp.is_payable
         FROM time_periods tp
         WHERE tp.begin_time >= ?1 AND tp.begin_time < ?2
         ORDER BY tp.begin_time DESC
         LIMIT 500",
    )?;
    let rows = stmt.query_map(params![start_ts, end_ts], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, Option<i64>>(2)?.unwrap_or(0),
            row.get::<_, i64>(3)?,
            row.get::<_, bool>(4)?,
        ))
    })?;
    rows.collect()
}

pub fn get_all_periods_ordered(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<(i64, i64, i64, i64, bool)>> {
    let mut stmt = conn.prepare(
        "SELECT tp.task_id, tp.begin_time, tp.end_time, tp.duration_seconds, tp.is_payable
         FROM time_periods tp
         ORDER BY tp.begin_time DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit as i64], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, Option<i64>>(2)?.unwrap_or(0),
            row.get::<_, i64>(3)?,
            row.get::<_, bool>(4)?,
        ))
    })?;
    rows.collect()
}

pub struct DayStats {
    pub today: i64,
    pub yesterday: i64,
    pub this_week: i64,
    pub this_month: i64,
    pub billable_today: i64,
}

pub struct TaskReportItem {
    pub task_id: i64,
    pub task_name: String,
    pub total_secs: i64,
    pub billable_secs: i64,
}

pub fn get_task_report(
    conn: &Connection,
    start_ts: i64,
    end_ts: i64,
) -> Result<Vec<TaskReportItem>> {
    let mut stmt = conn.prepare(
        "SELECT tp.task_id, t.name, COALESCE(SUM(tp.duration_seconds), 0),
                COALESCE(SUM(CASE WHEN tp.is_payable THEN tp.duration_seconds ELSE 0 END), 0)
         FROM time_periods tp
         JOIN tasks t ON t.id = tp.task_id
         WHERE tp.begin_time >= ?1 AND tp.begin_time < ?2
         GROUP BY tp.task_id
         ORDER BY SUM(tp.duration_seconds) DESC",
    )?;
    let rows = stmt.query_map(params![start_ts, end_ts], |row| {
        Ok(TaskReportItem {
            task_id: row.get(0)?,
            task_name: row.get(1)?,
            total_secs: row.get(2)?,
            billable_secs: row.get(3)?,
        })
    })?;
    rows.collect()
}

pub fn compute_stats(conn: &Connection) -> Result<DayStats> {
    let now = now_ts();
    let today_start = day_start(now);
    let yesterday_start = today_start - 86400;
    let w_start = week_start(now);
    let m_start = month_start(now);

    Ok(DayStats {
        today: get_total_in_range(conn, today_start, now + 1)?,
        yesterday: get_total_in_range(conn, yesterday_start, today_start)?,
        this_week: get_total_in_range(conn, w_start, now + 1)?,
        this_month: get_total_in_range(conn, m_start, now + 1)?,
        billable_today: get_billable_in_range(conn, today_start, now + 1)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn setup() -> Connection {
        let conn = db::open_database(":memory:").unwrap();
        let task_id = db::create_task(&conn, None, "Test", false, true).unwrap();
        let now = now_ts();
        for i in 0..5 {
            let begin = now - (i as i64 * 3600);
            db::create_time_period(&conn, task_id, begin, Some(begin + 1800), 1800, i % 2 == 0)
                .unwrap();
        }
        conn
    }

    #[test]
    fn test_get_total_in_range() {
        let conn = setup();
        let now = now_ts();
        let total = get_total_in_range(&conn, now - 3600, now + 1).unwrap();
        assert_eq!(total, 3600);
    }

    #[test]
    fn test_get_all_periods_ordered() {
        let conn = setup();
        let periods = get_all_periods_ordered(&conn, 10).unwrap();
        assert_eq!(periods.len(), 5);
    }

    #[test]
    fn test_compute_stats() {
        let conn = setup();
        let stats = compute_stats(&conn).unwrap();
        assert!(stats.today >= 0);
    }

    #[test]
    fn test_day_start() {
        let ts = 1720000000;
        let ds = day_start(ts);
        assert_eq!(ds % 86400, 0);
        assert!(ds <= ts);
    }

    #[test]
    fn test_week_start() {
        let ts = 1720000000;
        let ws = week_start(ts);
        assert_eq!(ws % 86400, 0);
        assert!(ws <= ts);
    }
}
