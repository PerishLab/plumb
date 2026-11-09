mod changes;
mod fixture;

use fixture::{issued, publish};
use plumb::semantics::Decision;
use plumb::semantics::admission::{Checkpoint, Selection, Selector, Tag};
use plumb::windtun::qualification::State;
use serde_json::json;

#[test]
fn current() {
    let (held, selection) = fixture::setup();
    let reference = selection.reference().unwrap();
    let selected = held.inspector().selected(&selection.root, "plumb").unwrap();
    assert_eq!(selected.family().root(), &selection.root);
    assert_eq!(selected.selection().reference(), &reference);
    let value = serde_json::to_value(selected).unwrap();
    assert_eq!(value["authentication"], true);
    assert_eq!(value["publication"], false);
    assert_eq!(value["family"]["prior"], "unknown");
}

#[test]
fn materials() {
    let (mut held, mut selection) = fixture::setup();
    selection.native.run += 20;
    selection.native.job += 20;
    held.fixture.values.remove(&Selector::url("plumb").unwrap());
    held.fixture
        .values
        .remove(&Selector::inception("plumb").unwrap());
    let inspector = plumb::semantics::admission::Inspector::new(fixture::Initial(held));
    let prepared = inspector.nominate(&selection, "plumb").unwrap();
    let value = serde_json::to_value(prepared).unwrap();
    assert_eq!(value["authentication"], false);
    assert_eq!(value["publication"], false);
    assert_eq!(value["family"]["authentication"], true);
    assert!(
        inspector
            .choice(&selection.reference().unwrap(), "plumb")
            .is_err()
    );
}

#[test]
fn missing() {
    let (mut held, selection) = fixture::setup();
    held.fixture.values.remove(&Selector::url("plumb").unwrap());
    assert_eq!(
        held.inspector()
            .selected(&selection.root, "plumb")
            .unwrap_err()
            .state,
        State::Unknown
    );
}

#[test]
fn origin() {
    let (mut held, selection) = fixture::setup();
    let route = format!(
        "https://api.github.com/repos/PerishLab/wharf/actions/runs/{}/attempts/1/jobs?per_page=100",
        selection.native.run
    );
    held.fixture.edit(&route, |value| {
        value["jobs"][0]["conclusion"] = "failure".into()
    });
    assert!(held.inspector().selected(&selection.root, "plumb").is_err());
}

#[test]
fn root() {
    let (held, selection) = fixture::setup();
    assert!(
        held.inspector()
            .selected(&held.history, "plumb")
            .unwrap_err()
            .reason
            .contains("expected original adoption")
    );
    let mut wrong = selection;
    wrong.root = held.history.clone();
    assert!(held.inspector().nominate(&wrong, "plumb").is_err());
}

#[test]
fn descendant() {
    let (mut held, first) = fixture::setup();
    held.append("v0.139.0", Decision::Changed);
    held.present();
    let next = fixture::next(&held, &first);
    issued(&mut held, &next);
    publish(&mut held, &next);
    let selected = held.inspector().selected(&first.root, "plumb").unwrap();
    assert_eq!(selected.family().stable().world().source.marker, "v0.139.0");
    assert_eq!(selected.family().root(), &first.root);
    assert_eq!(selected.family().anchor(), &held.reference);
}

#[test]
fn fork() {
    let (mut held, first) = fixture::setup();
    let mut next = first.clone();
    next.previous = Some(first.reference().unwrap());
    next.sequence += 1;
    next.native.run += 1;
    next.native.job += 1;
    issued(&mut held, &next);
    publish(&mut held, &next);
    assert!(
        held.inspector()
            .selected(&first.root, "plumb")
            .unwrap_err()
            .reason
            .contains("did not advance stable")
    );
}

#[test]
fn rollback() {
    let (mut held, first) = fixture::setup();
    let old = first.clone();
    held.append("v0.139.0", Decision::Changed);
    held.present();
    publish(&mut held, &old);
    assert!(
        held.inspector()
            .selected(&old.root, "plumb")
            .unwrap_err()
            .reason
            .contains("latest retained stable")
    );
}

#[test]
fn removed() {
    let (mut held, first) = fixture::setup();
    let mut checkpoint = Checkpoint::parse(&held.fixture.values[&held.history.url]).unwrap();
    checkpoint = super::append(&checkpoint, &held.history);
    checkpoint.markers.push(Tag {
        marker: "v0.139.0".into(),
        commit: "a".repeat(40),
    });
    checkpoint.markers.sort_by(|a, b| a.marker.cmp(&b.marker));
    held.history = super::retained(&mut held.fixture, &checkpoint);
    held.fixture.put(
        &plumb::semantics::admission::Discovery::url("plumb").unwrap(),
        json!({"schema":plumb::semantics::admission::Discovery::SCHEMA,"reference":held.history}),
    );
    assert!(
        held.inspector()
            .selected(&first.root, "plumb")
            .unwrap_err()
            .reason
            .contains("latest retained stable")
    );
}

#[test]
fn canonical() {
    let (_, selection) = fixture::setup();
    Selection::parse(&selection.encode().unwrap()).unwrap();
    let mut value = serde_json::to_value(selection).unwrap();
    value["authentication"] = true.into();
    value.sort_all_objects();
    assert!(Selection::parse(&serde_json::to_vec(&value).unwrap()).is_err());
}

#[test]
fn sequence() {
    let (mut held, first) = fixture::setup();
    held.append("v0.139.0", Decision::Changed);
    held.present();
    let mut next = fixture::next(&held, &first);
    next.sequence += 1;
    assert!(
        held.inspector()
            .nominate(&next, "plumb")
            .unwrap_err()
            .reason
            .contains("sequence")
    );
}
