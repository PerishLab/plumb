use plumb::lane::evidence::{
    Confirmation, Observation, Reason, Record, Reference, Selection, Verified,
};
use plumb::lane::operation::{Material, Operation, Outcome, Request, State, Writer};
use plumb::lane::{Action, Atom, Digest};
use serde_json::json;

fn reference(record: &Record) -> Reference {
    Reference::new(Atom::read("wharf").unwrap(), record.digest().unwrap()).unwrap()
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

fn pair() -> (Record, Record) {
    let operation = super::operation::operation();
    let unknown = Operation::new(
        operation.request().clone(),
        operation.assessment().clone(),
        Outcome::Unknown {
            reason: Reason::Unverified,
        },
    )
    .unwrap();
    let original = Record::execution(unknown);
    let authorization = Reference::new(
        Atom::read("wharf").unwrap(),
        Digest::read(&"d".repeat(64)).unwrap(),
    )
    .unwrap();
    let confirmation = Confirmation::new(reference(&original), authorization, operation).unwrap();
    (original, Record::confirmed(confirmation))
}

fn verified(record: &Record) -> Verified {
    let held = reference(record);
    held.verify(&held, &record.encode().unwrap()).unwrap()
}

#[test]
fn confirming() {
    let (original, record) = pair();
    let requested = super::operation::request();
    let observed = verified(&record);
    let receipt = observed.confirm(&requested, &verified(&original)).unwrap();
    assert_eq!(receipt.reference(), observed.reference());
    assert_eq!(receipt.confirmation(), record.confirmation().unwrap());
    assert_eq!(receipt.confirmation().original(), &reference(&original));
    assert_eq!(receipt.confirmation().operation().request(), &requested);
    assert!(record.operation().is_none());
    assert!(observed.settle(&requested).is_err());
    assert!(verified(&original).settle(&requested).is_err());
    assert!(verified(&original).confirm(&requested, &observed).is_err());
}

#[test]
fn canonical() {
    let (original, record) = pair();
    assert_eq!(
        original.digest().unwrap().value(),
        "74a9ca8335473f0f664d0a59200613a5f7e29e17ba24b5310b42c6def4fd8034"
    );
    assert_eq!(
        record.digest().unwrap().value(),
        "d3dbdf2186325bbf291765669fe645a45eb20f5e48eb0a1fdae93a0b4670f51c"
    );
    let held = record.confirmation().unwrap();
    let expected = format!(
        "{{\"schema\":3,\"entry\":{{\"kind\":\"confirmation\",\"value\":{{\"original\":{},\"authorization\":{},\"operation\":{}}}}}}}",
        serde_json::to_string(held.original()).unwrap(),
        serde_json::to_string(held.authorization()).unwrap(),
        serde_json::to_string(held.operation()).unwrap(),
    );
    assert_eq!(record.encode().unwrap(), expected.as_bytes());
    assert_eq!(verified(&record).record(), &record);
    let value = serde_json::to_value(&record).unwrap();
    let reordered = serde_json::to_vec(&value).unwrap();
    assert_ne!(reordered, record.encode().unwrap());
    let held = Reference::new(
        Atom::read("wharf").unwrap(),
        Digest::read(&plumb::depot::sha(&reordered)).unwrap(),
    )
    .unwrap();
    assert!(held.verify(&held, &reordered).is_err());
}

#[test]
fn versions() {
    let (original, record) = pair();
    for (held, schema) in [(&original, 3), (&record, 2), (&record, 4)] {
        let mut value = serde_json::to_value(held).unwrap();
        value["schema"] = json!(schema);
        assert!(serde_json::from_value::<Record>(value).is_err());
    }
    let mut value = serde_json::to_value(&record).unwrap();
    value["entry"]["unexpected"] = json!(true);
    assert!(serde_json::from_value::<Record>(value).is_err());
    let bytes = String::from_utf8(record.encode().unwrap()).unwrap();
    let duplicate = bytes.replacen("\"schema\":3", "\"schema\":3,\"schema\":3", 1);
    assert!(serde_json::from_str::<Record>(&duplicate).is_err());
}

#[test]
fn invariants() {
    let (_, record) = pair();
    let value = serde_json::to_value(&record).unwrap();
    let entry = &value["entry"]["value"];
    for change in [
        json!({"authorization":entry["original"]}),
        json!({"authorization":entry["operation"]["assessment"]["conditions"]["authorization"]["evidence"]}),
        json!({"authorization":{"authority":"other","digest":"d".repeat(64)}}),
        json!({"original":{"authority":"wharf","digest":"bad"}}),
        json!({"unexpected":true}),
    ] {
        let mut changed = value.clone();
        for (key, item) in change.as_object().unwrap() {
            changed["entry"]["value"][key] = item.clone();
        }
        assert!(serde_json::from_value::<Record>(changed).is_err());
    }
    for outcome in [
        json!({"outcome":"unknown","reason":"unverified"}),
        json!({"outcome":"refused","reason":"conflict"}),
        json!({"outcome":"applied","material":{"state":"absent"},"writer":"idle"}),
    ] {
        let mut changed = value.clone();
        changed["entry"]["value"]["operation"]["outcome"] = outcome;
        assert!(serde_json::from_value::<Record>(changed).is_err());
    }
    let mut inspect = value;
    inspect["entry"]["value"]["operation"]["request"]["intent"] = json!({"action":"inspect"});
    inspect["entry"]["value"]["operation"]["request"]["expected"] = serde_json::Value::Null;
    let request: Request =
        serde_json::from_value(inspect["entry"]["value"]["operation"]["request"].clone()).unwrap();
    let assessment = super::operation::bound(&request, Action::Inspect, State::Absent {});
    let operation = Operation::new(
        request,
        assessment,
        Outcome::Applied {
            material: Material::Absent {},
            writer: Writer::Idle,
        },
    )
    .unwrap();
    let confirmation = record.confirmation().unwrap();
    assert!(
        Confirmation::new(
            confirmation.original().clone(),
            confirmation.authorization().clone(),
            operation
        )
        .is_err()
    );
}

#[test]
fn links() {
    let (original, record) = pair();
    let requested = super::operation::request();
    let mut changed = serde_json::to_value(&record).unwrap();
    changed["entry"]["value"]["original"]["digest"] = json!("a".repeat(64));
    let changed: Record = serde_json::from_value(changed).unwrap();
    assert!(
        verified(&changed)
            .confirm(&requested, &verified(&original))
            .is_err()
    );
    let mut changed = serde_json::to_value(&record).unwrap();
    changed["entry"]["value"]["operation"]["assessment"]["conditions"]["authorization"]["evidence"]
        ["digest"] = json!("a".repeat(64));
    let changed: Record = serde_json::from_value(changed).unwrap();
    assert!(
        verified(&changed)
            .confirm(&requested, &verified(&original))
            .is_err()
    );
    let mut foreign = serde_json::to_value(&requested).unwrap();
    foreign["context"]["revision"] = json!(3);
    let foreign: Request = serde_json::from_value(foreign).unwrap();
    assert!(
        verified(&record)
            .confirm(&foreign, &verified(&original))
            .is_err()
    );
    let held = record.confirmation().unwrap();
    let operation = Operation::new(
        foreign.clone(),
        super::operation::bound(&foreign, Action::Deploy, State::Absent {}),
        super::operation::applied(),
    )
    .unwrap();
    let value = Confirmation::new(
        held.original().clone(),
        held.authorization().clone(),
        operation,
    )
    .unwrap();
    assert!(
        verified(&Record::confirmed(value))
            .confirm(&foreign, &verified(&original))
            .is_err()
    );
    let other = Reference::new(Atom::read("other").unwrap(), record.digest().unwrap()).unwrap();
    let foreign = other.verify(&other, &record.encode().unwrap()).unwrap();
    assert!(foreign.confirm(&requested, &verified(&original)).is_err());
    let selection = Selection::new(
        requested.context().clone(),
        Observation::Unknown {
            reason: Reason::Unverified,
        },
    )
    .unwrap();
    let selected = Record::new(selection);
    let held = record.confirmation().unwrap();
    let value = Confirmation::new(
        reference(&selected),
        held.authorization().clone(),
        held.operation().clone(),
    )
    .unwrap();
    assert!(
        verified(&Record::confirmed(value))
            .confirm(&requested, &verified(&selected))
            .is_err()
    );
    for outcome in [
        super::operation::applied(),
        Outcome::Refused {
            reason: Reason::Conflict,
        },
    ] {
        let operation = Operation::new(
            requested.clone(),
            super::operation::assessment(Action::Deploy, State::Absent {}),
            outcome,
        )
        .unwrap();
        let original = Record::execution(operation);
        let held = record.confirmation().unwrap();
        let value = Confirmation::new(
            super::receipt::reference(&original),
            held.authorization().clone(),
            held.operation().clone(),
        )
        .unwrap();
        let changed = Record::confirmed(value);
        assert!(
            verified(&changed)
                .confirm(&requested, &verified(&original))
                .is_err()
        );
    }
}
