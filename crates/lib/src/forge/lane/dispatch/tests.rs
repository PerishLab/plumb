use super::*;
use serde_json::{Value, json};

fn document() -> Value {
    json!({"schema":"wharf.lane.dispatch/v1", "target":{"repository":"PerishLab/crest","app":"crest","lane":"design"},
        "operation":"apply", "request":"a".repeat(64), "revision":2, "expected":{"state":"absent"},
        "source":{"commit":"b".repeat(40),"tree":"c".repeat(40),"declaration":"d".repeat(64)},
        "registration":"e".repeat(64), "caller":"LiberteCode", "workflow":format!("PerishLab/wharf/.github/workflows/lane.yml@{}", "f".repeat(40))})
}

fn read(value: Value) -> Result<Dispatch, String> {
    Dispatch::read(&serde_json::to_vec(&value).unwrap())
}

#[test]
fn boundary() {
    let good = read(document()).unwrap();
    assert_eq!(good.capability(), Action::Deploy);
    assert_eq!(Dispatch::read(&good.bytes().unwrap()).unwrap(), good);
    for (field, value) in [
        ("schema", json!("wharf.preview.request/v1")),
        ("revision", json!(0)),
        ("revision", json!(u64::MAX)),
        ("expected", json!({"state":"unknown"})),
        ("caller", json!("secret/value")),
        (
            "workflow",
            json!("PerishLab/wharf/.github/workflows/ship.yml@main"),
        ),
        ("source", Value::Null),
        ("request", json!("a".repeat(32))),
        ("provider_token", json!("private-provider-value")),
    ] {
        let mut changed = document();
        changed[field] = value;
        assert!(read(changed).is_err(), "{field}");
    }
    assert!(Dispatch::read(&vec![b' '; 65537]).is_err());
}

#[test]
fn precondition() {
    let mut value = document();
    value["operation"] = json!("dispose");
    value["source"] = Value::Null;
    assert!(read(value.clone()).is_err());
    value["expected"] = json!({"state":"present","revision":1,"digest":"1".repeat(64)});
    let held = read(value.clone()).unwrap();
    assert_eq!(held.capability(), Action::Dispose);
    let mut missing = value.clone();
    missing.as_object_mut().unwrap().remove("source");
    assert!(read(missing).is_err());
    value["expected"]["revision"] = json!(0);
    assert!(read(value.clone()).is_err());
    value["expected"]["revision"] = json!(2);
    assert!(read(value).is_err());
}

#[test]
fn context() {
    let held = read(document()).unwrap();
    let artifact = super::super::Artifact::new(
        Digest::read(&"1".repeat(64)).unwrap(),
        super::super::Source::new("PerishLab/crest".into(), "b".repeat(40), "c".repeat(40))
            .unwrap(),
        Some(super::super::Build::new(
            Digest::read(&"2".repeat(64)).unwrap(),
            Digest::read(&"3".repeat(64)).unwrap(),
        )),
    );
    let context = super::super::evidence::Context::new(
        held.target().clone(),
        held.request().clone(),
        2,
        super::super::Atom::read("static").unwrap(),
    )
    .unwrap();
    let request = Request::new(
        context,
        Intent::Deploy {
            artifact: Box::new(artifact),
        },
        Some(State::Absent {}),
    )
    .unwrap();
    held.matches(&request).unwrap();
    for key in [
        "request",
        "revision",
        "lane",
        "commit",
        "tree",
        "expected",
        "operation",
    ] {
        let changed = changed(key);
        assert!(read(changed).unwrap().matches(&request).is_err(), "{key}");
    }
}

fn changed(key: &str) -> Value {
    let mut changed = document();
    match key {
        "request" => changed[key] = json!("0".repeat(64)),
        "revision" => changed[key] = json!(3),
        "lane" => changed["target"][key] = json!("review"),
        "commit" | "tree" => changed["source"][key] = json!("0".repeat(40)),
        "expected" => {
            changed[key] = json!({"state":"present","revision":1,"digest":"0".repeat(64)})
        }
        _ => {
            changed[key] = json!("dispose");
            changed["source"] = Value::Null;
            changed["expected"] = json!({"state":"present","revision":1,"digest":"0".repeat(64)});
        }
    }
    changed
}
