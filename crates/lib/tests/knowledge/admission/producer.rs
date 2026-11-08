use super::fixture::{Fixture, WHARF};
use plumb::semantics::admission::{Inspector, Record, Reference};
use plumb::windtun::qualification::{Fault, Read, State};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::cell::Cell;

const RUN: &str = "https://api.github.com/repos/PerishLab/wharf/actions/runs/32/attempts/1";

struct Progress {
    source: Fixture,
    reads: Cell<usize>,
}

impl Read for Progress {
    fn bytes(&self, url: &str, limit: usize) -> Result<Vec<u8>, Fault> {
        let raw = self.source.bytes(url, limit)?;
        if url == RUN {
            self.reads.set(self.reads.get() + 1);
            let mut value: Value = serde_json::from_slice(&raw).unwrap();
            value["status"] = if self.reads.get() == 1 {
                "in_progress"
            } else {
                "completed"
            }
            .into();
            value["updated_at"] = self.reads.get().into();
            return Ok(serde_json::to_vec(&value).unwrap());
        }
        Ok(raw)
    }

    fn image(&self, reference: &str) -> Result<String, Fault> {
        self.source.image(reference)
    }
}

pub(super) fn setup() -> (Fixture, Reference) {
    let mut fixture = Fixture::new();
    let reference = json!({"url":format!("https://releases.plumb.perish.uk/v1/semantics/objects/{}.json", "b".repeat(64)),"sha256":"b".repeat(64),"size":100});
    let mut raw = json!({"schema":"wharf.release.semantics.publication/v1", "product":"plumb","marker":"v0.139.0", "intent":reference,"world":reference,"native":{"run":32,"attempt":1,"commit":"a".repeat(40),"job":77}});
    raw.sort_all_objects();
    let raw = serde_json::to_vec(&raw).unwrap();
    let record = Record::parse(&raw).unwrap();
    let bytes = record.encode().unwrap();
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let reference = Reference {
        url: format!("https://releases.plumb.perish.uk/v1/semantics/objects/{digest}.json"),
        sha256: digest.clone(),
        size: bytes.len() as u64,
    };
    fixture.values.insert(reference.url.clone(), bytes);
    let repository =
        json!({"id":1374358737_u64,"node_id":"R_kgDOUesM0Q","full_name":"PerishLab/wharf"});
    fixture.put(RUN, json!({"id":32,"run_attempt":1,"head_sha":"a".repeat(40),"head_branch":"main","event":"workflow_dispatch","path":".github/workflows/ship.yml","status":"completed","conclusion":"success","repository":repository,"head_repository":repository}));
    fixture.put(&format!("{RUN}/jobs?per_page=100"), json!({"total_count":1,"jobs":[{"id":77,"run_id":32,"run_attempt":1,"head_sha":"a".repeat(40),"head_branch":"main","name":"[issue] version semantics","status":"completed","conclusion":"success","steps":[{"name":format!("[issue] semantics {digest}"),"status":"completed","conclusion":"success"}]}]}));
    fixture.put(&format!("{WHARF}/compare/{}...{}", "a".repeat(40), "a".repeat(40)), json!({"base_commit":{"sha":"a".repeat(40)},"merge_base_commit":{"sha":"a".repeat(40)},"status":"identical"}));
    (fixture, reference)
}

#[test]
fn origin() {
    let (fixture, reference) = setup();
    let observed = Inspector::new(fixture).record(&reference, "plumb").unwrap();
    assert!(observed.authentication);
    assert!(!observed.publication);
    assert_eq!(observed.record.marker(), "v0.139.0");
    assert_eq!(observed.reference, reference);
    assert_eq!(observed.job["id"], 77);
    assert!(observed.authority.requirements.scopes.is_empty());
}

#[test]
fn progress() {
    let (source, reference) = setup();
    let reader = Progress {
        source,
        reads: Cell::new(0),
    };
    let observed = Inspector::new(reader).record(&reference, "plumb").unwrap();
    assert_eq!(observed.producer["status"], "in_progress");
    assert!(observed.authentication);
    assert!(!observed.publication);
}

#[test]
fn run() {
    for (field, value) in [
        ("id", json!(33)),
        ("run_attempt", json!(2)),
        ("head_sha", json!("b".repeat(40))),
        ("head_branch", json!("foreign")),
        ("event", json!("pull_request_target")),
        ("path", json!(".github/workflows/other.yml")),
        ("status", json!("queued")),
        (
            "repository",
            json!({"id":1374358737_u64,"node_id":"foreign","full_name":"PerishLab/wharf"}),
        ),
        (
            "head_repository",
            json!({"id":1,"node_id":"R_kgDOUesM0Q","full_name":"PerishLab/wharf"}),
        ),
    ] {
        let (mut fixture, reference) = setup();
        fixture.edit(RUN, |held| held[field] = value);
        assert!(
            Inspector::new(fixture).record(&reference, "plumb").is_err(),
            "{field}"
        );
    }
}

#[test]
fn role() {
    for (field, value) in [
        ("id", json!(78)),
        ("run_id", json!(33)),
        ("run_attempt", json!(2)),
        ("head_sha", json!("b".repeat(40))),
        ("head_branch", json!("foreign")),
        ("name", json!("[read] original semantics materials")),
        ("status", json!("in_progress")),
        ("conclusion", json!("failure")),
        ("steps", json!([])),
    ] {
        let (mut fixture, reference) = setup();
        fixture.edit(&format!("{RUN}/jobs?per_page=100"), |held| {
            held["jobs"][0][field] = value
        });
        assert!(
            Inspector::new(fixture).record(&reference, "plumb").is_err(),
            "{field}"
        );
    }
}

#[test]
fn confirmation() {
    for (field, value) in [
        (
            "name",
            json!(format!("[issue] semantics {}", "c".repeat(64))),
        ),
        ("status", json!("in_progress")),
        ("conclusion", json!("skipped")),
    ] {
        let (mut fixture, reference) = setup();
        fixture.edit(&format!("{RUN}/jobs?per_page=100"), |held| {
            held["jobs"][0]["steps"][0][field] = value
        });
        assert!(
            Inspector::new(fixture).record(&reference, "plumb").is_err(),
            "{field}"
        );
    }
    let (mut fixture, reference) = setup();
    fixture.edit(&format!("{RUN}/jobs?per_page=100"), |held| {
        let repeated = held["jobs"][0]["steps"][0].clone();
        held["jobs"][0]["steps"]
            .as_array_mut()
            .unwrap()
            .push(repeated);
    });
    assert!(Inspector::new(fixture).record(&reference, "plumb").is_err());
}

#[test]
fn inventory() {
    for value in [json!(0), json!(2), json!(true)] {
        let (mut fixture, reference) = setup();
        fixture.edit(&format!("{RUN}/jobs?per_page=100"), |held| {
            held["total_count"] = value
        });
        assert!(Inspector::new(fixture).record(&reference, "plumb").is_err());
    }
    let (mut fixture, reference) = setup();
    fixture.edit(&format!("{RUN}/jobs?per_page=100"), |held| {
        let repeated = held["jobs"][0].clone();
        held["jobs"].as_array_mut().unwrap().push(repeated);
        held["total_count"] = 2.into();
    });
    assert!(Inspector::new(fixture).record(&reference, "plumb").is_err());
}

#[test]
fn forged() {
    let (mut fixture, mut reference) = setup();
    let mut record: Value = serde_json::from_slice(&fixture.values[&reference.url]).unwrap();
    record["marker"] = "v0.140.0".into();
    let bytes = serde_json::to_vec(&record).unwrap();
    Record::parse(&bytes).unwrap();
    reference.sha256 = format!("{:x}", Sha256::digest(&bytes));
    reference.url = format!(
        "https://releases.plumb.perish.uk/v1/semantics/objects/{}.json",
        reference.sha256
    );
    reference.size = bytes.len() as u64;
    fixture.values.insert(reference.url.clone(), bytes);
    assert!(Inspector::new(fixture).record(&reference, "plumb").is_err());
}

#[test]
fn authority() {
    let (mut fixture, reference) = setup();
    fixture.moved = true;
    assert!(Inspector::new(fixture).record(&reference, "plumb").is_err());
    let (fixture, reference) = setup();
    assert!(
        Inspector::new(fixture)
            .record(&reference, "concord")
            .is_err()
    );
    let (mut fixture, reference) = setup();
    fixture.edit(
        &format!("{WHARF}/compare/{}...{}", "a".repeat(40), "a".repeat(40)),
        |held| held["status"] = "diverged".into(),
    );
    assert!(Inspector::new(fixture).record(&reference, "plumb").is_err());
}

#[test]
fn missing() {
    let (mut fixture, reference) = setup();
    fixture.values.remove(RUN);
    let fault = Inspector::new(fixture)
        .record(&reference, "plumb")
        .unwrap_err();
    assert_eq!(fault.state, State::Unknown);
}
