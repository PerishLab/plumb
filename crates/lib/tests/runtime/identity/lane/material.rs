use super::operation::{applied, bound, context, present};
use plumb::lane::operation::{Intent, Material, Operation, Outcome, Request, State, Writer};
use plumb::lane::{Action, Artifact, Build, Digest};

#[test]
fn available() {
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
    ] {
        let action = intent.action();
        let request = Request::new(context(), intent, Some(State::Absent {})).unwrap();
        let operation = Operation::new(
            request.clone(),
            bound(&request, action, State::Absent {}),
            applied(),
        )
        .unwrap();
        assert_eq!(operation.request().intent().action(), action);
        assert!(super::artifact::sample().build().is_none());
    }
}

#[test]
fn building() {
    let sample = super::artifact::sample();
    let build = Build::new(
        Digest::read(&"c".repeat(64)).unwrap(),
        Digest::read(&"d".repeat(64)).unwrap(),
    );
    let intent = Intent::Build {
        source: sample.source().clone(),
        build: build.clone(),
    };
    let request = Request::new(context(), intent, None).unwrap();
    let built = Artifact::new(
        sample.digest().clone(),
        sample.source().clone(),
        Some(build),
    );
    let outcome = Outcome::Applied {
        material: Material::Present {
            artifact: Box::new(built),
        },
        writer: Writer::Unknown,
    };
    assert!(
        Operation::new(
            request.clone(),
            bound(&request, Action::Build, State::Unknown {}),
            outcome
        )
        .is_ok()
    );
    assert!(
        Operation::new(
            request.clone(),
            bound(&request, Action::Build, State::Unknown {}),
            applied()
        )
        .is_err()
    );
    assert!(Request::new(context(), request.intent().clone(), Some(State::Absent {})).is_err());
    let inspect = Request::new(context(), Intent::Inspect {}, None).unwrap();
    assert!(
        Operation::new(
            inspect.clone(),
            bound(&inspect, Action::Inspect, State::Unknown {}),
            applied()
        )
        .is_err()
    );
    assert!(
        Operation::new(
            inspect.clone(),
            bound(&inspect, Action::Inspect, present()),
            applied()
        )
        .is_ok()
    );
}
