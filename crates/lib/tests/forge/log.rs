use plumb::forge::route;

#[test]
fn attempt() {
    assert_eq!(route(316, 1, 1), "/actions/runs/316/jobs/1/attempt/1/logs");
}

#[test]
fn later() {
    assert_eq!(route(316, 1, 3), "/actions/runs/316/jobs/1/attempt/3/logs");
}

#[test]
fn local() {
    assert_ne!(route(2729, 1, 1), route(316, 1, 1));
}

#[test]
fn first() {
    assert_eq!(route(316, 0, 1), "/actions/runs/316/jobs/0/attempt/1/logs");
}

#[test]
fn relative() {
    let held = route(1, 2, 3);
    assert!(held.starts_with("/actions/"));
    assert!(!held.contains("/api/"));
}
