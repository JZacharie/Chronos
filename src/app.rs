use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::Result;

use crate::backup;
use crate::db;
use crate::idle::{IdleDetector, IdleEvent};
use crate::notify;
use crate::stats;
use crate::tracker::{TimeEntry, TimeTracker, TrackerState};

pub struct AppState {
    pub tracker: TimeTracker,
    pub db_path: PathBuf,
    pub db: Mutex<rusqlite::Connection>,
    pub current_task_name: Option<String>,
    pub current_task_id: Option<i64>,
    pub window_visible: bool,
    pub last_status: String,
    pub idle: IdleDetector,
    pub idle_dialog: Option<IdleEvent>,
    last_notify: String,
}

impl AppState {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        let conn = db::open_database(
            db_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid database path"))?,
        )?;

        let _ = backup::daily_backup(&db_path);
        let _ = backup::prune_backups(&db_path, 30);

        Ok(Self {
            tracker: TimeTracker::new(),
            db_path,
            db: Mutex::new(conn),
            current_task_name: None,
            current_task_id: None,
            window_visible: true,
            last_status: String::new(),
            idle: IdleDetector::new(),
            idle_dialog: None,
            last_notify: String::new(),
        })
    }

    fn notify(&mut self, summary: &str, body: &str) {
        let key = format!("{summary}|{body}");
        if self.last_notify == key {
            return;
        }
        self.last_notify = key;
        notify::send(summary, body);
    }

    pub fn tracker_state(&self) -> TrackerState {
        self.tracker.state()
    }

    pub fn start_tracking(&mut self, task_id: i64, task_name: &str) -> Result<(), String> {
        if self.tracker.state() != TrackerState::Idle {
            let _ = self.stop_tracking();
        }
        self.tracker.start(task_id).map_err(|e| e.to_string())?;
        self.current_task_id = Some(task_id);
        self.current_task_name = Some(task_name.to_string());
        self.notify("Chronos", &format!("Started tracking: {task_name}"));
        Ok(())
    }

    pub fn stop_tracking(&mut self) -> Result<TimeEntry, String> {
        if self.tracker.state() == TrackerState::Idle {
            return Err("Not tracking".to_string());
        }
        let entry = self.tracker.stop().map_err(|e| e.to_string())?;
        if let Some(task_id) = entry.task_id {
            let duration = entry.duration.as_secs() as i64;
            if duration > 0 {
                let now = stats::now_ts();
                let begin = now - duration;
                let db = self.db.lock().unwrap();
                let _ = db::create_time_period(&db, task_id, begin, Some(now), duration, true);
            }
        }
        self.current_task_name = None;
        self.current_task_id = None;
        let dur = format_duration(entry.duration.as_secs());
        self.notify("Chronos", &format!("Stopped tracking. Duration: {dur}"));
        Ok(entry)
    }

    pub fn pause_tracking(&mut self) -> Result<(), String> {
        self.tracker.pause().map_err(|e| e.to_string())?;
        self.notify("Chronos", "Tracking paused");
        Ok(())
    }

    pub fn resume_tracking(&mut self) -> Result<(), String> {
        self.tracker.resume().map_err(|e| e.to_string())?;
        let name = self.current_task_name.as_deref().unwrap_or("task");
        self.notify("Chronos", &format!("Resumed tracking: {name}"));
        Ok(())
    }

    pub fn elapsed_seconds(&self) -> u64 {
        self.tracker.elapsed().as_secs()
    }

    pub fn toggle_window(&mut self) {
        self.window_visible = !self.window_visible;
    }

    pub fn status_line(&self) -> &str {
        &self.last_status
    }

    pub fn register_activity(&mut self) {
        self.idle.poke();
    }

    pub fn check_idle(&mut self) {
        let was_running = self.tracker.state() == TrackerState::Running;
        if let Some(event) = self.idle.update(was_running) {
            match event {
                IdleEvent::BecameIdle(dur) => {
                    if self.tracker.state() == TrackerState::Running {
                        let _ = self.tracker.pause();
                        self.last_status = format!(
                            "Auto-paused after idle ({})",
                            format_duration(dur.as_secs())
                        );
                    }
                    self.idle_dialog = Some(event);
                }
                IdleEvent::ReturnedFromIdle => {
                    self.idle_dialog = Some(event);
                }
            }
        }
    }

    pub fn update_status(&mut self) {
        let state = self.tracker.state();
        let name = self.current_task_name.as_deref().unwrap_or("");
        self.last_status = match state {
            TrackerState::Idle => "Idle".to_string(),
            TrackerState::Running => {
                let secs = self.elapsed_seconds();
                if name.is_empty() {
                    format!("Tracking ({})", format_duration(secs))
                } else {
                    format!("{} \u{2014} {}", name, format_duration(secs))
                }
            }
            TrackerState::Paused => {
                let secs = self.elapsed_seconds();
                if name.is_empty() {
                    format!("Paused ({})", format_duration(secs))
                } else {
                    format!("{} \u{2014} Paused ({})", name, format_duration(secs))
                }
            }
        };
    }
}

pub fn format_duration(total_secs: u64) -> String {
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{}h {:02}m {:02}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {:02}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}
