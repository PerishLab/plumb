use plumb::lane::evidence::{Reason, Record, Reference};
use plumb::lane::operation::{Operation, Outcome, Request, State};
use plumb::lane::{Action, Name};
use serde_json::json;

fn reference(record: &Record) -> Reference {
    Reference::new(Name::read("wharf").unwrap(), record.digest().unwrap()).unwrap()
}

#[test]
fn settlement() {
    let operation = super::operation::operation();
    let record = Record::execution(operation.clone());
    assert!(record.selection().is_none());
    assert_eq!(record.operation(), Some(&operation));
    let held = reference(&record);
    let verified = held.verify(&held, &record.encode().unwrap()).unwrap();
    let receipt = verified.settle(operation.request()).unwrap();
    assert_eq!(receipt.reference(), &held);
    assert_eq!(receipt.operation(), &operation);
    let mut foreign = serde_json::to_value(operation.request()).unwrap();
    foreign["context"]["revision"] = json!(3);
    let foreign: Request = serde_json::from_value(foreign).unwrap();
    assert_ne!(
        foreign.digest().unwrap(),
        operation.request().digest().unwrap()
    );
    assert!(verified.settle(&foreign).is_err());
    assert!(held.verify(&held, b"publication failed").is_err());
}

#[test]
fn unfinished() {
    for outcome in [
        Outcome::Unknown {
            reason: Reason::Unavailable,
        },
        Outcome::Refused {
            reason: Reason::Conflict,
        },
    ] {
        let operation = Operation::new(
            super::operation::request(),
            super::operation::assessment(Action::Deploy, State::Absent {}),
            outcome,
        )
        .unwrap();
        let record = Record::execution(operation.clone());
        let held = reference(&record);
        let verified = held.verify(&held, &record.encode().unwrap()).unwrap();
        assert!(verified.settle(operation.request()).is_err());
    }
    let mut value = serde_json::to_value(Record::execution(super::operation::operation())).unwrap();
    value["schema"] = json!(1);
    assert!(serde_json::from_value::<Record>(value).is_err());
    let legacy = json!({"schema":1,"selection":{}});
    assert!(serde_json::from_value::<Record>(legacy).is_err());
}
