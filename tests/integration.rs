use chronos::db;
use chronos::tracker::{TimeTracker, TrackerState};
use chronos::tree::TaskTree;

#[test]
fn tracker_complete_workflow() {
    let mut tracker = TimeTracker::new();
    assert_eq!(tracker.state(), TrackerState::Idle);

    tracker.start(1).unwrap();
    assert_eq!(tracker.state(), TrackerState::Running);

    std::thread::sleep(std::time::Duration::from_millis(5));
    let entry = tracker.stop().unwrap();
    assert_eq!(entry.task_id, Some(1));
    assert!(entry.duration.as_millis() >= 5);
    assert_eq!(tracker.state(), TrackerState::Idle);
}

#[test]
fn task_tree_complete_workflow() {
    let mut tree = TaskTree::new();

    let project = tree.add_task(None, "Client Project", true, true).unwrap();
    let dev = tree
        .add_task(Some(project), "Development", false, true)
        .unwrap();
    let meeting = tree
        .add_task(Some(project), "Meeting", false, false)
        .unwrap();

    assert_eq!(tree.root_tasks().len(), 1);
    assert_eq!(tree.children_of(project).len(), 2);

    tree.update_duration(dev, 7200).unwrap();
    tree.update_duration(meeting, 3600).unwrap();

    assert_eq!(tree.cumulative_duration(project).unwrap(), 10800);
    assert_eq!(tree.cumulative_duration(dev).unwrap(), 7200);

    let path = tree.path_to_root(dev);
    assert_eq!(path, vec![dev, project]);
}

#[test]
fn database_complete_workflow() {
    let conn = db::open_database(":memory:").unwrap();

    let project_id = db::create_task(&conn, None, "Integration Project", true, true).unwrap();
    let task_id =
        db::create_task(&conn, Some(project_id), "Integration Task", false, true).unwrap();

    db::create_time_period(
        &conn,
        task_id,
        "2026-07-06 09:00:00",
        Some("2026-07-06 10:30:00"),
        5400,
        true,
    )
    .unwrap();

    let total = db::get_total_duration_for_task(&conn, task_id).unwrap();
    assert_eq!(total, 5400);

    let tasks = db::get_all_tasks(&conn).unwrap();
    assert_eq!(tasks.len(), 2);

    let project = db::get_task(&conn, project_id).unwrap().unwrap();
    assert!(project.is_project);
}

#[test]
fn tracker_pause_resume_accumulates_correctly() {
    let mut tracker = TimeTracker::new();

    tracker.start(1).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));

    tracker.pause().unwrap();
    let time_at_pause = tracker.accumulated();

    std::thread::sleep(std::time::Duration::from_millis(20));
    assert_eq!(tracker.accumulated(), time_at_pause);

    tracker.resume().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));

    let entry = tracker.stop().unwrap();
    assert!(entry.duration >= std::time::Duration::from_millis(20));
}

#[test]
fn task_tree_nested_cumulative_duration() {
    let mut tree = TaskTree::new();

    let root = tree.add_task(None, "Root", true, true).unwrap();
    let a = tree.add_task(Some(root), "A", true, true).unwrap();
    let b = tree.add_task(Some(root), "B", true, true).unwrap();
    let a1 = tree.add_task(Some(a), "A1", false, true).unwrap();
    let a2 = tree.add_task(Some(a), "A2", false, true).unwrap();
    let b1 = tree.add_task(Some(b), "B1", false, true).unwrap();

    tree.update_duration(a1, 1000).unwrap();
    tree.update_duration(a2, 2000).unwrap();
    tree.update_duration(b1, 3000).unwrap();
    tree.update_duration(root, 500).unwrap();

    assert_eq!(tree.cumulative_duration(a1).unwrap(), 1000);
    assert_eq!(tree.cumulative_duration(a).unwrap(), 3000);
    assert_eq!(tree.cumulative_duration(b).unwrap(), 3000);
    assert_eq!(tree.cumulative_duration(root).unwrap(), 6500);
}
