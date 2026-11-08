use super::{
    baseline,
    fixture::{COMMENT, Fixture, LOCATION, PERMISSION},
};
use plumb::semantics::admission::{Binding, Comparison, Inspector, Record, World};
use plumb::semantics::{Baseline, Decision, Kind, Material, Release, Request, Snapshot, State};
use serde_json::json;

struct Inputs {
    snapshot: Snapshot,
    before: World,
    after: World,
}

impl Inputs {
    fn new() -> Self {
        let (fixture, reference) = baseline::setup();
        let Record::Adoption(adopted) = Record::parse(&fixture.values[&reference.url]).unwrap()
        else {
            panic!("adoption fixture");
        };
        let before = World::parse(&fixture.values[&adopted.world.url]).unwrap();
        let mut after = before.clone();
        after.domain.insert("pnpm.version".into(), "11.28.6".into());
        Self {
            snapshot: adopted.release.snapshot,
            before,
            after,
        }
    }

    fn judgment(&self, decision: Decision) -> Comparison {
        Comparison {
            schema: Comparison::SCHEMA.into(),
            product: "plumb".into(),
            before: Binding::new(&self.snapshot, &self.before).unwrap(),
            after: Binding::new(&self.snapshot, &self.after).unwrap(),
            anchor: Binding::new(&self.snapshot, &self.before).unwrap(),
            decision,
            basis: "Synthetic exact world-only comparison, not a native publication".into(),
        }
    }

    fn request(&self, judgment: &Comparison) -> Request {
        let projection = judgment
            .project(
                LOCATION,
                [
                    (&self.snapshot, &self.before),
                    (&self.snapshot, &self.after),
                    (&self.snapshot, &self.before),
                ],
            )
            .unwrap();
        Request {
            schema: Request::SCHEMA.into(),
            baseline: Baseline {
                stable: Release {
                    marker: "v0.138.0".into(),
                    snapshot: projection.before,
                },
                anchor: Release {
                    marker: "v0.138.0".into(),
                    snapshot: projection.anchor,
                },
                review: None,
            },
            candidate: projection.after,
            review: Some(projection.review),
            reservations: vec!["v0.138.0".into()],
            major: None,
        }
    }

    fn body(&self) -> String {
        let value = serde_json::to_string(&self.judgment(Decision::Same)).unwrap();
        format!("<!-- plumb.release.review/v2\n{value}\n-->")
    }
}

#[test]
fn projection() {
    let inputs = Inputs::new();
    let original = inputs.snapshot.clone();
    for (decision, state, marker) in [
        (Decision::Same, State::Same, "v0.138.1"),
        (Decision::Changed, State::Changed, "v0.139.0"),
    ] {
        let request = inputs.request(&inputs.judgment(decision));
        assert_ne!(request.baseline.stable.snapshot, request.candidate);
        let assessment = request.assess().unwrap();
        assert_eq!(assessment.state, state);
        assert_eq!(assessment.marker.as_deref(), Some(marker));
        assert!(!assessment.authentication && !assessment.publication);
    }
    let mut request = inputs.request(&inputs.judgment(Decision::Same));
    request.review = None;
    assert_eq!(request.assess().unwrap().state, State::Unknown);
    assert_eq!(inputs.snapshot, original);
}

#[test]
fn binding() {
    let inputs = Inputs::new();
    for (role, world) in [
        (0, true),
        (1, true),
        (2, true),
        (0, false),
        (1, false),
        (2, false),
    ] {
        let mut judgment = inputs.judgment(Decision::Same);
        let binding = match role {
            0 => &mut judgment.before,
            1 => &mut judgment.after,
            _ => &mut judgment.anchor,
        };
        if world {
            binding.world = "0".repeat(64);
        } else {
            binding.snapshot = "0".repeat(64);
        }
        assert!(
            judgment
                .project(
                    LOCATION,
                    [
                        (&inputs.snapshot, &inputs.before),
                        (&inputs.snapshot, &inputs.after),
                        (&inputs.snapshot, &inputs.before),
                    ],
                )
                .is_err()
        );
    }
    let mut moved = inputs.before.clone();
    moved.source.commit = "f".repeat(40);
    assert!(Binding::new(&inputs.snapshot, &moved).is_err());
    let mut repeated = inputs.snapshot.clone();
    repeated.materials.push(
        Material::new(
            Kind::Runtime,
            "plumb/release/world.json".into(),
            b"caller-supplied world",
            false,
        )
        .unwrap(),
    );
    let mut judgment = inputs.judgment(Decision::Same);
    judgment.before = Binding::new(&repeated, &inputs.before).unwrap();
    assert!(
        judgment
            .project(
                LOCATION,
                [
                    (&repeated, &inputs.before),
                    (&inputs.snapshot, &inputs.after),
                    (&inputs.snapshot, &inputs.before),
                ],
            )
            .is_err()
    );
}

#[test]
fn effective() {
    let inputs = Inputs::new();
    let mut world = inputs.before.clone();
    world.head = "a".repeat(40);
    world.tree = "c".repeat(40);
    world.guard.guard = world
        .guard
        .guard
        .resolved(plumb::packages::Resolution {
            context: "ci-latest".into(),
            tree: world.tree.clone(),
            packages: vec![],
        })
        .unwrap();
    world.guard.digest =
        plumb::guard::Runtime::seal(&world.guard.commit, &world.guard.guard).unwrap();
    world.validate().unwrap();
    let fault = Binding::new(&inputs.snapshot, &world).unwrap_err();
    assert_eq!(fault.state, plumb::windtun::qualification::State::Unknown);
    assert!(fault.reason.contains("complete effective-source materials"));
}

#[test]
fn declarations() {
    let inputs = Inputs::new();
    let body = inputs.body();
    assert!(Comparison::parse(&body).is_ok());
    for changed in [
        body.replace("review/v2", "review/v1"),
        body.clone() + &body,
        body.clone() + "\n<!-- plumb.release.review/v1\n{}\n-->",
        body.replace("\"decision\":\"same\"", "\"decision\":\"major\""),
        body.replacen("\"world\":", "\"extra\":true,\"world\":", 1),
        body.replacen("\"world\":", "\"world\":\"foreign\",\"world\":", 1),
    ] {
        assert!(Comparison::parse(&changed).is_err());
    }
    let mut judgment = inputs.judgment(Decision::Changed);
    judgment.after = judgment.before.clone();
    let value = serde_json::to_string(&judgment).unwrap();
    assert!(Comparison::parse(&format!("<!-- plumb.release.review/v2\n{value}\n-->")).is_err());
}

#[test]
fn author() {
    let inputs = Inputs::new();
    let mut fixture = Fixture::new();
    fixture.edit(COMMENT, |held| held["body"] = inputs.body().into());
    let observed = Inspector::new(fixture)
        .comparison(LOCATION, "plumb")
        .unwrap();
    assert_eq!(observed.author.id, 23);
    assert_eq!(
        observed.judgment.before,
        inputs.judgment(Decision::Same).before
    );
    assert!(
        Inspector::new(Fixture::new())
            .comparison(LOCATION, "plumb")
            .is_err()
    );
    let mut fixture = Fixture::new();
    fixture.edit(COMMENT, |held| held["body"] = inputs.body().into());
    fixture.edit(PERMISSION, |held| held["permission"] = json!("read"));
    assert!(
        Inspector::new(fixture)
            .comparison(LOCATION, "plumb")
            .is_err()
    );
}
