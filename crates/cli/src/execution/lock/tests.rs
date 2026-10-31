use super::*;
use serde_json::json;

#[test]
fn configuration() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("clippy.toml"),
        "disallowed-methods = [\"custom::call\", \"std::fs::File::lock\"]\nmsrv = \"1.89\"\n",
    )
    .unwrap();
    let text = policy::configuration(root.path()).unwrap();
    let value = text.parse::<toml::Table>().unwrap();
    let methods = value["disallowed-methods"].as_array().unwrap();
    assert_eq!(methods.len(), 6);
    assert!(
        methods
            .iter()
            .any(|value| value.as_str() == Some("custom::call"))
    );
    assert_eq!(value["msrv"].as_str(), Some("1.89"));
    let lock = methods
        .iter()
        .find(|value| {
            value.get("path").and_then(toml::Value::as_str) == Some("std::fs::File::lock")
        })
        .unwrap();
    assert_eq!(
        lock["reason"].as_str(),
        Some("plumb.lock/std::fs::File::lock")
    );
}

#[test]
fn collision() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("clippy.toml"), "").unwrap();
    std::fs::write(root.path().join(".clippy.toml"), "").unwrap();
    assert!(
        policy::configuration(root.path())
            .unwrap_err()
            .contains("conflicting")
    );
}

fn diagnostic() -> serde_json::Value {
    serde_json::from_str(r#"{"reason":"compiler-message", "package_id":"path+file:///app#app@0.0.0", "target":{"name":"app", "kind":["lib"]}, "message":{"message":"use of a disallowed method `std::fs::File::try_lock`","code":{"code":"clippy::disallowed_methods"}, "children":[{"message":"plumb.lock/std::fs::File::try_lock"}], "spans":[{"file_name":"carrier/src/lib.rs","is_primary":true,"expansion":{"span":{"file_name":"app/src/lib.rs","line_start":9}}}]}}"#).unwrap()
}

#[test]
fn expansion() {
    let message = diagnostic();
    let text = format!("{message}\n{{\"reason\":\"build-finished\",\"success\":true}}\n");
    let report = messages::read(text.as_bytes()).unwrap();
    let value = serde_json::to_value(report).unwrap();
    assert_eq!(value["findings"][0]["operation"], "std::fs::File::try_lock");
    assert_eq!(
        value["findings"][0]["spans"][0]["expansion"]["span"]["file_name"],
        "app/src/lib.rs"
    );
    assert_eq!(
        value["findings"][0]["package"],
        "path+file:///app#app@0.0.0"
    );
}

#[test]
fn incomplete() {
    for text in [
        "",
        "invalid\n",
        "{\"reason\":\"build-finished\",\"success\":false}\n",
        "{\"reason\":\"unknown\"}\n",
        "{\"reason\":\"build-finished\",\"success\":true}\n{}\n",
    ] {
        assert!(messages::read(text.as_bytes()).is_err(), "{text}");
    }
}

#[test]
fn unrelated() {
    let mut message = diagnostic();
    message["message"]["children"] = json!([{"message":"custom::call is forbidden"}]);
    let text = format!("{message}\n{{\"reason\":\"build-finished\",\"success\":true}}\n");
    assert!(messages::read(text.as_bytes()).unwrap().findings.is_empty());
}

#[test]
fn malformed() {
    let mut message = diagnostic();
    message["message"]["spans"] = json!([]);
    let text = format!("{message}\n{{\"reason\":\"build-finished\",\"success\":true}}\n");
    assert!(messages::read(text.as_bytes()).is_err());
}

#[test]
fn mismatch() {
    let mut message = diagnostic();
    message["message"]["message"] = json!("use of a disallowed method `std::mem::drop`");
    let text = format!("{message}\n{{\"reason\":\"build-finished\",\"success\":true}}\n");
    assert!(messages::read(text.as_bytes()).is_err());
}
