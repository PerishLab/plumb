use super::operation::{bound, context, present};
use plumb::lane::evidence::Reason;
use plumb::lane::operation::{Intent, Request, State};
use plumb::lane::{Action, Digest};
use serde_json::json;

#[test]
fn replay() {
    let original = super::operation::request();
    let assessment = bound(&original, Action::Deploy, State::Absent {});
    for (path, value) in [
        ("/context/revision", json!(3)),
        ("/context/request", json!("d".repeat(64))),
        ("/context/requested/lane", json!("preview.b")),
        ("/context/adaptor", json!("binary")),
        ("/intent/artifact/digest", json!("d".repeat(64))),
    ] {
        let mut held = serde_json::to_value(&original).unwrap();
        *held.pointer_mut(path).unwrap() = value;
        let held: Request = serde_json::from_value(held).unwrap();
        assert_eq!(held.admit(&assessment), Err(Reason::Conflict), "{path}");
    }
}

#[test]
fn preconditions() {
    for intent in [
        Intent::Publish {
            artifact: Box::new(super::artifact::sample()),
        },
        Intent::Install {
            artifact: Box::new(super::artifact::sample()),
        },
        Intent::Deploy {
            artifact: Box::new(super::artifact::sample()),
        },
        Intent::Uninstall {},
        Intent::Dispose {},
    ] {
        assert!(Request::new(context(), intent.clone(), None).is_err());
        assert!(Request::new(context(), intent.clone(), Some(State::Unknown {})).is_err());
        let expected = match intent {
            Intent::Uninstall {} | Intent::Dispose {} => present(),
            _ => State::Absent {},
        };
        assert!(Request::new(context(), intent, Some(expected)).is_ok());
    }
    assert!(
        Request::new(
            context(),
            Intent::Publish {
                artifact: Box::new(super::artifact::sample())
            },
            Some(present())
        )
        .is_err()
    );
    for intent in [Intent::Uninstall {}, Intent::Dispose {}] {
        assert!(Request::new(context(), intent, Some(State::Absent {})).is_err());
    }
    let held = Request::new(
        context(),
        Intent::Deploy {
            artifact: Box::new(super::artifact::sample()),
        },
        Some(present()),
    )
    .unwrap();
    assert!(held.admit(&bound(&held, Action::Deploy, present())).is_ok());
    for (revision, digest) in [(2, "a"), (1, "d")] {
        let observed = State::Present {
            revision,
            digest: Digest::read(&digest.repeat(64)).unwrap(),
        };
        assert_eq!(
            held.admit(&bound(&held, Action::Deploy, observed)),
            Err(Reason::Conflict)
        );
    }
}
