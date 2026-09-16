use plumb::rule::{Completion, Resource};
use std::collections::{BTreeMap, BTreeSet};

fn receipt() -> Completion {
    Completion {
        schema: "plumb.ship-completion/v1".into(),
        marker: "a".repeat(64),
        contract: "b".repeat(64),
        resources: BTreeMap::from([
            ("ship/binary".into(), resource()),
            ("ship/npm.cli".into(), resource()),
        ]),
    }
}

fn resource() -> Resource {
    Resource {
        source: "https://releases.example/v1/objects/sha256/object".into(),
        digest: "c".repeat(64),
    }
}

fn verify(held: &Completion) -> Result<(), String> {
    held.verify(
        &"a".repeat(64),
        &"b".repeat(64),
        &BTreeSet::from(["ship/binary".into(), "ship/npm.cli".into()]),
    )
}

#[test]
fn complete() {
    let held = receipt();
    verify(&held).unwrap();
    let encoded = serde_json::to_vec(&held).unwrap();
    assert_eq!(
        serde_json::from_slice::<Completion>(&encoded).unwrap(),
        held
    );
}

#[test]
fn omitted() {
    let mut held = receipt();
    held.resources.remove("ship/npm.cli");
    assert!(verify(&held).unwrap_err().contains("exact distribution"));
}

#[test]
fn unexpected() {
    let mut held = receipt();
    held.resources.insert("ship/extra".into(), resource());
    assert!(verify(&held).unwrap_err().contains("exact distribution"));
}

#[test]
fn identity() {
    let mut held = receipt();
    held.marker = "d".repeat(64);
    assert!(verify(&held).is_err());
    held = receipt();
    held.contract = "e".repeat(64);
    assert!(verify(&held).is_err());
}

#[test]
fn malformed() {
    let mut held = receipt();
    held.resources.get_mut("ship/binary").unwrap().digest = "not-a-digest".into();
    assert!(verify(&held).is_err());
    held = receipt();
    held.schema = "plumb.workflow-inventory/v1".into();
    assert!(verify(&held).is_err());
}

#[test]
fn empty() {
    let mut held = receipt();
    held.resources.clear();
    assert!(
        held.verify(&held.marker, &held.contract, &BTreeSet::new())
            .is_err()
    );
}

#[test]
fn address() {
    for source in [
        "http://example/object",
        "https:///object",
        "https://user:secret@example/object",
        "https://example/object?signature=secret",
        "https://example/object#fragment",
        "https://example/unsafe path",
    ] {
        let mut held = receipt();
        held.resources.get_mut("ship/binary").unwrap().source = source.into();
        assert!(verify(&held).is_err(), "{source}");
    }
}

#[test]
fn unknown() {
    let mut held = serde_json::to_value(receipt()).unwrap();
    held["cache"] = serde_json::json!("not a business proof");
    assert!(serde_json::from_value::<Completion>(held).is_err());
}

#[test]
fn duplicated() {
    let resource = serde_json::to_string(&resource()).unwrap();
    let text = format!(
        r#"{{"schema":"plumb.ship-completion/v1","marker":"{}","contract":"{}","resources":{{"ship/binary":{resource},"ship/binary":{resource}}}}}"#,
        "a".repeat(64),
        "b".repeat(64),
    );
    assert!(serde_json::from_str::<Completion>(&text).is_err());
}
