use super::*;
use crate::windtun::qualification::State;
use std::cell::Cell;

struct Source(Cell<usize>, Cell<bool>);

impl Read for Source {
    fn bytes(&self, _: &str, _: usize) -> Result<Vec<u8>, Fault> {
        self.0.set(self.0.get() + 1);
        Ok(vec![1; 4])
    }

    fn optional(&self, _: &str, _: usize) -> Result<Option<Vec<u8>>, Fault> {
        self.0.set(self.0.get() + 1);
        Ok(self.1.get().then(|| vec![1; 4]))
    }

    fn image(&self, _: &str) -> Result<String, Fault> {
        self.0.set(self.0.get() + 1);
        Ok("sha256:digest".into())
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
