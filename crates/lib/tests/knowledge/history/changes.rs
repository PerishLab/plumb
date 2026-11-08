use super::*;
use plumb::windtun::qualification::{Fault, Read, State};
use std::cell::Cell;

#[test]
fn ancestry() {
    for field in ["root", "sequence", "previous", "repository"] {
        let (mut fixture, checkpoint, reference) = setup();
        let mut next = append(&checkpoint, &reference);
        match field {
            "root" => next.root = reference.clone(),
            "sequence" => next.sequence += 1,
            "previous" => next.previous = Some(checkpoint.root.clone()),
            "repository" => next.repository["id"] = 11.into(),
            _ => unreachable!(),
        }
        let head = retained(&mut fixture, &next);
        assert!(
            Inspector::new(fixture).history(&head, "plumb").is_err(),
            "{field}"
        );
    }
    let (mut fixture, checkpoint, reference) = setup();
    let next = append(&checkpoint, &reference);
    let head = retained(&mut fixture, &next);
    fixture.values.remove(&reference.url);
    let fault = Inspector::new(fixture).history(&head, "plumb").unwrap_err();
    assert_eq!(fault.state, State::Unknown);
}

#[test]
fn deleted() {
    for retarget in [false, true] {
        let (mut fixture, checkpoint, reference) = setup();
        let mut next = append(&checkpoint, &reference);
        if retarget {
            next.markers[1].commit = "f".repeat(40);
        } else {
            next.markers.pop();
        }
        let head = retained(&mut fixture, &next);
        let fault = Inspector::new(fixture).history(&head, "plumb").unwrap_err();
        assert!(fault.reason.contains("erased or retargeted"));
    }
}

#[test]
fn native() {
    for (field, value) in [
        ("name", json!("[issue] version semantics")),
        ("conclusion", json!("failure")),
        ("steps", json!([])),
        ("run_attempt", json!(2)),
    ] {
        let (mut fixture, _, reference) = setup();
        fixture.edit("https://api.github.com/repos/PerishLab/wharf/actions/runs/40/attempts/1/jobs?per_page=100", |held| held["jobs"][0][field] = value);
        assert!(
            Inspector::new(fixture)
                .history(&reference, "plumb")
                .is_err(),
            "{field}"
        );
    }
    let (mut fixture, checkpoint, _) = setup();
    let mut changed = checkpoint;
    changed.markers[0].commit = "e".repeat(40);
    let reference = retained(&mut fixture, &changed);
    assert!(
        Inspector::new(fixture)
            .history(&reference, "plumb")
            .is_err()
    );
}

#[test]
fn bounds() {
    let (mut fixture, mut checkpoint, mut reference) = setup();
    for _ in 0..128 {
        checkpoint = append(&checkpoint, &reference);
        reference = retained(&mut fixture, &checkpoint);
    }
    let fault = Inspector::new(fixture)
        .history(&reference, "plumb")
        .unwrap_err();
    assert_eq!(fault.state, State::Unknown);
    assert!(fault.reason.contains("bounds"));
}

struct Moving {
    fixture: Fixture,
    url: String,
    count: Cell<usize>,
}

impl Read for Moving {
    fn bytes(&self, url: &str, limit: usize) -> Result<Vec<u8>, Fault> {
        if url == self.url {
            self.count.set(self.count.get() + 1);
            if self.count.get() > 1 {
                return Err(Fault::unknown("checkpoint moved"));
            }
        }
        self.fixture.bytes(url, limit)
    }

    fn image(&self, reference: &str) -> Result<String, Fault> {
        self.fixture.image(reference)
    }
}

#[test]
fn freshness() {
    let (fixture, _, reference) = setup();
    let reader = Moving {
        fixture,
        url: reference.url.clone(),
        count: Cell::new(0),
    };
    let fault = Inspector::new(reader)
        .history(&reference, "plumb")
        .unwrap_err();
    assert_eq!(fault.state, State::Unknown);
    let (mut fixture, _, reference) = setup();
    fixture.moved = true;
    assert!(
        Inspector::new(fixture)
            .history(&reference, "plumb")
            .is_err()
    );
    let (fixture, _, reference) = setup();
    assert!(
        Inspector::new(fixture)
            .history(&reference, "concord")
            .is_err()
    );
}

#[test]
fn discovery() {
    use plumb::semantics::admission::Discovery;
    let (mut fixture, _, reference) = setup();
    let head = Discovery {
        schema: Discovery::SCHEMA.into(),
        reference,
    };
    let url = Discovery::url("plumb").unwrap();
    fixture
        .values
        .insert(url.clone(), head.encode("plumb").unwrap());
    let reader = Moving {
        fixture,
        url,
        count: Cell::new(0),
    };
    let fault = Inspector::new(reader).discover("plumb").unwrap_err();
    assert_eq!(fault.state, State::Unknown);
    assert_eq!(fault.reason, "checkpoint moved");
}
