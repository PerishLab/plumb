mod capture;
mod changes;
mod family;
mod observation;
mod reconciliation;
mod selection;
mod validation;

use super::{baseline, fixture::Fixture};
use plumb::semantics::admission::{Checkpoint, Inspector, Native, Record, Reference, Tag};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn retained(fixture: &mut Fixture, checkpoint: &Checkpoint) -> Reference {
    let raw = checkpoint.encode().unwrap();
    let sha256 = format!("{:x}", Sha256::digest(&raw));
    let reference = Reference {
        url: format!("https://releases.plumb.perish.uk/v1/semantics/objects/{sha256}.json"),
        sha256,
        size: raw.len() as u64,
    };
    fixture.values.insert(reference.url.clone(), raw);
    let original = "https://api.github.com/repos/PerishLab/wharf/actions/runs/32/attempts/1";
    let mut run: Value = serde_json::from_slice(&fixture.values[original]).unwrap();
    run["id"] = checkpoint.native.run.into();
    let route = format!(
        "https://api.github.com/repos/PerishLab/wharf/actions/runs/{}/attempts/1",
        checkpoint.native.run
    );
    fixture.put(&route, run);
    fixture.put(&format!("{route}/jobs?per_page=100"), json!({"total_count":1,"jobs":[{
        "id":checkpoint.native.job,"run_id":checkpoint.native.run,"run_attempt":1,
        "head_sha":checkpoint.native.commit,"head_branch":"main","name":"[retain] semantic history",
        "status":"completed","conclusion":"success","steps":[{"name":format!("[retain] history {}",reference.sha256),"status":"completed","conclusion":"success"}]
    }]}));
    reference
}

fn setup() -> (Fixture, Checkpoint, Reference) {
    let (mut fixture, root) = baseline::setup();
    let Record::Adoption(adopted) = Record::parse(&fixture.values[&root.url]).unwrap() else {
        panic!("adoption fixture")
    };
    let repository: Value =
        serde_json::from_slice(&fixture.values["https://api.github.com/repos/PerishLab/plumb"])
            .unwrap();
    let checkpoint = Checkpoint {
        schema: Checkpoint::SCHEMA.into(),
        product: "plumb".into(),
        repository: json!({"id":repository["id"],"node_id":repository["node_id"],"full_name":repository["full_name"],"default_branch":repository["default_branch"],"owner":repository["owner"]}),
        root,
        previous: None,
        sequence: 1,
        markers: vec![
            Tag {
                marker: adopted.release.marker,
                commit: adopted.release.snapshot.source.commit,
            },
            Tag {
                marker: "v0.139.0-rc.1".into(),
                commit: "d".repeat(40),
            },
        ],
        native: Native {
            run: 40,
            attempt: 1,
            commit: "a".repeat(40),
            job: 88,
        },
    };
    let reference = retained(&mut fixture, &checkpoint);
    (fixture, checkpoint, reference)
}

fn append(checkpoint: &Checkpoint, reference: &Reference) -> Checkpoint {
    let mut next = checkpoint.clone();
    next.previous = Some(reference.clone());
    next.sequence += 1;
    next.native.run += 1;
    next.native.job += 1;
    next
}

#[test]
fn observed() {
    let (mut fixture, checkpoint, reference) = setup();
    let mut next = append(&checkpoint, &reference);
    next.markers.push(Tag {
        marker: "v0.139.0-rc.2".into(),
        commit: "e".repeat(40),
    });
    let head = retained(&mut fixture, &next);
    let history = Inspector::new(fixture).history(&head, "plumb").unwrap();
    assert_eq!(history.reference(), &head);
    assert_eq!(history.checkpoints().len(), 2);
    assert_eq!(history.checkpoints()[0].markers.len(), 3);
    assert_eq!(history.checkpoints()[1].root, checkpoint.root);
    assert_eq!(history.root().origin.reference, checkpoint.root);
    let value = serde_json::to_value(history).unwrap();
    assert_eq!(value["authentication"], true);
    assert_eq!(value["prior"], "unknown");
    assert_eq!(value["publication"], false);
}

#[test]
fn declarations() {
    let (_, checkpoint, _) = setup();
    let raw = checkpoint.encode().unwrap();
    Checkpoint::parse(&raw).unwrap();
    for (field, value) in [
        ("schema", json!("foreign")),
        ("sequence", json!(0)),
        ("previous", json!({})),
        ("markers", json!([])),
        ("complete", json!(true)),
    ] {
        let mut changed = serde_json::to_value(&checkpoint).unwrap();
        changed[field] = value;
        changed.sort_all_objects();
        assert!(
            Checkpoint::parse(&serde_json::to_vec(&changed).unwrap()).is_err(),
            "{field}"
        );
    }
    let duplicate = String::from_utf8(raw.clone())
        .unwrap()
        .replace("\"schema\":", "\"schema\":\"foreign\",\"schema\":");
    assert!(Checkpoint::parse(duplicate.as_bytes()).is_err());
    assert!(Checkpoint::parse(&[raw, b"\n".to_vec()].concat()).is_err());
}

#[test]
fn reservations() {
    let (_, checkpoint, reference) = setup();
    let mut next = append(&checkpoint, &reference);
    next.follows(&reference, &checkpoint).unwrap();
    next.markers.pop();
    assert!(next.follows(&reference, &checkpoint).is_err());
    let mut next = append(&checkpoint, &reference);
    next.markers[1].commit = "f".repeat(40);
    assert!(next.follows(&reference, &checkpoint).is_err());
    let mut next = append(&checkpoint, &reference);
    next.markers.reverse();
    assert!(next.encode().is_err());
    let mut next = append(&checkpoint, &reference);
    next.markers.push(next.markers[1].clone());
    assert!(next.encode().is_err());
}
