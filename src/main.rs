use chronos::tracker::{TimeTracker, TrackerState};

fn main() {
    println!("Initializing Chronos...");

    let mut tracker = TimeTracker::new();
    println!("Tracker state: {:?}", tracker.state());

    match tracker.start(1) {
        Ok(()) => println!("Tracking started for task 1"),
        Err(e) => eprintln!("Error: {e}"),
    }

    match tracker.stop() {
        Ok(entry) => println!(
            "Stopped task {:?} — duration: {:?}",
            entry.task_id, entry.duration
        ),
        Err(e) => eprintln!("Error: {e}"),
    }

    assert_eq!(tracker.state(), TrackerState::Idle);
    println!("Chronos ready.");
}
