mod fixture;

use fixture::{BASE, Fixture, POINTER};
use serde_json::json;

#[test]
fn projection() {
    let report = Fixture::new().inspect();
    assert_eq!(report.state, "observed", "{:?}", report.reason);
    assert!(!report.adoption);
    assert!(!report.publication);
    let snapshot = report.snapshot.unwrap();
    assert_eq!(snapshot.coverage.len(), 2);
    assert!(snapshot.coverage.iter().any(|case| case.case == "hidden"));
    assert!(
        !snapshot
            .coverage
            .iter()
            .any(|case| case.case == "diagnostic")
    );
    assert_eq!(snapshot.product, "plumb");
    assert!(snapshot.digest().is_ok());
}

#[test]
fn historical() {
    let mut fixture = Fixture::new();
    fixture.scene(None);
    let report = fixture.inspect();
    assert_eq!(report.state, "unknown");
    assert!(report.reason.unwrap().contains("no subject metadata"));
    assert!(report.observation.unwrap().scenes.is_some());
    assert!(report.snapshot.is_none());
}

#[test]
fn ownership() {
    let mut fixture = Fixture::new();
    let mut mapping = Fixture::subjects();
    mapping
        .cases
        .iter_mut()
        .for_each(|case| case.subjects = vec!["concord".into()]);
    fixture.scene(Some(mapping));
    let report = fixture.inspect();
    assert_eq!(report.state, "unknown");
    assert!(report.reason.unwrap().contains("no normative coverage"));
}

#[test]
fn incomplete() {
    let mut fixture = Fixture::new();
    let mut mapping = Fixture::subjects();
    mapping.cases.pop();
    fixture.scene(Some(mapping));
    let report = fixture.inspect();
    assert_eq!(report.state, "refused");
    assert!(report.reason.unwrap().contains("every indexed case"));
}

#[test]
fn producer() {
    let mut fixture = Fixture::new();
    let mut mapping = Fixture::subjects();
    mapping.product = "plumb".into();
    fixture.scene(Some(mapping));
    let report = fixture.inspect();
    assert_eq!(report.state, "refused");
    assert!(report.reason.unwrap().contains("another Scene producer"));
}

#[test]
fn tampering() {
    let mut fixture = Fixture::new();
    let path = fixture
        .values
        .keys()
        .find(|key| key.ends_with("objects/group/main.py"))
        .unwrap()
        .clone();
    fixture
        .values
        .insert(path, b"assert something_else\n".to_vec());
    let report = fixture.inspect();
    assert_eq!(report.state, "refused");
    assert!(report.snapshot.is_none());
}

#[test]
fn moving() {
    let mut fixture = Fixture::new();
    fixture.moved = true;
    let report = fixture.inspect();
    assert_eq!(report.state, "unknown");
    assert!(report.snapshot.is_none());
    assert!(report.reason.unwrap().contains("moved"));
}

#[test]
fn source() {
    for field in ["full_name", "visibility"] {
        let mut fixture = Fixture::new();
        fixture.edit(BASE, |value| value[field] = json!("foreign"));
        assert_eq!(fixture.inspect().state, "refused");
    }
    let mut fixture = Fixture::new();
    fixture.edit(BASE, |value| value["owner"]["id"] = json!(1));
    assert_eq!(fixture.inspect().state, "refused");
    let mut fixture = Fixture::new();
    let url = format!("{BASE}/git/trees/{}?recursive=1", "b".repeat(40));
    fixture.edit(&url, |value| value["truncated"] = json!(true));
    assert_eq!(fixture.inspect().state, "refused");
}

#[test]
fn standing() {
    let mut fixture = Fixture::new();
    fixture.edit(POINTER, |value| {
        value["createdAt"] = json!("2026-11-06T13:00:00Z")
    });
    let report = fixture.inspect();
    assert_eq!(report.state, "refused");
    assert!(report.reason.unwrap().contains("standing differ"));
}
