use rusqlite::{Connection, Result, params};

const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_id INTEGER REFERENCES tasks(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    is_project BOOLEAN DEFAULT FALSE,
    is_payable BOOLEAN DEFAULT TRUE,
    is_archived BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS time_periods (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER REFERENCES tasks(id) ON DELETE CASCADE,
    begin_time TIMESTAMP NOT NULL,
    end_time TIMESTAMP,
    duration_seconds INTEGER DEFAULT 0,
    is_payable BOOLEAN DEFAULT TRUE
);
";

#[derive(Debug, Clone)]
pub struct TaskRow {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub is_project: bool,
    pub is_payable: bool,
    pub is_archived: bool,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct TimePeriodRow {
    pub id: i64,
    pub task_id: i64,
    pub begin_time: String,
    pub end_time: Option<String>,
    pub duration_seconds: i64,
    pub is_payable: bool,
}

pub fn open_database(path: &str) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    conn.execute_batch(SCHEMA_SQL)?;
    Ok(conn)
}

pub fn create_task(
    conn: &Connection,
    parent_id: Option<i64>,
    name: &str,
    is_project: bool,
    is_payable: bool,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO tasks (parent_id, name, is_project, is_payable) VALUES (?1, ?2, ?3, ?4)",
        params![parent_id, name, is_project, is_payable],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_task(conn: &Connection, id: i64) -> Result<Option<TaskRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, parent_id, name, is_project, is_payable, is_archived, created_at
         FROM tasks WHERE id = ?1",
    )?;

    let mut rows = stmt.query_map(params![id], |row| {
        Ok(TaskRow {
            id: row.get(0)?,
            parent_id: row.get(1)?,
            name: row.get(2)?,
            is_project: row.get(3)?,
            is_payable: row.get(4)?,
            is_archived: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?;

    match rows.next() {
        Some(Ok(task)) => Ok(Some(task)),
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}

pub fn get_all_tasks(conn: &Connection) -> Result<Vec<TaskRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, parent_id, name, is_project, is_payable, is_archived, created_at
         FROM tasks ORDER BY created_at ASC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(TaskRow {
            id: row.get(0)?,
            parent_id: row.get(1)?,
            name: row.get(2)?,
            is_project: row.get(3)?,
            is_payable: row.get(4)?,
            is_archived: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?;

    rows.collect()
}

pub fn delete_task(conn: &Connection, id: i64) -> Result<bool> {
    let affected = conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
    Ok(affected > 0)
}

pub fn archive_task(conn: &Connection, id: i64, archived: bool) -> Result<bool> {
    let affected = conn.execute(
        "UPDATE tasks SET is_archived = ?1 WHERE id = ?2",
        params![archived, id],
    )?;
    Ok(affected > 0)
}

pub fn create_time_period(
    conn: &Connection,
    task_id: i64,
    begin_time: &str,
    end_time: Option<&str>,
    duration_seconds: i64,
    is_payable: bool,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO time_periods (task_id, begin_time, end_time, duration_seconds, is_payable)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![task_id, begin_time, end_time, duration_seconds, is_payable],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_time_periods_for_task(conn: &Connection, task_id: i64) -> Result<Vec<TimePeriodRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, begin_time, end_time, duration_seconds, is_payable
         FROM time_periods WHERE task_id = ?1 ORDER BY begin_time ASC",
    )?;

    let rows = stmt.query_map(params![task_id], |row| {
        Ok(TimePeriodRow {
            id: row.get(0)?,
            task_id: row.get(1)?,
            begin_time: row.get(2)?,
            end_time: row.get(3)?,
            duration_seconds: row.get(4)?,
            is_payable: row.get(5)?,
        })
    })?;

    rows.collect()
}

pub fn get_total_duration_for_task(conn: &Connection, task_id: i64) -> Result<i64> {
    conn.query_row(
        "SELECT COALESCE(SUM(duration_seconds), 0) FROM time_periods WHERE task_id = ?1",
        params![task_id],
        |row| row.get(0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_in_memory_db() -> Connection {
        open_database(":memory:").expect("Failed to create in-memory database")
    }

    #[test]
    fn open_database_creates_tables() {
        let conn = setup_in_memory_db();
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(tables.contains(&"tasks".to_string()));
        assert!(tables.contains(&"time_periods".to_string()));
    }

    #[test]
    fn create_and_retrieve_task() {
        let conn = setup_in_memory_db();
        let id = create_task(&conn, None, "Test Task", true, true).unwrap();
        let task = get_task(&conn, id).unwrap().expect("Task should exist");
        assert_eq!(task.name, "Test Task");
        assert!(task.is_project);
        assert!(task.is_payable);
        assert!(task.parent_id.is_none());
    }

    #[test]
    fn create_task_with_parent() {
        let conn = setup_in_memory_db();
        let parent_id = create_task(&conn, None, "Project", true, true).unwrap();
        let child_id = create_task(&conn, Some(parent_id), "Subtask", false, true).unwrap();
        let child = get_task(&conn, child_id)
            .unwrap()
            .expect("Child should exist");
        assert_eq!(child.parent_id, Some(parent_id));
    }

    #[test]
    fn get_nonexistent_task_returns_none() {
        let conn = setup_in_memory_db();
        let result = get_task(&conn, 999).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn get_all_tasks_returns_all() {
        let conn = setup_in_memory_db();
        create_task(&conn, None, "A", true, true).unwrap();
        create_task(&conn, None, "B", false, true).unwrap();
        create_task(&conn, None, "C", true, false).unwrap();
        let tasks = get_all_tasks(&conn).unwrap();
        assert_eq!(tasks.len(), 3);
    }

    #[test]
    fn delete_task_test() {
        let conn = setup_in_memory_db();
        let id = create_task(&conn, None, "To Delete", true, true).unwrap();
        assert!(super::delete_task(&conn, id).unwrap());
        assert!(get_task(&conn, id).unwrap().is_none());
    }

    #[test]
    fn delete_nonexistent_task_returns_false() {
        let conn = setup_in_memory_db();
        assert!(!super::delete_task(&conn, 999).unwrap());
    }

    #[test]
    fn archive_and_unarchive() {
        let conn = setup_in_memory_db();
        let id = create_task(&conn, None, "Archivable", true, true).unwrap();
        archive_task(&conn, id, true).unwrap();
        let task = get_task(&conn, id).unwrap().unwrap();
        assert!(task.is_archived);
        archive_task(&conn, id, false).unwrap();
        let task = get_task(&conn, id).unwrap().unwrap();
        assert!(!task.is_archived);
    }

    #[test]
    fn create_and_retrieve_time_period() {
        let conn = setup_in_memory_db();
        let task_id = create_task(&conn, None, "Task", false, true).unwrap();
        let tp_id = create_time_period(
            &conn,
            task_id,
            "2026-07-06 09:00:00",
            Some("2026-07-06 10:00:00"),
            3600,
            true,
        )
        .unwrap();
        let periods = get_time_periods_for_task(&conn, task_id).unwrap();
        assert_eq!(periods.len(), 1);
        assert_eq!(periods[0].id, tp_id);
        assert_eq!(periods[0].duration_seconds, 3600);
    }

    #[test]
    fn total_duration_for_task() {
        let conn = setup_in_memory_db();
        let task_id = create_task(&conn, None, "Task", false, true).unwrap();
        create_time_period(
            &conn,
            task_id,
            "2026-07-06 09:00:00",
            Some("2026-07-06 11:00:00"),
            7200,
            true,
        )
        .unwrap();
        create_time_period(
            &conn,
            task_id,
            "2026-07-06 14:00:00",
            Some("2026-07-06 15:30:00"),
            5400,
            true,
        )
        .unwrap();
        let total = get_total_duration_for_task(&conn, task_id).unwrap();
        assert_eq!(total, 7200 + 5400);
    }

    #[test]
    fn task_without_parent_is_root() {
        let conn = setup_in_memory_db();
        let id = create_task(&conn, None, "Root", true, true).unwrap();
        let task = get_task(&conn, id).unwrap().unwrap();
        assert!(task.parent_id.is_none());
    }
}
