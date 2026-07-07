use std::fs::File;
use std::io::Write;

use crate::app::format_duration;
use crate::db;
use crate::stats;
use crate::util;

pub fn export_csv(path: &str, conn: &rusqlite::Connection) -> Result<(), String> {
    let mut file = File::create(path).map_err(|e| format!("Cannot create file: {e}"))?;

    writeln!(
        file,
        "Task,Start Time,End Time,Duration (seconds),Duration (formatted),Billable"
    )
    .map_err(|e| format!("Write error: {e}"))?;

    let periods =
        stats::get_all_periods_ordered(conn, 10000).map_err(|e| format!("DB error: {e}"))?;

    for (_pid, task_id, begin, end, dur, paid) in &periods {
        let task_name = db::get_task(conn, *task_id)
            .ok()
            .flatten()
            .map(|t| escape_csv(&t.name))
            .unwrap_or_else(|| format!("#{task_id}"));

        let start_str = util::ts_to_string(*begin);
        let end_str = util::ts_to_string(*end);
        let dur_fmt = format_duration(*dur as u64);
        let paid_str = if *paid { "Yes" } else { "No" };

        writeln!(
            file,
            "{task_name},{start_str},{end_str},{dur},{dur_fmt},{paid_str}"
        )
        .map_err(|e| format!("Write error: {e}"))?;
    }

    Ok(())
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn test_export_csv() {
        let conn = db::open_database(":memory:").unwrap();
        let task_id = db::create_task(&conn, None, "Test Task", false, true).unwrap();
        db::create_time_period(&conn, task_id, 1000, Some(2000), 1000, true).unwrap();
        db::create_time_period(&conn, task_id, 2000, Some(2500), 500, false).unwrap();

        let temp_dir = std::env::temp_dir();
        let path_buf = temp_dir.join("test_chronos_export.csv");
        let path = path_buf.to_str().unwrap();
        export_csv(path, &conn).unwrap();

        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("Test Task"));
        assert!(content.contains("1000"));
        assert!(content.contains("Yes"));
        assert!(content.contains("No"));

        let _ = std::fs::remove_file(path);
    }
}
