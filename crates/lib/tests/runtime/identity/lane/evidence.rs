use plumb::lane::evidence::{Context, Observation, Policy, Reason, Record, Reference, Selection};
use plumb::lane::{Address, Digest, Name};
use serde_json::{Value, json};

fn address(lane: &str) -> Address {
    Address::new(
        "PerishLab/crest".into(),
        "crest".into(),
        Name::read(lane).unwrap(),
    )
    .unwrap()
}

fn context() -> Context {
    Context::new(
        address("preview.a"),
        Digest::read(&"b".repeat(64)).unwrap(),
        1,
        Name::read("static").unwrap(),
    )
    .unwrap()
}

fn matched() -> Record {
    Record::new(
        Selection::new(
            context(),
            Observation::Matched {
                actual: address("preview.a"),
                artifact: Box::new(super::artifact::sample()),
            },
        )
        .unwrap(),
    )
}

fn reference(record: &Record) -> Reference {
    Reference::new(Name::read("wharf").unwrap(), record.digest().unwrap()).unwrap()
}

#[test]
fn roundtrip() {
    let record = matched();
    let bytes = record.encode().unwrap();
    assert_eq!(serde_json::from_slice::<Record>(&bytes).unwrap(), record);
    assert_eq!(record.selection().unwrap().policy(), Policy::Exact);
    assert_eq!(record.selection().unwrap().context().revision(), 1);
    assert_eq!(
        record.selection().unwrap().context().adaptor().value(),
        "static"
    );
    assert_eq!(
        record.selection().unwrap().context().request().value(),
        "b".repeat(64)
    );
    let held = reference(&record);
    assert_eq!(
        serde_json::from_value::<Reference>(serde_json::to_value(&held).unwrap()).unwrap(),
        held
    );
    let verified = held.verify(&held, &bytes).unwrap();
    assert_eq!(verified.reference(), &held);
    assert_eq!(verified.record(), &record);
    assert!(verified.settle(&super::operation::request()).is_err());
    assert_eq!(held.digest(), &record.digest().unwrap());
    assert_eq!(held.authority().value(), "wharf");
    assert_eq!(record.digest().unwrap().value(), plumb::depot::sha(&bytes));
}

#[test]
fn exact() {
    for lane in ["preview", "preview.b", "preview-a"] {
        let observation = Observation::Matched {
            actual: address(lane),
            artifact: Box::new(super::artifact::sample()),
        };
        assert!(Selection::new(context(), observation).is_err());
        let mut value = serde_json::to_value(matched()).unwrap();
        value["entry"]["value"]["observation"]["actual"]["lane"] = json!(lane);
        assert!(serde_json::from_value::<Record>(value).is_err());
    }
    for (field, text) in [("repository", "PerishLab/design"), ("app", "design")] {
        let mut value = serde_json::to_value(matched()).unwrap();
        value["entry"]["value"]["observation"]["actual"][field] = json!(text);
        assert!(serde_json::from_value::<Record>(value).is_err());
    }
    let mut value = serde_json::to_value(matched()).unwrap();
    value["entry"]["value"]["policy"] = json!("stable-latest");
    assert!(serde_json::from_value::<Record>(value).is_err());
}

#[test]
fn uncertainty() {
    for reason in [
        Reason::Missing,
        Reason::Unsupported,
        Reason::Denied,
        Reason::Conflict,
    ] {
        let selection = Selection::new(context(), Observation::Unmet { reason }).unwrap();
        let record = Record::new(selection);
        assert_eq!(
            serde_json::from_slice::<Record>(&record.encode().unwrap()).unwrap(),
            record
        );
        assert!(Selection::new(context(), Observation::Unknown { reason }).is_err());
    }
    for reason in [Reason::Unavailable, Reason::Unverified] {
        let selection = Selection::new(context(), Observation::Unknown { reason }).unwrap();
        let record = Record::new(selection);
        assert_eq!(
            serde_json::from_slice::<Record>(&record.encode().unwrap()).unwrap(),
            record
        );
        assert!(Selection::new(context(), Observation::Unmet { reason }).is_err());
    }
    for observation in [
        json!({"outcome":"unknown","reason":"missing"}),
        json!({"outcome":"unmet","reason":"unavailable"}),
        json!({"outcome":"unknown","reason":"timeout","actual":null}),
        json!({"outcome":"matched","actual":super::address("preview.a")}),
    ] {
        let mut value = serde_json::to_value(matched()).unwrap();
        value["entry"]["value"]["observation"] = observation;
        assert!(serde_json::from_value::<Record>(value).is_err());
    }
}

#[test]
fn retrieval() {
    let record = matched();
    let held = reference(&record);
    let bytes = record.encode().unwrap();
    let other = Reference::new(Name::read("foreign").unwrap(), held.digest().clone()).unwrap();
    assert!(held.verify(&other, &bytes).is_err());
    assert!(held.verify(&held, b"missing").is_err());
    let mut changed = serde_json::to_value(&record).unwrap();
    changed["entry"]["value"]["context"]["revision"] = json!(2);
    let changed = serde_json::to_vec(&changed).unwrap();
    assert!(held.verify(&held, &changed).is_err());
    let pretty = serde_json::to_vec_pretty(&record).unwrap();
    let prettyref = Reference::new(
        Name::read("wharf").unwrap(),
        Digest::read(&plumb::depot::sha(&pretty)).unwrap(),
    )
    .unwrap();
    assert!(prettyref.verify(&prettyref, &pretty).is_err());
    let mut changed = serde_json::to_value(&record).unwrap();
    changed["schema"] = json!(1);
    let changed = serde_json::to_vec(&changed).unwrap();
    let changedref = Reference::new(
        Name::read("wharf").unwrap(),
        Digest::read(&plumb::depot::sha(&changed)).unwrap(),
    )
    .unwrap();
    assert!(changedref.verify(&changedref, &changed).is_err());
}

#[test]
fn projection() {
    for path in [
        "",
        "/entry",
        "/entry/value",
        "/entry/value/context",
        "/entry/value/context/requested",
        "/entry/value/observation",
        "/entry/value/observation/artifact",
        "/entry/value/observation/artifact/source",
    ] {
        for field in ["token", "credentials", "payload", "url", "stamp"] {
            let mut value = serde_json::to_value(matched()).unwrap();
            value.pointer_mut(path).unwrap()[field] = json!("private");
            assert!(
                serde_json::from_value::<Record>(value).is_err(),
                "{path}/{field}"
            );
        }
    }
    let mut value = serde_json::to_value(matched()).unwrap();
    value["entry"]["value"]["observation"]["artifact"]["build"] = json!({
        "inputs":"c".repeat(64), "world":"d".repeat(64), "credentials":"private"
    });
    assert!(serde_json::from_value::<Record>(value).is_err());
}

#[test]
fn validation() {
    let original = serde_json::to_value(matched()).unwrap();
    for (path, value) in [
        ("/schema", json!(0)),
        ("/entry/value/context/revision", json!(0)),
        ("/entry/value/context/adaptor", json!("static.a")),
        ("/entry/value/context/request", json!("b".repeat(63))),
        ("/entry/value/policy", json!("fallback")),
    ] {
        let mut held = original.clone();
        *held.pointer_mut(path).unwrap() = value;
        assert!(serde_json::from_value::<Record>(held).is_err(), "{path}");
    }
    assert!(
        Context::new(
            address("preview"),
            Digest::read(&"a".repeat(64)).unwrap(),
            0,
            Name::read("static").unwrap()
        )
        .is_err()
    );
    assert!(Reference::new(Name::read("wharf.a").unwrap(), matched().digest().unwrap()).is_err());
    for value in [
        json!({"authority":"wharf.a","digest":"a".repeat(64)}),
        json!({"authority":"wharf","digest":"A".repeat(64)}),
        json!({"authority":"wharf","digest":"a".repeat(64),"url":"https://example.com/latest"}),
    ] {
        assert!(serde_json::from_value::<Reference>(value).is_err());
    }
    let repeated = String::from_utf8(matched().encode().unwrap())
        .unwrap()
        .replacen("\"schema\":2", "\"schema\":2,\"schema\":2", 1);
    assert!(serde_json::from_str::<Record>(&repeated).is_err());
    let reference = format!(
        r#"{{"authority":"wharf","authority":"other","digest":"{}"}}"#,
        "a".repeat(64)
    );
    assert!(serde_json::from_str::<Reference>(&reference).is_err());
    assert!(serde_json::from_value::<Record>(Value::Null).is_err());
}
