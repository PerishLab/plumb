use super::{
    baseline,
    fixture::{Fixture, LOCATION},
    producer,
    source::Inputs,
};
use plumb::seat::depot::v3::Pointer;
use plumb::semantics::admission::{
    Binding, Comparison, Inspector, Native, Preparation, Record, Reference,
};
use plumb::semantics::{Baseline, Decision, Release, Request};
use plumb::windtun::qualification::State;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const JOBS: &str =
    "https://api.github.com/repos/PerishLab/wharf/actions/runs/32/attempts/1/jobs?per_page=100";

pub(super) struct Source;

impl Source {
    fn frame() -> (Fixture, Reference, Pointer) {
        let (mut fixture, root) = baseline::setup();
        let Record::Adoption(adoption) = Record::parse(&fixture.values[&root.url]).unwrap() else {
            panic!("adoption fixture");
        };
        let (prepared, descriptor) = Self::setup();
        fixture.values.extend(prepared.values);
        let raw = serde_json::to_vec(&descriptor).unwrap();
        let sha256 = format!("{:x}", Sha256::digest(&raw));
        let reference = Reference {
            url: format!("https://releases.plumb.perish.uk/v1/semantics/objects/{sha256}.json"),
            sha256,
            size: raw.len() as u64,
        };
        fixture.values.insert(reference.url.clone(), raw);
        (fixture, reference, adoption.scenes)
    }

    fn retain(fixture: &mut Fixture, value: Value) -> Reference {
        let raw = serde_json::to_vec(&value).unwrap();
        let sha256 = format!("{:x}", Sha256::digest(&raw));
        let reference = Reference {
            url: format!("https://releases.plumb.perish.uk/v1/semantics/objects/{sha256}.json"),
            sha256,
            size: raw.len() as u64,
        };
        fixture.values.insert(reference.url.clone(), raw);
        reference
    }

    pub(super) fn setup() -> (Fixture, Preparation) {
        let (mut fixture, _) = producer::setup();
        let inputs = Inputs::new();
        let world = Self::retain(&mut fixture, serde_json::to_value(&inputs.world).unwrap());
        let material = Self::retain(&mut fixture, inputs.value());
        let preparation = Preparation {
            schema: Preparation::SCHEMA.into(),
            native: Native {
                run: 32,
                attempt: 1,
                commit: "a".repeat(40),
                job: 77,
            },
            world,
            material,
        };
        fixture.edit(JOBS, |value| {
            value["jobs"][0]["name"] = json!("[prepare] semantic world");
            value["jobs"][0]["steps"][0]["name"] = json!(format!(
                "[prepare] world {} {}",
                preparation.world.sha256, preparation.material.sha256
            ));
        });
        (fixture, preparation)
    }
}

#[test]
fn observed() {
    let (fixture, preparation) = Source::setup();
    let observed = Inspector::new(fixture)
        .prepared(&preparation, "plumb")
        .unwrap();
    assert!(observed.authentication);
    assert!(!observed.publication);
    assert!(observed.authority.requirements.scopes.is_empty());
    assert_eq!(observed.material.files.len(), 4);
    assert_ne!(observed.material.source.tree, observed.material.tree);
    assert_eq!(observed.material.sha256, preparation.material.sha256);
    let report = serde_json::to_value(&observed).unwrap();
    assert_eq!(report["material"]["files"], 4);
    assert_eq!(report["material"]["sha256"], preparation.material.sha256);
    assert!(serde_json::to_vec(&report).unwrap().len() < 32768);
}

#[test]
fn declarations() {
    let (_, preparation) = Source::setup();
    let raw = serde_json::to_vec(&preparation).unwrap();
    Preparation::parse(&raw).unwrap();
    for (field, value) in [
        ("authentication", json!(true)),
        ("schema", json!("foreign")),
        ("native", json!({})),
    ] {
        let mut changed = serde_json::to_value(&preparation).unwrap();
        changed[field] = value;
        assert!(Preparation::parse(&serde_json::to_vec(&changed).unwrap()).is_err());
    }
    let raw = String::from_utf8(raw)
        .unwrap()
        .replace("\"schema\":", "\"schema\":\"foreign\",\"schema\":");
    assert!(Preparation::parse(raw.as_bytes()).is_err());
}

#[test]
fn native() {
    for (field, value) in [
        ("name", json!("[issue] version semantics")),
        ("id", json!(78)),
        ("run_attempt", json!(2)),
        ("head_branch", json!("feature")),
        ("status", json!("in_progress")),
        ("conclusion", json!("failure")),
        ("steps", json!([])),
    ] {
        let (mut fixture, preparation) = Source::setup();
        fixture.edit(JOBS, |held| held["jobs"][0][field] = value);
        assert!(
            Inspector::new(fixture)
                .prepared(&preparation, "plumb")
                .is_err(),
            "{field}"
        );
    }
    for step in [
        "[prepare] world forged forged".to_string(),
        "[prepare] actual world".to_string(),
    ] {
        let (mut fixture, preparation) = Source::setup();
        fixture.edit(JOBS, |held| {
            held["jobs"][0]["steps"][0]["name"] = json!(step)
        });
        assert!(
            Inspector::new(fixture)
                .prepared(&preparation, "plumb")
                .is_err()
        );
    }
    let (mut fixture, preparation) = Source::setup();
    fixture.edit(JOBS, |held| {
        let repeated = held["jobs"][0]["steps"][0].clone();
        held["jobs"][0]["steps"]
            .as_array_mut()
            .unwrap()
            .push(repeated);
    });
    assert!(
        Inspector::new(fixture)
            .prepared(&preparation, "plumb")
            .is_err()
    );
}

#[test]
fn bytes() {
    let (mut fixture, mut preparation) = Source::setup();
    let mut value: Value =
        serde_json::from_slice(&fixture.values[&preparation.material.url]).unwrap();
    value["head"] = json!("0".repeat(40));
    preparation.material = Source::retain(&mut fixture, value);
    assert!(
        Inspector::new(fixture)
            .prepared(&preparation, "plumb")
            .is_err()
    );
    let (mut fixture, preparation) = Source::setup();
    fixture
        .values
        .insert(preparation.world.url.clone(), b"{}".to_vec());
    assert!(
        Inspector::new(fixture)
            .prepared(&preparation, "plumb")
            .is_err()
    );
    let (mut fixture, preparation) = Source::setup();
    fixture.edit(JOBS, |held| {
        held["jobs"][0]["steps"][0]["name"] = json!(format!(
            "[prepare] world {} {}",
            preparation.world.sha256,
            "0".repeat(64)
        ))
    });
    assert!(
        Inspector::new(fixture)
            .prepared(&preparation, "plumb")
            .is_err()
    );
}

#[test]
fn authority() {
    let (fixture, preparation) = Source::setup();
    assert!(
        Inspector::new(fixture)
            .prepared(&preparation, "concord")
            .is_err()
    );
    let (mut fixture, preparation) = Source::setup();
    fixture.moved = true;
    assert!(
        Inspector::new(fixture)
            .prepared(&preparation, "plumb")
            .is_err()
    );
    let (mut fixture, preparation) = Source::setup();
    fixture.values.remove(JOBS);
    let fault = Inspector::new(fixture)
        .prepared(&preparation, "plumb")
        .unwrap_err();
    assert_eq!(fault.state, State::Unknown);
}

#[test]
fn frame() {
    let (fixture, reference, scenes) = Source::frame();
    let frame = Inspector::new(fixture)
        .frame(&reference, &scenes, "plumb")
        .unwrap();
    assert_ne!(frame.world().tree, frame.world().source.tree);
    assert!(Binding::new(frame.snapshot(), frame.world()).is_err());
    assert!(frame.snapshot().materials.iter().any(|item| item.name
        == "plumb/release/preparation.json"
        && item.sha256 == reference.sha256));
    let comparison = Comparison {
        schema: Comparison::SCHEMA.into(),
        product: "plumb".into(),
        before: frame.binding().unwrap(),
        after: frame.binding().unwrap(),
        anchor: frame.binding().unwrap(),
        decision: Decision::Same,
        basis: "Synthetic authenticated complete-source frame, no publication grant".into(),
    };
    let projection = comparison
        .frames(LOCATION, [&frame, &frame, &frame])
        .unwrap();
    let request = Request {
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
    };
    let verdict = request.assess().unwrap();
    assert_eq!(verdict.marker.as_deref(), Some("v0.138.1"));
    assert!(!verdict.authentication && !verdict.publication);
}

#[test]
fn collection() {
    let (mut fixture, reference, scenes) = Source::frame();
    let raw = fixture.values[&reference.url].clone();
    let descriptor: Value = serde_json::from_slice(&raw).unwrap();
    fixture
        .values
        .remove(descriptor["material"]["url"].as_str().unwrap());
    assert!(
        Inspector::new(fixture)
            .frame(&reference, &scenes, "plumb")
            .is_err()
    );
    let (fixture, reference, scenes) = Source::frame();
    assert!(
        Inspector::new(fixture)
            .frame(&reference, &scenes, "concord")
            .is_err()
    );
}
