use plumb::lane::declaration::{Authorities, Declaration};
use plumb::lane::evidence::{Reason, Record, Reference};
use plumb::lane::operation::{Assessment, Request, State};
use plumb::lane::{Action, Capabilities, Name};
use serde_json::{Value, json};

fn declaration() -> Declaration {
    let request = super::operation::request();
    Declaration::new(
        request.context().requested().clone(),
        request.context().adaptor().clone(),
        Capabilities::new(vec![Action::Deploy]).unwrap(),
        Authorities::new(Name::read("wharf").unwrap(), Name::read("public").unwrap()).unwrap(),
    )
    .unwrap()
}

fn assessment() -> Assessment {
    super::operation::assessment(Action::Deploy, State::Absent {})
}

#[test]
fn canonical() {
    let held = declaration();
    let value = serde_json::to_value(&held).unwrap();
    assert_eq!(
        serde_json::from_value::<Declaration>(value.clone()).unwrap(),
        held
    );
    assert_eq!(held.digest().unwrap(), declaration().digest().unwrap());
    assert_eq!(held.target().app(), "crest");
    assert_eq!(held.adaptor().value(), "static");
    assert_eq!(held.authorities().authorization().value(), "wharf");
    assert_eq!(held.authorities().publication().value(), "public");
    assert!(
        held.admit(&super::operation::request(), &assessment())
            .is_ok()
    );
    for field in ["stamp", "url", "package", "worker", "path", "token"] {
        assert!(value.get(field).is_none());
    }
}

#[test]
fn projection() {
    let original = serde_json::to_value(declaration()).unwrap();
    for (path, value) in [
        ("/adaptor", json!("static.key")),
        ("/authorities/authorization", json!("wharf.key")),
        ("/authorities/publication", json!("public.key")),
        ("/target/app", json!("crest.review")),
        ("/capabilities", json!(["repair"])),
    ] {
        let mut held = original.clone();
        *held.pointer_mut(path).unwrap() = value;
        assert!(
            serde_json::from_value::<Declaration>(held).is_err(),
            "{path}"
        );
    }
    for path in ["", "/authorities", "/target"] {
        let mut held = original.clone();
        held.pointer_mut(path).unwrap()["token"] = json!("secret");
        assert!(serde_json::from_value::<Declaration>(held).is_err());
    }
    for field in ["target", "adaptor", "capabilities", "authorities"] {
        let mut held = original.clone();
        held.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<Declaration>(held).is_err());
    }
}

#[test]
fn admission() {
    let original = serde_json::to_value(declaration()).unwrap();
    for (path, value, reason) in [
        ("/target/lane", json!("preview.b"), Reason::Conflict),
        ("/target/app", json!("design"), Reason::Conflict),
        (
            "/target/repository",
            json!("PerishLab/design"),
            Reason::Conflict,
        ),
        ("/adaptor", json!("binary"), Reason::Conflict),
        (
            "/capabilities",
            json!(["inspect", "deploy"]),
            Reason::Conflict,
        ),
        (
            "/authorities/authorization",
            json!("foreign"),
            Reason::Unverified,
        ),
    ] {
        let mut held = original.clone();
        *held.pointer_mut(path).unwrap() = value;
        let held: Declaration = serde_json::from_value(held).unwrap();
        assert_eq!(
            held.admit(&super::operation::request(), &assessment()),
            Err(reason)
        );
        assert_ne!(held.digest().unwrap(), declaration().digest().unwrap());
    }
    for (path, value, reason) in [
        ("/conditions/writer", json!("active"), Reason::Conflict),
        ("/conditions/writer", json!("unknown"), Reason::Unavailable),
        (
            "/conditions/authorization",
            json!({"state":"unknown"}),
            Reason::Unverified,
        ),
        (
            "/conditions/authorization/state",
            json!("denied"),
            Reason::Denied,
        ),
        ("/request", json!("d".repeat(64)), Reason::Conflict),
    ] {
        let mut held = serde_json::to_value(assessment()).unwrap();
        *held.pointer_mut(path).unwrap() = value;
        let held: Assessment = serde_json::from_value(held).unwrap();
        assert_eq!(
            declaration().admit(&super::operation::request(), &held),
            Err(reason)
        );
    }
}

#[test]
fn completion() {
    let record = Record::execution(super::operation::operation());
    let bytes = record.encode().unwrap();
    for (authority, accepted) in [("public", true), ("wharf", false), ("foreign", false)] {
        let reference =
            Reference::new(Name::read(authority).unwrap(), record.digest().unwrap()).unwrap();
        let verified = reference.verify(&reference, &bytes).unwrap();
        let receipt = declaration().settle(&super::operation::request(), &verified);
        assert_eq!(receipt.is_ok(), accepted);
        let mut changed = serde_json::to_value(declaration()).unwrap();
        changed["capabilities"] = json!(["inspect", "deploy"]);
        let changed: Declaration = serde_json::from_value(changed).unwrap();
        assert!(
            changed
                .settle(&super::operation::request(), &verified)
                .is_err()
        );
        if let Ok(receipt) = receipt {
            assert_eq!(receipt.reference(), &reference);
        }
        let mut changed = serde_json::to_value(super::operation::request()).unwrap();
        changed["context"]["revision"] = json!(3);
        let changed: Request = serde_json::from_value(changed).unwrap();
        assert!(declaration().settle(&changed, &verified).is_err());
    }
}

#[test]
fn unsupported() {
    let mut held = serde_json::to_value(declaration()).unwrap();
    held["capabilities"] = json!(["inspect"]);
    let held: Declaration = serde_json::from_value(held).unwrap();
    let assessment = super::operation::assessment(Action::Inspect, State::Absent {});
    assert_eq!(
        held.admit(&super::operation::request(), &assessment),
        Err(Reason::Unsupported)
    );
    for name in ["wharf.key", "public.key"] {
        assert!(
            Authorities::new(Name::read(name).unwrap(), Name::read("public").unwrap()).is_err()
        );
        assert!(Authorities::new(Name::read("wharf").unwrap(), Name::read(name).unwrap()).is_err());
    }
    let duplicate = r#"{"authorization":"wharf","publication":"public","publication":"foreign"}"#;
    assert!(serde_json::from_str::<Authorities>(duplicate).is_err());
}

#[test]
fn profiles() {
    let original = serde_json::to_value(declaration()).unwrap();
    for (adaptor, lane, actions) in [
        ("static", "preview", json!(["inspect", "deploy", "dispose"])),
        (
            "binary",
            "lane.focus",
            json!(["inspect", "install", "uninstall"]),
        ),
        (
            "package",
            "lane.focus",
            json!(["inspect", "publish", "install"]),
        ),
    ] {
        let mut held = original.clone();
        held["adaptor"] = json!(adaptor);
        held["target"]["lane"] = json!(lane);
        held["capabilities"] = actions;
        let held: Declaration = serde_json::from_value(held).unwrap();
        assert_eq!(held.target().app(), "crest");
        let value: Value = serde_json::to_value(&held).unwrap();
        assert!(value.get("url").is_none());
        assert!(!held.capabilities().supports(Action::Build));
        if adaptor == "package" {
            assert!(!held.capabilities().supports(Action::Dispose));
        }
    }
}
