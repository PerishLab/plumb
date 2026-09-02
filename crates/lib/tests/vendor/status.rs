use plumb::forgejo::State;
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
    use plumb::forgejo::{Cut, settled};
    assert_eq!(
        settled("release/v1.0.0", "main", "abc", "abc"),
        Ok(Cut::Held("abc".into()))
    );
    let moved = settled("release/v1.0.0", "main", "abc", "def").expect_err("a frozen line");
    assert!(moved.contains("frozen"));
    assert!(moved.contains("abc") && moved.contains("def"));
    let unread = settled("release/v1.0.0", "main", "", "def").expect_err("unread commit");
    assert!(unread.contains("could not be compared"));
}

#[test]
fn graph() {
    use plumb::forgejo::{Outcome, graph};
    use serde_json::json;

    let task = |status: &str| json!({ "name": status, "status": status });
    assert_eq!(
        graph(&[]).expect("empty graph"),
        Outcome::Waiting,
        "a run that has laid out no job has not finished"
    );
    assert_eq!(
        graph(&[task("blocked"), task("running")]).expect("partial graph"),
        Outcome::Waiting
    );
    assert_eq!(
        graph(&[task("success"), task("skipped")]).expect("settled graph"),
        Outcome::Success
    );
    assert!(matches!(
        graph(&[task("success"), task("failure")]).expect("failed graph"),
        Outcome::Failed { status, tasks } if status == "failed" && tasks == ["failure"]
    ));
}
