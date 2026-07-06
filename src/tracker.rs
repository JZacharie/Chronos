use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackerState {
    Idle,
    Running,
    Paused,
}

#[derive(Debug)]
pub struct TimeTracker {
    state: TrackerState,
    current_task_id: Option<i64>,
    session_start: Option<Instant>,
    accumulated: Duration,
}

impl TimeTracker {
    pub fn new() -> Self {
        Self {
            state: TrackerState::Idle,
            current_task_id: None,
            session_start: None,
            accumulated: Duration::ZERO,
        }
    }

    pub fn state(&self) -> TrackerState {
        self.state
    }

    pub fn current_task_id(&self) -> Option<i64> {
        self.current_task_id
    }

    pub fn accumulated(&self) -> Duration {
        self.accumulated
    }

    pub fn session_duration(&self) -> Option<Duration> {
        self.session_start.map(|start| start.elapsed())
    }

    pub fn start(&mut self, task_id: i64) -> Result<(), &'static str> {
        if self.state != TrackerState::Idle {
            return Err("Cannot start: tracker is not idle");
        }
        if task_id <= 0 {
            return Err("Invalid task ID");
        }
        self.state = TrackerState::Running;
        self.current_task_id = Some(task_id);
        self.session_start = Some(Instant::now());
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), &'static str> {
        if self.state != TrackerState::Running {
            return Err("Cannot pause: tracker is not running");
        }
        if let Some(start) = self.session_start {
            self.accumulated += start.elapsed();
        }
        self.state = TrackerState::Paused;
        self.session_start = None;
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), &'static str> {
        if self.state != TrackerState::Paused {
            return Err("Cannot resume: tracker is not paused");
        }
        if self.current_task_id.is_none() {
            return Err("Cannot resume: no task selected");
        }
        self.state = TrackerState::Running;
        self.session_start = Some(Instant::now());
        Ok(())
    }

    pub fn stop(&mut self) -> Result<TimeEntry, &'static str> {
        if self.state == TrackerState::Idle {
            return Err("Cannot stop: tracker is already idle");
        }
        let total = if let Some(start) = self.session_start {
            self.accumulated + start.elapsed()
        } else {
            self.accumulated
        };
        let entry = TimeEntry {
            task_id: self.current_task_id,
            duration: total,
        };
        self.state = TrackerState::Idle;
        self.current_task_id = None;
        self.session_start = None;
        self.accumulated = Duration::ZERO;
        Ok(entry)
    }

    pub fn switch_task(&mut self, new_task_id: i64) -> Result<TimeEntry, &'static str> {
        if self.state != TrackerState::Running && self.state != TrackerState::Paused {
            return Err("Cannot switch: tracker is idle");
        }
        if new_task_id <= 0 {
            return Err("Invalid task ID");
        }
        let entry = self.stop()?;
        self.start(new_task_id)?;
        Ok(entry)
    }

    pub fn elapsed(&self) -> Duration {
        let base = self.accumulated;
        if let Some(start) = self.session_start {
            base + start.elapsed()
        } else {
            base
        }
    }
}

impl Default for TimeTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeEntry {
    pub task_id: Option<i64>,
    pub duration: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_is_idle() {
        let tracker = TimeTracker::new();
        assert_eq!(tracker.state(), TrackerState::Idle);
        assert!(tracker.current_task_id().is_none());
        assert_eq!(tracker.accumulated(), Duration::ZERO);
    }

    #[test]
    fn start_transitions_to_running() {
        let mut tracker = TimeTracker::new();
        tracker.start(1).unwrap();
        assert_eq!(tracker.state(), TrackerState::Running);
        assert_eq!(tracker.current_task_id(), Some(1));
    }

    #[test]
    fn start_rejects_invalid_task_id() {
        let mut tracker = TimeTracker::new();
        assert!(tracker.start(0).is_err());
        assert!(tracker.start(-1).is_err());
    }

    #[test]
    fn start_fails_if_already_running() {
        let mut tracker = TimeTracker::new();
        tracker.start(1).unwrap();
        assert!(tracker.start(2).is_err());
    }

    #[test]
    fn pause_transitions_to_paused() {
        let mut tracker = TimeTracker::new();
        tracker.start(1).unwrap();
        tracker.pause().unwrap();
        assert_eq!(tracker.state(), TrackerState::Paused);
    }

    #[test]
    fn pause_fails_if_not_running() {
        let mut tracker = TimeTracker::new();
        assert!(tracker.pause().is_err());
        tracker.start(1).unwrap();
        tracker.pause().unwrap();
        assert!(tracker.pause().is_err());
    }

    #[test]
    fn resume_after_pause() {
        let mut tracker = TimeTracker::new();
        tracker.start(1).unwrap();
        tracker.pause().unwrap();
        tracker.resume().unwrap();
        assert_eq!(tracker.state(), TrackerState::Running);
        assert_eq!(tracker.current_task_id(), Some(1));
    }

    #[test]
    fn resume_fails_if_not_paused() {
        let mut tracker = TimeTracker::new();
        assert!(tracker.resume().is_err());
        tracker.start(1).unwrap();
        assert!(tracker.resume().is_err());
    }

    #[test]
    fn stop_returns_entry_and_resets() {
        let mut tracker = TimeTracker::new();
        tracker.start(1).unwrap();
        std::thread::sleep(Duration::from_millis(10));
        let entry = tracker.stop().unwrap();
        assert_eq!(entry.task_id, Some(1));
        assert!(entry.duration > Duration::ZERO);
        assert_eq!(tracker.state(), TrackerState::Idle);
        assert!(tracker.current_task_id().is_none());
    }

    #[test]
    fn stop_fails_if_idle() {
        let mut tracker = TimeTracker::new();
        assert!(tracker.stop().is_err());
    }

    #[test]
    fn switch_task_stops_current_and_starts_new() {
        let mut tracker = TimeTracker::new();
        tracker.start(1).unwrap();
        std::thread::sleep(Duration::from_millis(5));
        let entry = tracker.switch_task(2).unwrap();
        assert_eq!(entry.task_id, Some(1));
        assert!(entry.duration > Duration::ZERO);
        assert_eq!(tracker.state(), TrackerState::Running);
        assert_eq!(tracker.current_task_id(), Some(2));
    }

    #[test]
    fn accumulator_preserves_time_across_pause_resume() {
        let mut tracker = TimeTracker::new();
        tracker.start(1).unwrap();
        std::thread::sleep(Duration::from_millis(20));
        tracker.pause().unwrap();
        let paused_duration = tracker.accumulated();
        assert!(paused_duration >= Duration::from_millis(20));
        std::thread::sleep(Duration::from_millis(30));
        tracker.resume().unwrap();
        std::thread::sleep(Duration::from_millis(10));
        let total = tracker.elapsed();
        assert!(total > paused_duration);
    }

    #[test]
    fn state_transition_coverage() {
        let states = [
            (TrackerState::Idle, TrackerState::Running, "start"),
            (TrackerState::Running, TrackerState::Paused, "pause"),
            (TrackerState::Paused, TrackerState::Running, "resume"),
            (TrackerState::Running, TrackerState::Idle, "stop"),
            (TrackerState::Paused, TrackerState::Idle, "stop"),
        ];
        for &(_, target, action) in &states {
            let mut t = TimeTracker::new();
            if action == "start" {
                t.start(1).unwrap();
                assert_eq!(t.state(), target, "start -> {:?}", target);
            }
            if action == "pause" {
                t.start(1).unwrap();
                t.pause().unwrap();
                assert_eq!(t.state(), target);
            }
            if action == "resume" {
                t.start(1).unwrap();
                t.pause().unwrap();
                t.resume().unwrap();
                assert_eq!(t.state(), target);
            }
            if action == "stop" {
                t.start(1).unwrap();
                t.stop().unwrap();
                assert_eq!(t.state(), target);
            }
        }
    }

    #[test]
    fn pause_accumulates_time_correctly() {
        let mut tracker = TimeTracker::new();
        tracker.start(1).unwrap();
        std::thread::sleep(Duration::from_millis(15));
        tracker.pause().unwrap();
        let after_pause = tracker.accumulated();
        std::thread::sleep(Duration::from_millis(30));
        let still_same = tracker.accumulated();
        assert_eq!(
            after_pause, still_same,
            "accumulated should not change while paused"
        );
    }

    #[test]
    fn elapsed_includes_current_session() {
        let mut tracker = TimeTracker::new();
        tracker.start(1).unwrap();
        std::thread::sleep(Duration::from_millis(10));
        let e = tracker.elapsed();
        assert!(e >= Duration::from_millis(10));
        tracker.pause().unwrap();
        let e2 = tracker.elapsed();
        assert!(e2 >= e);
    }
}
