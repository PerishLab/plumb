use super::world::{Fixture, GH, ROOT};
use plumb::windtun::qualification::{Inspector, State};
use serde_json::{Value, json};

#[test]
fn coverage() {
    for (field, value) in [
        ("cleanup", json!(false)),
        (
            "requirements",
            json!({"platform":"windows-x64","declared":["health"],"selected":["health"],"scope":"declared"}),
        ),
        ("groups", json!([])),
        (
            "groups",
            json!([{"digest":"f".repeat(64),"cases":["health","other"]}]),
        ),
        (
            "groups",
            json!([{"digest":"f".repeat(64),"cases":["health"]},{"digest":"f".repeat(64),"cases":["health"]}]),
        ),
        ("products", json!([])),
        ("images", json!([])),
        ("declaration", Value::Null),
        (
            "observer",
            json!({"run":"10","attempt":1,"wharf":"d".repeat(40)}),
        ),
        ("prepared", json!(true)),
    ] {
        let mut fixture = Fixture::new();
        fixture.pass[field] = value;
        fixture.publish();
        let report = Inspector::new(&fixture).inspect("PerishLab/keel", "v1.0.0-rc.1");
        assert_eq!(report.state, State::Refused, "{field}: {report:?}");
        assert!(!report.qualified && !report.promotion);
    }
}

#[test]
fn references() {
    for value in [
        json!({"url":"https://foreign/intent.json","size":1,"sha256":"f".repeat(64)}),
        json!({"url":format!("{ROOT}/attempts/10-1/intent.json"),"size":true,"sha256":"f".repeat(64)}),
        json!({"url":format!("{ROOT}/attempts/10-1/intent.json"),"size":1,"sha256":"f".repeat(64)}),
        json!({"url":format!("{ROOT}/attempts/10-1/intent.json?foreign"),"size":1,"sha256":"f".repeat(64)}),
    ] {
        let mut fixture = Fixture::new();
        let url = format!("{ROOT}/latest.json");
        let mut index = fixture.get(&url);
        index["intent"] = value;
        fixture.set(&url, &index);
        let report = Inspector::new(&fixture).inspect("PerishLab/keel", "v1.0.0-rc.1");
        assert_eq!(report.state, State::Refused, "{report:?}");
    }
}

#[test]
fn native() {
    let url = format!("{GH}/actions/runs/10/attempts/1/jobs?per_page=100");
    for field in ["head_sha", "run_attempt", "conclusion", "steps"] {
        let mut fixture = Fixture::new();
        let mut jobs = fixture.get(&url);
        jobs["jobs"][1][field] = json!("foreign");
        fixture.set(&url, &jobs);
        let report = Inspector::new(&fixture).inspect("PerishLab/keel", "v1.0.0-rc.1");
        assert_eq!(report.state, State::Refused, "{field}: {report:?}");
    }
}

#[test]
fn failures() {
    for state in ["failed", "cancelled", "unknown", "pending"] {
        let mut fixture = Fixture::new();
        fixture.result["ok"] = json!(false);
        fixture.result["state"] = json!(state);
        fixture.publish();
        let report = Inspector::new(&fixture).inspect("PerishLab/keel", "v1.0.0-rc.1");
        assert_eq!(report.state, State::Refused, "{report:?}");
    }
    let mut fixture = Fixture::new();
    fixture
        .bodies
        .remove(&format!("{ROOT}/attempts/10-1/result.json"));
    let report = Inspector::new(&fixture).inspect("PerishLab/keel", "v1.0.0-rc.1");
    assert_eq!(report.state, State::Unknown);
}

#[test]
fn duplicate() {
    let mut fixture = Fixture::new();
    let url = format!("{ROOT}/latest.json");
    let raw = fixture.bodies.get_mut(&url).unwrap();
    raw.pop();
    raw.extend_from_slice(b",\"schema\":\"wharf.windtun.index/v1\"}");
    let report = Inspector::new(&fixture).inspect("PerishLab/keel", "v1.0.0-rc.1");
    assert_eq!(report.state, State::Refused);
    let mut fixture = Fixture::new();
    let url = format!("{ROOT}/latest.json");
    let index = fixture.get(&url);
    fixture
        .bodies
        .insert(url, serde_json::to_vec_pretty(&index).unwrap());
    assert_eq!(
        Inspector::new(&fixture)
            .inspect("PerishLab/keel", "v1.0.0-rc.1")
            .state,
        State::Refused
    );
}

#[test]
fn artifact() {
    let mut fixture = Fixture::new();
    let url = fixture.pins["product"]["seal"]["artifacts"]["linux-x64"]["url"]
        .as_str()
        .unwrap()
        .to_string();
    fixture.bodies.get_mut(&url).unwrap().truncate(10);
    assert_eq!(
        Inspector::new(&fixture)
            .inspect("PerishLab/keel", "v1.0.0-rc.1")
            .state,
        State::Refused
    );
    let mut fixture = Fixture::new();
    fixture.pass["runner"]["sha256"] = json!("0".repeat(64));
    fixture.publish();
    assert_eq!(
        Inspector::new(&fixture)
            .inspect("PerishLab/keel", "v1.0.0-rc.1")
            .state,
        State::Refused
    );
}

#[test]
fn source() {
    for (url, field, value) in [
        (
            "https://api.github.com/repos/PerishLab/keel/git/ref/tags/v1.0.0-rc.1".into(),
            "object",
            json!({"type":"commit", "sha":"1".repeat(40)}),
        ),
        (
            format!(
                "https://api.github.com/repos/PerishLab/keel/git/tags/{}",
                "5".repeat(40)
            ),
            "tag",
            json!("v1.0.0-rc.2"),
        ),
        (GH.to_string(), "id", json!(1)),
        (format!("{GH}/branches/main"), "protected", json!(false)),
        (
            "https://api.github.com/repos/PerishLab/keel".into(),
            "owner",
            json!({"id":1,"node_id":"O_kgDOE68CNA","login":"PerishLab"}),
        ),
    ] {
        let mut fixture = Fixture::new();
        let mut document = fixture.get(&url);
        document[field] = value;
        fixture.set(&url, &document);
        assert_eq!(
            Inspector::new(&fixture)
                .inspect("PerishLab/keel", "v1.0.0-rc.1")
                .state,
            State::Refused
        );
    }
    let mut fixture = Fixture::new();
    let url = format!(
        "{GH}/contents/resources/check/windtun.json?ref={}",
        "a".repeat(40)
    );
    let mut document = fixture.get(&url);
    document["content"] = json!("e30=");
    fixture.set(&url, &document);
    assert_eq!(
        Inspector::new(&fixture)
            .inspect("PerishLab/keel", "v1.0.0-rc.1")
            .state,
        State::Refused
    );
}
