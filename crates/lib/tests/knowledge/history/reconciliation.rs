use super::family::fixture::Inputs;
use plumb::semantics::Decision;
use plumb::semantics::admission::{Checkpoint, Discovery, Record, Tag, Transition};
use serde_json::{Value, json};

struct Cumulative(Inputs);

impl Cumulative {
    fn new(marker: &str, decision: Decision) -> Self {
        let mut inputs = Inputs::new();
        inputs.append(marker, decision);
        inputs.present();
        Self(inputs)
    }

    fn record(&self) -> Transition {
        let Record::Transition(record) =
            Record::parse(&self.0.fixture.values[&self.0.reference.url]).unwrap()
        else {
            panic!("transition")
        };
        *record
    }

    fn retain(&mut self, markers: &[&str]) {
        let original = Checkpoint::parse(&self.0.fixture.values[&self.0.history.url]).unwrap();
        let mut checkpoint = super::append(&original, &self.0.history);
        let record = self.record();
        for marker in markers {
            checkpoint.markers.push(Tag {
                marker: (*marker).into(),
                commit: record.intent.request.candidate.source.commit.clone(),
            });
        }
        checkpoint.markers.sort_by(|a, b| a.marker.cmp(&b.marker));
        self.0.history = super::retained(&mut self.0.fixture, &checkpoint);
        let history = self.0.history.clone();
        self.0.edit(|record| {
            record.history = history;
            record.intent.request.reservations = checkpoint
                .markers
                .iter()
                .filter(|tag| tag.marker != record.intent.marker)
                .map(|tag| tag.marker.clone())
                .collect();
        });
        let head = Discovery {
            schema: Discovery::SCHEMA.into(),
            reference: self.0.history.clone(),
        };
        self.0.fixture.values.insert(
            Discovery::url("plumb").unwrap(),
            head.encode("plumb").unwrap(),
        );
        let tags: Vec<_> = checkpoint
            .markers
            .iter()
            .map(|tag| {
                json!({
                    "name":tag.marker,"commit":{"sha":tag.commit,"url":format!(
                        "https://api.github.com/repos/PerishLab/plumb/commits/{}",tag.commit
                    )}
                })
            })
            .collect();
        self.0.fixture.put(
            "https://api.github.com/repos/PerishLab/plumb/tags?per_page=100&page=1",
            json!(tags),
        );
    }

    fn historical(&self) -> Value {
        serde_json::to_value(
            self.0
                .inspector()
                .historical(&self.record(), "plumb")
                .unwrap(),
        )
        .unwrap()
    }
}

#[test]
fn cumulative() {
    let mut held = Cumulative::new("v0.138.1", Decision::Same);
    held.retain(&["v0.139.0-rc.1", "v0.139.0", "v0.139.1"]);
    let record = held.record();
    assert!(record.intent.request.assess().is_err());
    let historical = held.historical();
    assert_eq!(historical["freshness"], "historical");
    assert_eq!(
        historical["derived"]["judgment"],
        "historical_reconciliation"
    );
    assert_eq!(historical["derived"]["admission"], "unknown");
    assert_eq!(historical["derived"]["authentication"], false);
    let family = held
        .0
        .inspector()
        .family(&held.0.reference, "plumb")
        .unwrap();
    assert_eq!(family.admission(), "unknown");
    assert_eq!(family.anchor(), family.root());
    assert_eq!(family.stable().world().source.marker, "v0.138.1");
    assert_eq!(held.record().intent.request, record.intent.request);
    assert_eq!(held.record().history, held.0.history);
    assert!(held.0.inspector().transition(&record, "plumb").is_err());
}

#[test]
fn anchor() {
    let mut held = Cumulative::new("v0.139.0", Decision::Changed);
    held.retain(&["v0.140.0-rc.1", "v0.140.0"]);
    let anchor = held.0.reference.clone();
    let root = held
        .0
        .inspector()
        .family(&anchor, "plumb")
        .unwrap()
        .root()
        .clone();
    held.0.append("v0.139.1", Decision::Same);
    let family = held
        .0
        .inspector()
        .family(&held.0.reference, "plumb")
        .unwrap();
    assert_eq!(family.root(), &root);
    assert_eq!(family.anchor(), &anchor);
    assert_eq!(family.original().world().source.marker, "v0.139.0");
    assert_eq!(family.stable().world().source.marker, "v0.139.1");
    assert_eq!(family.admission(), "unknown");
}

#[test]
fn gap() {
    let mut held = Cumulative::new("v0.140.0", Decision::Changed);
    held.retain(&["v0.139.0", "v0.141.0"]);
    assert!(
        held.0
            .inspector()
            .historical(&held.record(), "plumb")
            .is_err()
    );
}

#[test]
fn reservations() {
    let mut held = Cumulative::new("v0.138.1", Decision::Same);
    held.retain(&["v0.139.0"]);
    held.0.edit(|record| {
        record
            .intent
            .request
            .reservations
            .retain(|m| m != "v0.139.0")
    });
    let fault = held
        .0
        .inspector()
        .historical(&held.record(), "plumb")
        .unwrap_err();
    assert!(fault.reason.contains("independently derived inputs"));
}

#[test]
fn current() {
    let mut held = Cumulative::new("v0.138.1", Decision::Same);
    held.retain(&["v0.139.0-rc.1"]);
    assert_eq!(held.historical()["derived"]["admission"], "unknown");
    let fault = held
        .0
        .inspector()
        .transition(&held.record(), "plumb")
        .unwrap_err();
    assert!(fault.reason.contains("complete reservation judgment"));
}

#[test]
fn major() {
    let mut held = Cumulative::new("v1.0.0", Decision::Changed);
    held.retain(&["v1.1.0"]);
    let fault = held
        .0
        .inspector()
        .historical(&held.record(), "plumb")
        .unwrap_err();
    assert_eq!(fault.state, plumb::windtun::qualification::State::Unknown);
    assert!(fault.reason.contains("user's decision"));
}
