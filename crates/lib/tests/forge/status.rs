use plumb::forge::State;
use serde_json::json;

fn lane() -> serde_json::Value {
    json!({
        "state": "success",
        "sha": "055018b61e106296436f36fe77f9826701656936",
        "statuses": [
            {"context": "guard / guard (pull_request)", "status": "success"},
            {"context": "guard / guard-windows (pull_request)", "status": "success"}
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
