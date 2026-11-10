use super::family::fixture::Inputs;
use plumb::semantics::Decision;
use plumb::semantics::admission::{Inspector, Record};
use plumb::windtun::qualification::{Fault, Read, State};
use std::cell::RefCell;
use std::collections::BTreeMap;

struct Trace {
    held: Inputs,
    reads: RefCell<BTreeMap<(String, usize), usize>>,
    changed: Option<String>,
    unavailable: bool,
}

impl Trace {
    fn new() -> Self {
        let mut held = Inputs::new();
        for marker in ["v0.138.1", "v0.138.2", "v0.139.0"] {
            held.append(
                marker,
                if marker == "v0.139.0" {
                    Decision::Changed
                } else {
                    Decision::Same
                },
            );
        }
        held.present();
        Self {
            held,
            reads: RefCell::default(),
            changed: None,
            unavailable: false,
        }
    }
}

impl Read for &Trace {
    fn bytes(&self, url: &str, limit: usize) -> Result<Vec<u8>, Fault> {
        let mut reads = self.reads.borrow_mut();
        let count = reads.entry((url.into(), limit)).or_default();
        *count += 1;
        let mut raw = self.held.fixture.bytes(url, limit)?;
        if self.changed.as_deref() == Some(url) && *count > 1 {
            if self.unavailable {
                return Err(Fault::unknown("fresh observation unavailable"));
            }
            raw.push(b' ');
        }
        Ok(raw)
    }

    fn image(&self, reference: &str) -> Result<String, Fault> {
        self.held.fixture.image(reference)
    }
}

#[test]
fn reuse() {
    let trace = Trace::new();
    let inspector = Inspector::new(&trace);
    let family = inspector.family(&trace.held.reference, "plumb").unwrap();
    assert_eq!(family.stable().world().source.marker, "v0.139.0");
    assert!(trace.reads.borrow().values().all(|count| *count == 2));
    inspector.family(&trace.held.reference, "plumb").unwrap();
    assert!(trace.reads.borrow().values().all(|count| *count == 4));
}

#[test]
fn derivation() {
    let trace = Trace::new();
    let Record::Transition(record) =
        Record::parse(&trace.held.fixture.values[&trace.held.reference.url]).unwrap()
    else {
        panic!("transition")
    };
    let result = Inspector::new(&trace).transition(&record, "plumb").unwrap();
    let value = serde_json::to_value(result).unwrap();
    assert_eq!(value["freshness"], "current");
    assert_eq!(value["previous"]["authentication"], true);
    assert_eq!(value["derived"]["authentication"], false);
    assert!(trace.reads.borrow().values().all(|count| *count <= 4));
}

#[test]
fn changed() {
    for url in [
        "https://api.github.com/repos/PerishLab/wharf/branches/main",
        "https://api.github.com/repos/PerishLab/plumb/collaborators/alice/permission",
        "https://api.github.com/repos/PerishLab/wharf/actions/runs/32/attempts/1/jobs?per_page=100",
    ] {
        let mut trace = Trace::new();
        trace.changed = Some(url.into());
        let fault = Inspector::new(&trace)
            .family(&trace.held.reference, "plumb")
            .unwrap_err();
        assert_eq!(fault.state, State::Refused);
        assert!(fault.reason.contains("moved before final confirmation"));
    }
}

#[test]
fn unavailable() {
    let mut trace = Trace::new();
    trace.changed = Some(trace.held.history.url.clone());
    trace.unavailable = true;
    let fault = Inspector::new(&trace)
        .family(&trace.held.reference, "plumb")
        .unwrap_err();
    assert_eq!(fault.state, State::Unknown);
    assert_eq!(fault.reason, "fresh observation unavailable");
}
