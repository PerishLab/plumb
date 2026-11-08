use super::fixture::Fixture;
use plumb::semantics::admission::Inspector;
use plumb::windtun::qualification::{Fault, Read, State};
use serde_json::{Value, json};
use std::cell::Cell;

const FIRST: &str = "https://api.github.com/repos/PerishLab/plumb/tags?per_page=100&page=1";

struct Source {
    fixture: Fixture,
    moving: bool,
    reads: Cell<usize>,
}

impl Source {
    fn new(count: usize) -> Self {
        let mut fixture = Fixture::new();
        for offset in (0..=count).step_by(100) {
            let entries = (offset..(offset + 100).min(count))
                .map(|number| Self::tag(&format!("v0.{}.0", number + 1)))
                .collect::<Vec<_>>();
            fixture.put(
                &format!(
                    "https://api.github.com/repos/PerishLab/plumb/tags?per_page=100&page={}",
                    offset / 100 + 1
                ),
                json!(entries),
            );
        }
        Self {
            fixture,
            moving: false,
            reads: Cell::new(0),
        }
    }

    fn tag(name: &str) -> Value {
        json!({"name":name,"commit":{"sha":"a".repeat(40),"url":format!("https://api.github.com/repos/PerishLab/plumb/commits/{}", "a".repeat(40))}})
    }
}

impl Read for Source {
    fn bytes(&self, url: &str, limit: usize) -> Result<Vec<u8>, Fault> {
        let raw = self.fixture.bytes(url, limit)?;
        if url == FIRST {
            self.reads.set(self.reads.get() + 1);
            if self.moving && self.reads.get() > 1 {
                let mut value: Value = serde_json::from_slice(&raw).unwrap();
                value[0] = Self::tag("v0.1000.0");
                return Ok(serde_json::to_vec(&value).unwrap());
            }
        }
        Ok(raw)
    }

    fn image(&self, reference: &str) -> Result<String, Fault> {
        self.fixture.image(reference)
    }
}

#[test]
fn inventory() {
    let mut source = Source::new(101);
    source.fixture.edit(
        "https://api.github.com/repos/PerishLab/plumb/tags?per_page=100&page=2",
        |value| value.as_array_mut().unwrap().push(Source::tag("snapshot")),
    );
    let observed = Inspector::new(source).markers("plumb").unwrap();
    assert_eq!(observed.repository["full_name"], "PerishLab/plumb");
    assert_eq!(observed.markers.len(), 101);
    assert!(observed.markers.iter().any(|tag| tag.marker == "v0.101.0"));
    assert!(
        observed
            .markers
            .iter()
            .all(|tag| tag.commit == "a".repeat(40))
    );
}

#[test]
fn empty() {
    assert!(
        Inspector::new(Source::new(0))
            .markers("plumb")
            .unwrap()
            .markers
            .is_empty()
    );
}

#[test]
fn incomplete() {
    let mut source = Source::new(100);
    source
        .fixture
        .values
        .remove("https://api.github.com/repos/PerishLab/plumb/tags?per_page=100&page=2");
    let fault = Inspector::new(source).markers("plumb").unwrap_err();
    assert_eq!(fault.state, State::Unknown);
    let mut source = Source::new(1);
    source.fixture.edit(FIRST, |held| {
        held.as_array_mut().unwrap().push(Source::tag("v0.1.0"))
    });
    let fault = Inspector::new(source).markers("plumb").unwrap_err();
    assert_eq!(fault.state, State::Unknown);
    assert!(fault.reason.contains("repeat a marker"));
}

#[test]
fn boundaries() {
    assert_eq!(
        Inspector::new(Source::new(4096))
            .markers("plumb")
            .unwrap()
            .markers
            .len(),
        4096
    );
    assert_eq!(
        Inspector::new(Source::new(4097))
            .markers("plumb")
            .unwrap_err()
            .state,
        State::Unknown
    );
    for page in [
        json!(null),
        json!({}),
        json!(vec![Source::tag("v0.1.0"); 101]),
    ] {
        let mut source = Source::new(0);
        source.fixture.put(FIRST, page);
        assert!(Inspector::new(source).markers("plumb").is_err());
    }
}

#[test]
fn malformed() {
    for (field, value) in [
        ("name", json!(null)),
        ("name", json!("v0.1.0+build")),
        ("name", json!("v0.1.0-rc.0")),
        ("commit", json!({"sha":"foreign","url":"foreign"})),
        (
            "commit",
            json!({"sha":"a".repeat(40),"url":"https://api.github.com/repos/foreign/project/commits/foreign"}),
        ),
    ] {
        let mut source = Source::new(1);
        source.fixture.edit(FIRST, |held| held[0][field] = value);
        assert!(Inspector::new(source).markers("plumb").is_err());
    }
    let mut source = Source::new(1);
    let raw = String::from_utf8(source.fixture.values[FIRST].clone())
        .unwrap()
        .replace("\"name\":", "\"name\":\"v0.9.0\",\"name\":");
    source.fixture.values.insert(FIRST.into(), raw.into_bytes());
    assert!(Inspector::new(source).markers("plumb").is_err());
}

#[test]
fn freshness() {
    let mut source = Source::new(1);
    source.moving = true;
    let fault = Inspector::new(source).markers("plumb").unwrap_err();
    assert_eq!(fault.state, State::Unknown);
    assert!(fault.reason.contains("moved during inventory"));
    let mut source = Source::new(1);
    source
        .fixture
        .edit("https://api.github.com/repos/PerishLab/plumb", |value| {
            value["owner"]["id"] = 1.into()
        });
    assert!(Inspector::new(source).markers("plumb").is_err());
}
