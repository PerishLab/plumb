use plumb::lane::evidence::{Context, Reason, Reference};
use plumb::lane::operation::{
    Assessment, Authorization, Conditions, Intent, Material, Operation, Outcome, Request, State,
    Writer,
};
use plumb::lane::{Action, Address, Artifact, Capabilities, Digest, Name};
use serde_json::json;

pub(super) fn context() -> Context {
    Context::new(
        Address::new(
            "PerishLab/crest".into(),
            "crest".into(),
            Name::read("preview.a").unwrap(),
        )
        .unwrap(),
        Digest::read(&"b".repeat(64)).unwrap(),
        2,
        Name::read("static").unwrap(),
    )
    .unwrap()
}

pub(super) fn request() -> Request {
    Request::new(
        context(),
        Intent::Deploy {
            artifact: Box::new(super::artifact::sample()),
        },
        Some(State::Absent {}),
    )
    .unwrap()
}

pub(super) fn assessment(action: Action, state: State) -> Assessment {
    bound(&request(), action, state)
}

pub(super) fn bound(request: &Request, action: Action, state: State) -> Assessment {
    let evidence = Reference::new(
        Name::read("wharf").unwrap(),
        Digest::read(&"c".repeat(64)).unwrap(),
    )
    .unwrap();
    Assessment::new(
        request.digest().unwrap(),
        Capabilities::new(vec![action]).unwrap(),
        Conditions::new(Authorization::Granted { evidence }, Writer::Idle, state).unwrap(),
    )
}

pub(super) fn applied() -> Outcome {
    Outcome::Applied {
        material: Material::Present {
            artifact: Box::new(super::artifact::sample()),
        },
        writer: Writer::Idle,
    }
}

pub(super) fn operation() -> Operation {
    Operation::new(
        request(),
        assessment(Action::Deploy, State::Absent {}),
        applied(),
    )
    .unwrap()
}

pub(super) fn present() -> State {
    State::Present {
        revision: 1,
        digest: super::artifact::sample().digest().clone(),
    }
}

#[test]
fn admission() {
    assert!(
        request()
            .admit(&assessment(Action::Deploy, State::Absent {}))
            .is_ok()
    );
    assert_eq!(
        request().admit(&assessment(Action::Install, State::Absent {})),
        Err(Reason::Unsupported)
    );
    assert_eq!(
        request().admit(&assessment(Action::Deploy, State::Unknown {})),
        Err(Reason::Unverified)
    );
    assert_eq!(
        request().admit(&assessment(Action::Deploy, present())),
        Err(Reason::Conflict)
    );
    for (field, value, reason) in [
        ("writer", json!("active"), Reason::Conflict),
        ("writer", json!("unknown"), Reason::Unavailable),
        (
            "authorization",
            json!({"state":"unknown"}),
            Reason::Unverified,
        ),
    ] {
        let mut held = serde_json::to_value(assessment(Action::Deploy, State::Absent {})).unwrap();
        held["conditions"][field] = value;
        let held: Assessment = serde_json::from_value(held).unwrap();
        assert_eq!(request().admit(&held), Err(reason));
        assert!(Operation::new(request(), held.clone(), applied()).is_err());
        let outcome = if reason == Reason::Conflict {
            Outcome::Refused { reason }
        } else {
            Outcome::Unknown { reason }
        };
        assert!(Operation::new(request(), held, outcome).is_ok());
    }
    let mut denied = serde_json::to_value(assessment(Action::Deploy, State::Absent {})).unwrap();
    denied["conditions"]["authorization"]["state"] = json!("denied");
    let denied: Assessment = serde_json::from_value(denied).unwrap();
    assert_eq!(request().admit(&denied), Err(Reason::Denied));
    assert!(
        Operation::new(
            request(),
            denied,
            Outcome::Unknown {
                reason: Reason::Unavailable
            }
        )
        .is_err()
    );
}

#[test]
fn material() {
    let operation = operation();
    assert_eq!(
        serde_json::from_value::<Operation>(serde_json::to_value(&operation).unwrap()).unwrap(),
        operation
    );
    assert_eq!(operation.assessment().writer(), Writer::Idle);
    for writer in [Writer::Unknown, Writer::Active] {
        let outcome = Outcome::Applied {
            material: Material::Present {
                artifact: Box::new(super::artifact::sample()),
            },
            writer,
        };
        assert!(
            Operation::new(
                request(),
                assessment(Action::Deploy, State::Absent {}),
                outcome
            )
            .is_err()
        );
    }
    let different = Artifact::new(
        Digest::read(&"d".repeat(64)).unwrap(),
        super::artifact::sample().source().clone(),
        None,
    );
    let outcome = Outcome::Applied {
        material: Material::Present {
            artifact: Box::new(different),
        },
        writer: Writer::Idle,
    };
    assert!(
        Operation::new(
            request(),
            assessment(Action::Deploy, State::Absent {}),
            outcome
        )
        .is_err()
    );
    for intent in [Intent::Uninstall {}, Intent::Dispose {}] {
        let action = intent.action();
        let request = Request::new(context(), intent, Some(present())).unwrap();
        assert!(
            Operation::new(
                request.clone(),
                bound(&request, action, present()),
                applied()
            )
            .is_err()
        );
        assert!(
            Operation::new(
                request.clone(),
                bound(&request, action, present()),
                Outcome::Applied {
                    material: Material::Absent {},
                    writer: Writer::Idle
                }
            )
            .is_ok()
        );
    }
}

#[test]
fn refusals() {
    let original = serde_json::to_value(operation()).unwrap();
    for (path, value) in [
        ("/request/expected", json!({"state":"unknown"})),
        ("/request/intent/action", json!("overwrite")),
        (
            "/assessment/conditions/observed",
            json!({"state":"present","revision":0,"digest":"a".repeat(64)}),
        ),
        ("/outcome/outcome", json!("complete")),
        ("/outcome/writer", json!("unknown")),
    ] {
        let mut held = original.clone();
        *held.pointer_mut(path).unwrap() = value;
        assert!(serde_json::from_value::<Operation>(held).is_err(), "{path}");
    }
    for path in [
        "",
        "/request",
        "/request/context",
        "/request/intent",
        "/assessment",
        "/assessment/conditions",
        "/assessment/conditions/authorization",
        "/outcome",
        "/outcome/material",
    ] {
        let mut held = original.clone();
        held.pointer_mut(path).unwrap()["credentials"] = json!("secret");
        assert!(serde_json::from_value::<Operation>(held).is_err());
    }
    for reason in [Reason::Unavailable, Reason::Unverified] {
        assert!(
            Operation::new(
                request(),
                assessment(Action::Deploy, State::Absent {}),
                Outcome::Refused { reason }
            )
            .is_err()
        );
    }
    assert!(
        Operation::new(
            request(),
            assessment(Action::Deploy, State::Absent {}),
            Outcome::Unknown {
                reason: Reason::Missing
            }
        )
        .is_err()
    );
}
