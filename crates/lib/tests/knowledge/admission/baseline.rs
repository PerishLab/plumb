use crate::collection::fixture as collection;

use super::{
    fixture::{COMMENT, Fixture},
    producer, runtime,
};
use plumb::semantics::admission::{Inspector, Record, Reference, World};
use plumb::windtun::qualification::State;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const JOBS: &str =
    "https://api.github.com/repos/PerishLab/wharf/actions/runs/32/attempts/1/jobs?per_page=100";
const DISTRIBUTION: &str =
    "https://releases.plumb.perish.uk/v1/releases/stable/v0.138.0/distribution.json";

fn object(fixture: &mut Fixture, value: &Value) -> Reference {
    let mut value = value.clone();
    value.sort_all_objects();
    let bytes = serde_json::to_vec(&value).unwrap();
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let reference = Reference {
        url: format!("https://releases.plumb.perish.uk/v1/semantics/objects/{digest}.json"),
        sha256: digest,
        size: bytes.len() as u64,
    };
    fixture.values.insert(reference.url.clone(), bytes);
    reference
}

fn issue(fixture: &mut Fixture, value: &Value) -> Reference {
    let reference = object(fixture, value);
    Record::parse(&fixture.values[&reference.url]).unwrap();
    fixture.edit(JOBS, |held| {
        held["jobs"][0]["steps"][0]["name"] =
            format!("[issue] semantics {}", reference.sha256).into()
    });
    reference
}

pub(super) fn setup() -> (Fixture, Reference) {
    let snapshot = collection::Fixture::new().inspect().snapshot.unwrap();
    let mut materials = collection::Fixture::new();
    materials.edit(collection::BASE, |held| {
        held["default_branch"] = "main".into()
    });
    let pointer: Value = serde_json::from_slice(&materials.values[collection::POINTER]).unwrap();
    let (mut fixture, _) = producer::setup();
    fixture.values.extend(materials.values);
    let source = json!({"repository":"PerishLab/plumb","marker":"v0.138.0","commit":"a".repeat(40),"tree":"b".repeat(40)});
    let guard = runtime::Fixture::runtime();
    let controls = json!({"plumb":"plumb v0.138.0","ectropy":"ectropy v0.51.0","authority":{"producer":format!("v0.138.0@{}", "c".repeat(40)),"depot":"d".repeat(64)},"stable":{"plumb":{"releaseVersion":"v0.138.0","commit":"c".repeat(40)},"ectropy":{"releaseVersion":"v0.51.0","commit":"e".repeat(40)}}});
    let world = json!({"schema":World::SCHEMA,"source":source,"head":guard.commit,"tree":"b".repeat(40),"packages":[],"guard":guard,
        "controls":controls,
        "domain":{"node.version":"24.18.0","pnpm.version":"11.28.5","rust.version":"1.96.1"}});
    World::parse(&serde_json::to_vec(&world).unwrap()).unwrap();
    let reference = object(&mut fixture, &world);
    let definition = json!({"schema":"plumb.release.adoption/v1","product":"plumb","marker":"v0.138.0","snapshot":snapshot.digest().unwrap(),"world":reference.sha256,"basis":"Synthetic exact retrospective materials; no original execution or qualification claim"});
    let body = format!("<!-- plumb.release.adoption/v1\n{definition}\n-->");
    fixture.edit(COMMENT, |held| held["body"] = body.clone().into());
    let review = json!({"url":super::fixture::LOCATION,"sha256":format!("{:x}", Sha256::digest(body.as_bytes())),"size":body.len()});
    let mut combination = world.clone();
    combination.as_object_mut().unwrap().remove("guard");
    let original = json!({"run":"10","attempt":"1","wharf":"a".repeat(40)});
    let plan = json!({"schema":"wharf.plan.reference/v1","repository":"PerishLab/plumb","marker":"v0.138.0","commit":"a".repeat(40),"tree":"b".repeat(40),"run":"10","attempt":"1","wharf":"a".repeat(40),"sha256":"9".repeat(64)});
    fixture.put(DISTRIBUTION, json!({"schema":1,"repository":"PerishLab/plumb","marker":"v0.138.0","commit":"a".repeat(40),"tree":"b".repeat(40),"media":{},"state":"complete","attempt":original,"completed":original,"resolution":{"binaries":combination},"trial":null,"plan":plan}));
    fixture.put(
        &format!("{}/git/ref/tags/v0.138.0", collection::BASE),
        json!({"ref":"refs/tags/v0.138.0","object":{"type":"tag","sha":"d".repeat(40)}}),
    );
    fixture.put(&format!("{}/git/tags/{}", collection::BASE, "d".repeat(40)), json!({"sha":"d".repeat(40),"tag":"v0.138.0","object":{"type":"commit","sha":"a".repeat(40)}}));
    let record = json!({"schema":"wharf.release.semantics.adoption/v1","product":"plumb","release":{"marker":"v0.138.0","snapshot":snapshot},"scenes":pointer,"world":reference,"review":review,"native":{"run":32,"attempt":1,"commit":"a".repeat(40),"job":77}});
    let reference = issue(&mut fixture, &record);
    (fixture, reference)
}

#[test]
fn adopted() {
    let (fixture, reference) = setup();
    let held = Inspector::new(fixture)
        .baseline(&reference, "plumb")
        .unwrap();
    assert_eq!(held.materials.world.source.marker, "v0.138.0");
    assert_eq!(
        held.materials.collection.snapshot.unwrap().coverage.len(),
        2
    );
    assert!(held.origin.authentication);
    assert!(!held.origin.publication);
    assert_eq!(held.materials.review.judgment.marker, "v0.138.0");
}

#[test]
fn distribution() {
    for (field, value) in [
        ("state", json!("pending")),
        ("commit", json!("c".repeat(40))),
        ("resolution", json!({})),
        ("resolution", json!({"binaries":{}})),
        ("plan", Value::Null),
    ] {
        let (mut fixture, reference) = setup();
        fixture.edit(DISTRIBUTION, |held| held[field] = value);
        assert!(
            Inspector::new(fixture)
                .baseline(&reference, "plumb")
                .is_err(),
            "{field}"
        );
    }
}

#[test]
fn historical() {
    let (mut fixture, reference) = setup();
    fixture.edit(DISTRIBUTION, |held| {
        held.as_object_mut().unwrap().remove("plan");
    });
    let fault = Inspector::new(fixture)
        .baseline(&reference, "plumb")
        .unwrap_err();
    assert_eq!(fault.state, State::Unknown);
    let (mut fixture, reference) = setup();
    fixture.values.remove(&format!(
        "{}/git/trees/{}?recursive=1",
        collection::BASE,
        "b".repeat(40)
    ));
    assert_eq!(
        Inspector::new(fixture)
            .baseline(&reference, "plumb")
            .unwrap_err()
            .state,
        State::Unknown
    );
}

#[test]
fn material() {
    let (mut fixture, reference) = setup();
    let mut record: Value = serde_json::from_slice(&fixture.values[&reference.url]).unwrap();
    record["release"]["snapshot"]["materials"][0]["sha256"] = "0".repeat(64).into();
    let reference = issue(&mut fixture, &record);
    assert!(
        Inspector::new(fixture)
            .baseline(&reference, "plumb")
            .is_err()
    );
}

#[test]
fn review() {
    let (mut fixture, reference) = setup();
    fixture.edit(COMMENT, |held| {
        held["body"] = format!("{}\nchanged", held["body"].as_str().unwrap()).into()
    });
    assert!(
        Inspector::new(fixture)
            .baseline(&reference, "plumb")
            .is_err()
    );
    let (mut fixture, reference) = setup();
    fixture.edit(super::fixture::PERMISSION, |held| {
        held["permission"] = "read".into()
    });
    assert!(
        Inspector::new(fixture)
            .baseline(&reference, "plumb")
            .is_err()
    );
}

#[test]
fn marker() {
    let (mut fixture, reference) = setup();
    fixture.put(
        &format!("{}/git/commits/{}", collection::BASE, "c".repeat(40)),
        json!({"sha":"c".repeat(40),"tree":{"sha":"b".repeat(40)}}),
    );
    fixture.edit(
        &format!("{}/git/tags/{}", collection::BASE, "d".repeat(40)),
        |held| held["object"]["sha"] = "c".repeat(40).into(),
    );
    let fault = Inspector::new(fixture)
        .baseline(&reference, "plumb")
        .unwrap_err();
    assert_eq!(fault.state, State::Refused);
    assert!(fault.reason.contains("actual annotated marker"));
}

#[test]
fn purpose() {
    let (fixture, reference) = producer::setup();
    assert!(
        Inspector::new(fixture)
            .baseline(&reference, "plumb")
            .is_err()
    );
}

#[test]
fn case() {
    let (mut fixture, reference) = setup();
    let mut record: Value = serde_json::from_slice(&fixture.values[&reference.url]).unwrap();
    let snapshot = &mut record["release"]["snapshot"];
    let material = snapshot["materials"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|held| held["kind"] == "case")
        .unwrap();
    let prior = material["name"].clone();
    material["name"] = "foreign".into();
    for coverage in snapshot["coverage"].as_array_mut().unwrap() {
        if coverage["case"] == prior {
            coverage["case"] = "foreign".into();
        }
    }
    let reference = issue(&mut fixture, &record);
    let fault = Inspector::new(fixture)
        .baseline(&reference, "plumb")
        .unwrap_err();
    assert_eq!(fault.state, State::Refused);
    assert!(fault.reason.contains("exact source and formal Scene"));
}

#[test]
fn world() {
    let (mut fixture, reference) = setup();
    let mut record: Value = serde_json::from_slice(&fixture.values[&reference.url]).unwrap();
    let url = record["world"]["url"].as_str().unwrap();
    let mut world = World::parse(&fixture.values[url]).unwrap();
    world.guard.guard.resolution = None;
    world.guard.guard.digest =
        "d27fbee012bb9959f4daa46e3dd06e67ff31c6116432b40d4afcb6016f44ac5f".into();
    world.guard.digest =
        plumb::guard::Runtime::seal(&world.guard.commit, &world.guard.guard).unwrap();
    world.guard.validate().unwrap();
    record["world"] =
        serde_json::to_value(object(&mut fixture, &serde_json::to_value(world).unwrap())).unwrap();
    let reference = issue(&mut fixture, &record);
    let fault = Inspector::new(fixture)
        .baseline(&reference, "plumb")
        .unwrap_err();
    assert_eq!(fault.state, State::Unknown);
    assert!(fault.reason.contains("no actual resolved Guard input"));
}

#[test]
fn effective() {
    let (mut fixture, reference) = setup();
    let mut record: Value = serde_json::from_slice(&fixture.values[&reference.url]).unwrap();
    let url = record["world"]["url"].as_str().unwrap();
    let mut world = World::parse(&fixture.values[url]).unwrap();
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
    record["world"] =
        serde_json::to_value(object(&mut fixture, &serde_json::to_value(world).unwrap())).unwrap();
    let reference = issue(&mut fixture, &record);
    let fault = Inspector::new(fixture)
        .baseline(&reference, "plumb")
        .unwrap_err();
    assert_eq!(fault.state, State::Unknown);
    assert!(fault.reason.contains("complete effective-source materials"));
}
