use plumb::vendor::forgejo::State;
use serde_json::json;

fn lane() -> serde_json::Value {
    json!({
        "state": "success",
        "sha": "055018b61e106296436f36fe77f9826701656936",
        "statuses": [
            {"context": "guard / guard (pull_request)", "status": "success"},
            {"context": "guard / proof (pull_request)", "status": "success"}
        ]
    })
}

#[test]
fn qualified() {
    let held = State::read(&lane());
    assert_eq!(held.state, "success");
    assert_eq!(held.count, 2);
}

#[test]
fn pending() {
    let mut value = lane();
    value["state"] = json!("pending");
    let held = State::read(&value);
    assert_eq!(held.state, "pending");
    assert_eq!(held.count, 2);
}

#[test]
fn absent() {
    let held = State::read(&json!({"state": "pending", "statuses": []}));
    assert_eq!(held.count, 0);
}

#[test]
fn malformed() {
    let held = State::read(&json!({}));
    assert_eq!(held.state, "");
    assert_eq!(held.count, 0);
}

#[test]
fn cut() {
    use plumb::vendor::forgejo::{Cut, settled};
    assert_eq!(
        settled("release/v1.0.0", "main", "abc", "abc"),
        Ok(Cut::Held)
    );
    let moved = settled("release/v1.0.0", "main", "abc", "def").expect_err("a frozen line");
    assert!(moved.contains("frozen"));
    assert!(moved.contains("abc") && moved.contains("def"));
    let unread = settled("release/v1.0.0", "main", "", "def").expect_err("unread commit");
    assert!(unread.contains("could not be compared"));
}
