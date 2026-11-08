use super::baseline;
use plumb::semantics::admission::{Inspector, Record};
use plumb::windtun::qualification::State;
use serde_json::Value;

const JOBS: &str =
    "https://api.github.com/repos/PerishLab/wharf/actions/runs/32/attempts/1/jobs?per_page=100";

#[test]
fn materials() {
    let (mut fixture, reference) = baseline::setup();
    fixture.values.remove(JOBS);
    let Record::Adoption(adoption) = Record::parse(&fixture.values[&reference.url]).unwrap() else {
        panic!("adoption fixture");
    };
    let inspector = Inspector::new(fixture);
    let prepared = inspector.materials(&adoption).unwrap();
    assert_eq!(prepared.collection.state, "observed");
    assert_eq!(prepared.review.judgment.marker, "v0.138.0");
    assert_eq!(
        inspector.baseline(&reference, "plumb").unwrap_err().state,
        State::Unknown
    );
}

#[test]
fn mismatch() {
    let (fixture, reference) = baseline::setup();
    let mut value: Value = serde_json::from_slice(&fixture.values[&reference.url]).unwrap();
    value["release"]["snapshot"]["materials"][0]["sha256"] = "0".repeat(64).into();
    value.sort_all_objects();
    let raw = serde_json::to_vec(&value).unwrap();
    let Record::Adoption(adoption) = Record::parse(&raw).unwrap() else {
        panic!("adoption fixture");
    };
    let fault = Inspector::new(fixture).materials(&adoption).unwrap_err();
    assert_eq!(fault.state, State::Refused);
    assert!(fault.reason.contains("exact source and formal Scene"));
}
