use super::*;
use crate::windtun::qualification::State;
use std::cell::Cell;

struct Source(Cell<usize>, Cell<bool>);

impl Read for Source {
    fn bytes(&self, _: &str, _: usize) -> Result<Vec<u8>, Fault> {
        self.0.set(self.0.get() + 1);
        Ok(vec![if self.1.get() { 2 } else { 1 }; 4])
    }

    fn optional(&self, _: &str, _: usize) -> Result<Option<Vec<u8>>, Fault> {
        self.0.set(self.0.get() + 1);
        Ok(self.1.get().then(|| vec![1; 4]))
    }

    fn image(&self, _: &str) -> Result<String, Fault> {
        self.0.set(self.0.get() + 1);
        Ok(if self.1.get() {
            "sha256:BODY_NEVER_SHOW"
        } else {
            "sha256:digest"
        }
        .into())
    }
}

#[test]
fn distinct() {
    let source = Source(Cell::new(0), Cell::new(false));
    let held = Observation::new(&source);
    held.bytes("address", 4).unwrap();
    held.optional("address", 4).unwrap();
    held.image("address").unwrap();
    held.bytes("address", 8).unwrap();
    held.bytes("address", 4).unwrap();
    assert_eq!(source.0.get(), 4);
    held.confirm().unwrap();
    assert_eq!(source.0.get(), 8);
}

#[test]
fn entries() {
    let source = Source(Cell::new(0), Cell::new(false));
    let held = Observation::new(&source);
    for number in 0..Observation::ENTRIES {
        held.bytes(&number.to_string(), 4).unwrap();
    }
    let fault = held.bytes("exhausted", 4).unwrap_err();
    assert_eq!(fault.state, State::Unknown);
    assert_eq!(source.0.get(), Observation::ENTRIES);
}

#[test]
fn bytes() {
    let source = Source(Cell::new(0), Cell::new(false));
    let held = Observation::new(&source);
    assert_eq!(
        held.bytes("oversized", 3).unwrap_err().state,
        State::Unknown
    );
    assert!(held.materials.borrow().values.is_empty());
    held.materials.borrow_mut().size = Observation::BYTES - 4;
    assert_eq!(
        held.bytes("aggregate", 4).unwrap_err().state,
        State::Unknown
    );
    assert!(held.materials.borrow().values.is_empty());
}

#[test]
fn deadline() {
    let source = Source(Cell::new(0), Cell::new(false));
    let mut held = Observation::new(&source);
    held.bytes("address", 4).unwrap();
    held.started -= Observation::TIME;
    assert_eq!(held.bytes("address", 4).unwrap_err().state, State::Unknown);
    assert_eq!(held.confirm().unwrap_err().state, State::Unknown);
    assert_eq!(source.0.get(), 1);
}

#[test]
fn presence() {
    let source = Source(Cell::new(0), Cell::new(false));
    let held = Observation::new(&source);
    assert_eq!(held.optional("address", 4).unwrap(), None);
    source.1.set(true);
    assert_eq!(held.confirm().unwrap_err().state, State::Refused);
    assert_eq!(source.0.get(), 2);
}

#[test]
fn movement() {
    let address = "https://example.invalid/private?token=ADDRESS_NEVER_SHOW";
    for (method, limit) in [("bytes", 4), ("bytes", 8), ("optional", 4), ("image", 4096)] {
        let source = Source(Cell::new(0), Cell::new(false));
        let held = Observation::new(&source);
        match method {
            "bytes" => {
                held.bytes(address, limit).unwrap();
            }
            "optional" => {
                held.optional(address, limit).unwrap();
            }
            _ => {
                held.image(address).unwrap();
            }
        }
        source.1.set(true);
        let fault = held.confirm().unwrap_err();
        assert_eq!(fault.state, State::Refused);
        assert!(
            fault
                .reason
                .starts_with("semantic observation moved before final confirmation")
        );
        assert!(
            fault
                .reason
                .contains(&format!("[{method}; limit:{limit}; address sha256:"))
        );
        assert!(
            fault
                .reason
                .contains("c223d589f42af225b7ae88cbb37dd62a40baeec71a97747abab974067d398c3c")
        );
        assert!(!fault.reason.contains("NEVER_SHOW"));
        assert!(!fault.reason.contains("example.invalid"));
        assert!(fault.reason.len() < 240);
        assert_eq!(source.0.get(), 2);
    }
}
