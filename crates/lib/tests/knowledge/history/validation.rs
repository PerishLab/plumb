use super::family::fixture::Inputs;
use plumb::semantics::Decision;
use plumb::semantics::admission::Record;
use serde_json::{Value, json};

#[test]
fn unissued() {
    let mut held = Inputs::new();
    held.append("v0.138.1", Decision::Same);
    held.present();
    let Record::Transition(record) =
        Record::parse(&held.fixture.values[&held.reference.url]).unwrap()
    else {
        panic!("transition")
    };
    let route = format!(
        "https://api.github.com/repos/PerishLab/wharf/actions/runs/{}/attempts/1/jobs?per_page=100",
        record.intent.native.run
    );
    held.fixture.edit(&route, |jobs| {
        jobs["jobs"][0]["conclusion"] = "failure".into()
    });
    let prepared = held.inspector().transition(&record, "plumb").unwrap();
    let value = serde_json::to_value(prepared).unwrap();
    assert_eq!(value["freshness"], "current");
    assert_eq!(value["authentication"], false);
    assert_eq!(value["publication"], false);
    assert_eq!(value["derived"]["authentication"], false);
    assert_eq!(value["previous"]["authentication"], true);
    assert_eq!(
        value["derived"]["stable"]["world"]["source"]["marker"],
        "v0.138.1"
    );
    assert!(held.inspector().family(&held.reference, "plumb").is_err());
}

#[test]
fn freshness() {
    let mut held = Inputs::new();
    held.append("v0.138.1", Decision::Same);
    held.present();
    let Record::Transition(record) =
        Record::parse(&held.fixture.values[&held.reference.url]).unwrap()
    else {
        panic!("transition")
    };
    held.fixture.edit("https://api.github.com/repos/PerishLab/plumb/tags?per_page=100&page=1", |tags| tags.as_array_mut().unwrap().push(json!({"name":"v0.139.0","commit":{"sha":"a".repeat(40),"url":format!("https://api.github.com/repos/PerishLab/plumb/commits/{}","a".repeat(40))}})));
    assert!(
        held.inspector()
            .transition(&record, "plumb")
            .unwrap_err()
            .reason
            .contains("current latest stable")
    );
    let historical =
        serde_json::to_value(held.inspector().historical(&record, "plumb").unwrap()).unwrap();
    assert_eq!(historical["freshness"], "historical");
    assert_eq!(historical["authentication"], false);
    assert_eq!(historical["derived"]["authentication"], false);
}

#[test]
fn discovery() {
    let mut held = Inputs::new();
    held.append("v0.138.1", Decision::Same);
    held.present();
    let Record::Transition(mut record) =
        Record::parse(&held.fixture.values[&held.reference.url]).unwrap()
    else {
        panic!("transition")
    };
    let checkpoint: Value =
        serde_json::from_slice(&held.fixture.values[&record.history.url]).unwrap();
    record.history = serde_json::from_value(checkpoint["previous"].clone()).unwrap();
    assert!(
        held.inspector()
            .transition(&record, "plumb")
            .unwrap_err()
            .reason
            .contains("current retained history")
    );
}

#[test]
fn coverage() {
    let mut held = Inputs::new();
    held.append("v0.138.1", Decision::Same);
    held.present();
    let Record::Transition(record) =
        Record::parse(&held.fixture.values[&held.reference.url]).unwrap()
    else {
        panic!("transition")
    };
    held.fixture.edit("https://api.github.com/repos/PerishLab/plumb/tags?per_page=100&page=1", |tags| tags.as_array_mut().unwrap().push(json!({"name":"v0.139.0-rc.1","commit":{"sha":"a".repeat(40),"url":format!("https://api.github.com/repos/PerishLab/plumb/commits/{}","a".repeat(40))}})));
    let fault = held.inspector().transition(&record, "plumb").unwrap_err();
    assert_eq!(fault.state, plumb::windtun::qualification::State::Unknown);
    assert!(fault.reason.contains("cover current tag inventory"));
}
