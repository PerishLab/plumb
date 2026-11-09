use super::fixture;
use plumb::semantics::admission::{Checkpoint, Discovery, Inspector, Selector, Tag};
use plumb::windtun::qualification::{Fault, Read, State};
use serde_json::json;
use std::cell::Cell;

#[test]
fn extension() {
    let (mut held, first) = fixture::setup();
    let old = held.history.clone();
    let checkpoint = Checkpoint::parse(&held.fixture.values[&old.url]).unwrap();
    let mut checkpoint = super::super::append(&checkpoint, &old);
    checkpoint.markers.push(Tag {
        marker: "v0.139.0-rc.1".into(),
        commit: "a".repeat(40),
    });
    checkpoint.markers.sort_by(|a, b| a.marker.cmp(&b.marker));
    held.history = super::super::retained(&mut held.fixture, &checkpoint);
    held.fixture.values.insert(
        Discovery::url("plumb").unwrap(),
        Discovery {
            schema: Discovery::SCHEMA.into(),
            reference: held.history.clone(),
        }
        .encode("plumb")
        .unwrap(),
    );
    let selected = held.inspector().selected(&first.root, "plumb").unwrap();
    assert_eq!(selected.history().reference(), &held.history);
    assert_eq!(selected.selection().record().history, old);
    assert!(
        held.inspector()
            .nominate(&first, "plumb")
            .unwrap_err()
            .reason
            .contains("current retained history")
    );
}

#[test]
fn absent() {
    let (mut held, first) = fixture::setup();
    held.fixture.edit(
        "https://api.github.com/repos/PerishLab/plumb/tags?per_page=100&page=1",
        |tags| {
            tags.as_array_mut()
                .unwrap()
                .retain(|tag| tag["name"] != first.marker)
        },
    );
    let fault = held.inspector().selected(&first.root, "plumb").unwrap_err();
    assert_eq!(fault.state, State::Unknown);
    assert!(fault.reason.contains("latest retained stable is absent"));
}

#[test]
fn uncovered() {
    let (mut held, first) = fixture::setup();
    held.fixture.edit("https://api.github.com/repos/PerishLab/plumb/tags?per_page=100&page=1", |tags| tags.as_array_mut().unwrap().push(json!({"name":"v0.140.0-rc.1","commit":{"sha":"a".repeat(40),"url":format!("https://api.github.com/repos/PerishLab/plumb/commits/{}","a".repeat(40))}})));
    let fault = held.inspector().selected(&first.root, "plumb").unwrap_err();
    assert_eq!(fault.state, State::Unknown);
    assert!(fault.reason.contains("complete current tag inventory"));
}

struct Moving {
    held: fixture::Inputs,
    reads: Cell<u8>,
}

impl Read for Moving {
    fn bytes(&self, url: &str, limit: usize) -> Result<Vec<u8>, Fault> {
        if url == Selector::url("plumb").unwrap() {
            self.reads.set(self.reads.get() + 1);
            if self.reads.get() > 1 {
                return Err(Fault::unknown("current selector moved"));
            }
        }
        self.held.fixture.bytes(url, limit)
    }
    fn image(&self, reference: &str) -> Result<String, Fault> {
        self.held.fixture.image(reference)
    }
}

#[test]
fn moved() {
    let (held, first) = fixture::setup();
    let inspector = Inspector::new(Moving {
        held,
        reads: Cell::new(0),
    });
    assert!(
        inspector
            .selected(&first.root, "plumb")
            .unwrap_err()
            .reason
            .contains("selector moved")
    );
}

#[test]
fn retargeted() {
    let (mut held, first) = fixture::setup();
    let mut checkpoint = Checkpoint::parse(&held.fixture.values[&held.history.url]).unwrap();
    checkpoint
        .markers
        .iter_mut()
        .find(|tag| tag.marker == first.marker)
        .unwrap()
        .commit = "f".repeat(40);
    let altered = super::super::retained(&mut held.fixture, &checkpoint);
    let mut selection = first;
    selection.history = altered;
    assert!(held.inspector().nominate(&selection, "plumb").is_err());
}

#[test]
fn inception() {
    let (mut held, first) = fixture::setup();
    held.fixture
        .values
        .remove(&Selector::inception("plumb").unwrap());
    let inspector = Inspector::new(fixture::Initial(held));
    assert_eq!(
        inspector.selected(&first.root, "plumb").unwrap_err().state,
        State::Unknown
    );
}

#[test]
fn replacement() {
    let (mut held, mut selection) = fixture::setup();
    selection.native.run += 9;
    selection.native.job += 9;
    fixture::issued(&mut held, &selection);
    fixture::publish(&mut held, &selection);
    assert!(
        held.inspector()
            .selected(&selection.root, "plumb")
            .unwrap_err()
            .reason
            .contains("original inception")
    );
    assert!(
        held.inspector()
            .nominate(&selection, "plumb")
            .unwrap_err()
            .reason
            .contains("original inception")
    );
}
