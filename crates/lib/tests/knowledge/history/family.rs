#[path = "fixture.rs"]
pub(super) mod fixture;

use fixture::Inputs;
use plumb::semantics::Decision;
use plumb::windtun::qualification::State;

#[test]
fn root() {
    let held = Inputs::new();
    let family = held.inspector().family(&held.reference, "plumb").unwrap();
    assert_eq!(family.reference(), family.root());
    assert_eq!(family.anchor(), family.root());
    let value = serde_json::to_value(family).unwrap();
    assert_eq!(value["authentication"], true);
    assert_eq!(value["publication"], false);
}

#[test]
fn continuity() {
    let mut held = Inputs::new();
    let root = held.reference.clone();
    for marker in ["v0.138.1", "v0.138.2"] {
        held.append(marker, Decision::Same);
        let family = held.inspector().family(&held.reference, "plumb").unwrap();
        assert_eq!(family.anchor(), &root);
        assert_eq!(family.original().world().source.marker, "v0.138.0");
        assert_eq!(family.stable().world().source.marker, marker);
    }
    held.append("v0.139.0", Decision::Changed);
    let anchor = held.reference.clone();
    held.append("v0.139.1", Decision::Same);
    let family = held.inspector().family(&held.reference, "plumb").unwrap();
    assert_eq!(family.root(), &root);
    assert_eq!(family.anchor(), &anchor);
    assert_eq!(family.original().world().source.marker, "v0.139.0");
    assert_eq!(family.stable().world().source.marker, "v0.139.1");
}

#[test]
fn anchor() {
    let mut held = Inputs::new();
    held.append("v0.138.1", Decision::Same);
    let rolled = held.reference.clone();
    held.append("v0.138.2", Decision::Same);
    held.edit(|transition| transition.intent.anchor = rolled);
    assert!(
        held.inspector()
            .family(&held.reference, "plumb")
            .unwrap_err()
            .reason
            .contains("original family anchor")
    );
}

#[test]
fn judgment() {
    let mut held = Inputs::new();
    held.append("v0.138.1", Decision::Changed);
    assert!(
        held.inspector()
            .family(&held.reference, "plumb")
            .unwrap_err()
            .reason
            .contains("shared semantic judgment")
    );
    let mut held = Inputs::new();
    held.append("v0.139.0", Decision::Same);
    assert!(held.inspector().family(&held.reference, "plumb").is_err());
}

#[test]
fn reservations() {
    let mut held = Inputs::new();
    held.append("v0.138.1", Decision::Same);
    held.edit(|transition| transition.intent.request.reservations.clear());
    assert!(
        held.inspector()
            .family(&held.reference, "plumb")
            .unwrap_err()
            .reason
            .contains("independently derived inputs")
    );
}

#[test]
fn gap() {
    let mut held = Inputs::new();
    let first = held.history.clone();
    held.append("v0.138.1", Decision::Same);
    held.append("v0.138.2", Decision::Same);
    held.edit(|transition| transition.history = first);
    assert!(
        held.inspector()
            .family(&held.reference, "plumb")
            .unwrap_err()
            .reason
            .contains("previously retained history")
    );
}

#[test]
fn major() {
    let mut held = Inputs::new();
    held.append("v0.138.1", Decision::Same);
    held.edit(|transition| {
        transition.intent.request.major =
            Some("https://github.com/PerishLab/.github/issues/178#issuecomment-7".into())
    });
    let fault = held
        .inspector()
        .family(&held.reference, "plumb")
        .unwrap_err();
    assert_eq!(fault.state, State::Unknown);
    assert!(fault.reason.contains("user's decision"));
}

#[test]
fn exhaustion() {
    let mut held = Inputs::new();
    held.append("v0.138.1", Decision::Same);
    for _ in 0..64 {
        let previous = held.reference.clone();
        held.edit(|transition| transition.intent.previous = previous);
    }
    assert_eq!(
        held.inspector()
            .family(&held.reference, "plumb")
            .unwrap_err()
            .state,
        State::Unknown
    );
}

#[test]
fn effective() {
    let mut held = Inputs::new();
    held.complete("v0.139.0", Decision::Changed);
    let family = held.inspector().family(&held.reference, "plumb").unwrap();
    assert_ne!(
        family.stable().world().tree,
        family.stable().snapshot().source.tree
    );
    assert!(
        family
            .stable()
            .snapshot()
            .materials
            .iter()
            .any(|item| item.name == "plumb/release/source.json")
    );
    held.edit(|transition| transition.source = None);
    let fault = held
        .inspector()
        .family(&held.reference, "plumb")
        .unwrap_err();
    assert_eq!(fault.state, State::Unknown);
    assert!(fault.reason.contains("complete effective-source materials"));
}

#[test]
fn issuer() {
    let mut held = Inputs::new();
    held.append("v0.138.1", Decision::Same);
    let value: serde_json::Value =
        serde_json::from_slice(&held.fixture.values[&held.reference.url]).unwrap();
    let native = &value["intent"]["native"];
    let route = format!(
        "https://api.github.com/repos/PerishLab/wharf/actions/runs/{}/attempts/1/jobs?per_page=100",
        native["run"]
    );
    held.fixture.edit(&route, |jobs| {
        jobs["jobs"][0]["conclusion"] = "failure".into()
    });
    assert!(
        held.inspector()
            .family(&held.reference, "plumb")
            .unwrap_err()
            .reason
            .contains("successfully")
    );
}

#[test]
fn review() {
    let mut held = Inputs::new();
    held.append("v0.138.1", Decision::Same);
    held.edit(|transition| transition.intent.review.as_mut().unwrap().sha256 = "0".repeat(64));
    assert!(
        held.inspector()
            .family(&held.reference, "plumb")
            .unwrap_err()
            .reason
            .contains("review bytes differ")
    );
}

#[test]
fn reference() {
    let mut held = Inputs::new();
    held.append("v0.138.1", Decision::Same);
    let history = held.history.clone();
    held.edit(|transition| transition.intent.root = history);
    assert!(
        held.inspector()
            .family(&held.reference, "plumb")
            .unwrap_err()
            .reason
            .contains("replaced its root")
    );
}

#[test]
fn source() {
    let mut held = Inputs::new();
    held.complete("v0.139.0", Decision::Changed);
    let value: serde_json::Value =
        serde_json::from_slice(&held.fixture.values[&held.reference.url]).unwrap();
    let location = value["source"]["url"].as_str().unwrap();
    held.fixture.values.get_mut(location).unwrap().push(b'\n');
    assert!(held.inspector().family(&held.reference, "plumb").is_err());
}

#[test]
fn conflict() {
    let mut held = Inputs::new();
    let checkpoint =
        plumb::semantics::admission::Checkpoint::parse(&held.fixture.values[&held.history.url])
            .unwrap();
    let mut checkpoint = super::append(&checkpoint, &held.history);
    checkpoint.markers.push(plumb::semantics::admission::Tag {
        marker: "v0.139.0-rc.1".into(),
        commit: "a".repeat(40),
    });
    held.history = super::retained(&mut held.fixture, &checkpoint);
    held.append("v0.138.1", Decision::Same);
    assert!(
        held.inspector()
            .family(&held.reference, "plumb")
            .unwrap_err()
            .reason
            .contains("shared semantic judgment")
    );
}
