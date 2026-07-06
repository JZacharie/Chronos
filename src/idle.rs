use std::time::{Duration, Instant};

const IDLE_TIMEOUT_SECS: u64 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleState {
    Active,
    Idle(Duration),
    Returning,
}

pub struct IdleDetector {
    last_activity: Instant,
    idle_state: IdleState,
    idle_threshold: Duration,
    was_tracking: bool,
}

impl IdleDetector {
    pub fn new() -> Self {
        Self {
            last_activity: Instant::now(),
            idle_state: IdleState::Active,
            idle_threshold: Duration::from_secs(IDLE_TIMEOUT_SECS),
            was_tracking: false,
        }
    }

    pub fn poke(&mut self) {
        self.last_activity = Instant::now();
        if matches!(self.idle_state, IdleState::Idle(..)) || self.idle_state == IdleState::Returning
        {
            self.idle_state = IdleState::Active;
        }
    }

    pub fn state(&self) -> IdleState {
        self.idle_state
    }

    pub fn last_activity_elapsed(&self) -> Duration {
        self.last_activity.elapsed()
    }

    pub fn set_threshold(&mut self, secs: u64) {
        self.idle_threshold = Duration::from_secs(secs);
    }

    pub fn update(&mut self, tracker_was_running: bool) -> Option<IdleEvent> {
        let elapsed = self.last_activity.elapsed();

        match self.idle_state {
            IdleState::Active => {
                if elapsed >= self.idle_threshold && tracker_was_running {
                    self.idle_state = IdleState::Idle(elapsed);
                    self.was_tracking = true;
                    return Some(IdleEvent::BecameIdle(elapsed));
                }
            }
            IdleState::Idle(..) => {
                if elapsed < self.idle_threshold {
                    self.idle_state = IdleState::Returning;
                    self.was_tracking = false;
                    return Some(IdleEvent::ReturnedFromIdle);
                }
            }
            IdleState::Returning => {
                if elapsed < Duration::from_millis(100) {
                    self.idle_state = IdleState::Active;
                }
            }
        }
        None
    }

    pub fn acknowledge_return(&mut self) {
        self.last_activity = Instant::now();
        self.idle_state = IdleState::Active;
    }
}

impl Default for IdleDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdleEvent {
    BecameIdle(Duration),
    ReturnedFromIdle,
}
