use plumb::semantics::{
    Baseline, Coverage, Decision, Kind, Material, Release, Request, Review, Snapshot, Source, State,
};

fn snapshot() -> Snapshot {
    Snapshot {
        product: "plumb".into(),
        source: Source {
            repository: "PerishLab/plumb".into(),
            commit: "a".repeat(40),
            tree: "b".repeat(40),
        },
        materials: vec![
            Material::new(Kind::Schema, "api.json".into(), b"{}", false).unwrap(),
            Material::new(Kind::Case, "writer".into(), b"assert original", true).unwrap(),
        ],
        coverage: vec![Coverage {
            case: "writer".into(),
            platform: "linux-x86_64".into(),
            backend: "none".into(),
        }],
    }
}

fn request() -> Request {
    let release = Release {
        marker: "v0.28.2".into(),
        snapshot: snapshot(),
    };
    Request {
        schema: Request::SCHEMA.into(),
        baseline: Baseline {
            stable: release.clone(),
            anchor: release,
            review: None,
        },
        candidate: snapshot(),
        review: None,
        reservations: vec![],
        major: None,
    }
}

fn review(held: &Request, decision: Decision) -> Review {
    Review {
        before: held.baseline.stable.snapshot.digest().unwrap(),
        after: held.candidate.digest().unwrap(),
        anchor: held.baseline.anchor.snapshot.digest().unwrap(),
        decision,
        reference: "https://github.com/PerishLab/.github/issues/176#issuecomment-1".into(),
    }
}

#[test]
fn unchanged() {
    let result = request().assess().unwrap();
    assert_eq!(result.state, State::Same);
    assert_eq!(result.marker.as_deref(), Some("v0.28.3"));
    assert_eq!(result.anchor, snapshot().digest().unwrap());
    assert_eq!(result.family, "v0.28.2");
    assert!(!result.authentication && !result.publication);
    assert_eq!(result.code(), 0);
}

#[test]
fn contradiction() {
    let mut held = request();
    held.review = Some(review(&held, Decision::Changed));
    assert!(held.assess().is_err());
    held.review = None;
    held.baseline.stable.marker = format!("v0.28.{}", u64::MAX);
    held.baseline.anchor.marker = held.baseline.stable.marker.clone();
    assert!(held.assess().unwrap_err().contains("exhausted"));
}

#[test]
fn bytes() {
    let mut held = request();
    held.candidate.materials[1] =
        Material::new(Kind::Case, "writer".into(), b"new assertion", true).unwrap();
    let unknown = held.assess().unwrap();
    assert_eq!(unknown.state, State::Unknown);
    assert_eq!(unknown.code(), 3);
    assert_eq!(unknown.changes.changed.len(), 1);
    held.review = Some(review(&held, Decision::Changed));
    let changed = held.assess().unwrap();
    assert_eq!(changed.state, State::Changed);
    assert_eq!(changed.marker.as_deref(), Some("v0.29.0"));
    held.review = Some(review(&held, Decision::Same));
    assert_eq!(held.assess().unwrap().marker.as_deref(), Some("v0.28.3"));
}

#[test]
fn coverage() {
    let mut held = request();
    held.candidate
        .materials
        .push(Material::new(Kind::Case, "again".into(), b"old promise", true).unwrap());
    held.candidate.coverage.push(Coverage {
        case: "again".into(),
        platform: "linux-x86_64".into(),
        backend: "none".into(),
    });
    let result = held.assess().unwrap();
    assert_eq!(result.state, State::Changed);
    assert_eq!(result.marker.as_deref(), Some("v0.29.0"));
    held.review = Some(review(&held, Decision::Same));
    assert!(held.assess().unwrap_err().contains("coverage"));
}

#[test]
fn binding() {
    let mut held = request();
    held.candidate.source.commit = "c".repeat(40);
    held.review = Some(review(&held, Decision::Same));
    assert_eq!(held.assess().unwrap().state, State::Same);
    for field in ["before", "after", "anchor"] {
        let mut value = serde_json::to_value(&held).unwrap();
        value["review"][field] = "0".repeat(64).into();
        assert!(Request::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    held.candidate.source.tree = "d".repeat(40);
    assert!(held.assess().is_err());
}

#[test]
fn anchor() {
    let mut held = request();
    held.baseline.stable.marker = "v0.28.3".into();
    held.baseline.stable.snapshot.source.commit = "e".repeat(40);
    assert!(held.assess().unwrap_err().contains("continuity"));
    held.baseline.review = Some(Review {
        before: held.baseline.anchor.snapshot.digest().unwrap(),
        after: held.baseline.stable.snapshot.digest().unwrap(),
        anchor: held.baseline.anchor.snapshot.digest().unwrap(),
        decision: Decision::Same,
        reference: review(&held, Decision::Same).reference,
    });
    held.candidate = held.baseline.stable.snapshot.clone();
    assert_eq!(held.assess().unwrap().marker.as_deref(), Some("v0.28.4"));
    held.baseline.review.as_mut().unwrap().decision = Decision::Changed;
    assert!(held.assess().is_err());
    held.baseline.anchor.marker = "v0.27.0".into();
    assert!(held.assess().is_err());
}

#[test]
fn continuity() {
    let mut held = request();
    held.baseline.stable.marker = "v0.28.3".into();
    held.baseline.stable.snapshot.coverage[0].backend = "pg".into();
    held.candidate = held.baseline.stable.snapshot.clone();
    held.baseline.review = Some(Review {
        before: held.baseline.anchor.snapshot.digest().unwrap(),
        after: held.baseline.stable.snapshot.digest().unwrap(),
        anchor: held.baseline.anchor.snapshot.digest().unwrap(),
        decision: Decision::Same,
        reference: review(&held, Decision::Same).reference,
    });
    assert!(held.assess().unwrap_err().contains("coverage"));
}

#[test]
fn conflict() {
    let mut held = request();
    held.reservations = vec!["v0.29.0-rc.1".into()];
    let result = held.assess().unwrap();
    assert_eq!(result.state, State::Conflict);
    assert_eq!(result.marker, None);
    assert_eq!(result.code(), 1);
}

#[test]
fn reservation() {
    for (reserved, expected) in [("v0.28.4-rc.1", "v0.28.4"), ("v0.28.3-beta.1", "v0.28.3")] {
        let mut held = request();
        held.reservations.push(reserved.into());
        assert_eq!(held.assess().unwrap().marker.as_deref(), Some(expected));
    }
    let mut held = request();
    held.candidate.source.commit = "d".repeat(40);
    held.review = Some(review(&held, Decision::Changed));
    for (reserved, expected) in [("v0.30.0-rc.1", "v0.30.0"), ("v0.30.2-rc.1", "v0.31.0")] {
        held.reservations = vec![reserved.into()];
        assert_eq!(held.assess().unwrap().marker.as_deref(), Some(expected));
    }
    held.reservations = vec!["v1.0.0-rc.1".into()];
    assert_eq!(held.assess().unwrap().state, State::Conflict);
}

#[test]
fn markers() {
    for marker in [
        "v0.29.0",
        "0.28.3",
        "v0.28.3+build",
        "v0.28.3-rc.0",
        "v0.28.3-rc.01",
        "v0.28.3-dev.1",
    ] {
        let mut held = request();
        held.reservations = vec![marker.into()];
        assert!(held.assess().is_err(), "{marker}");
    }
    let mut held = request();
    held.reservations = vec!["v0.28.3-rc.1".into(); 2];
    assert!(held.assess().is_err());
}

#[test]
fn human() {
    let mut held = request();
    held.major = Some(review(&held, Decision::Same).reference);
    let result = held.assess().unwrap();
    assert_eq!(result.state, State::Human);
    assert_eq!(result.code(), 3);
    assert_eq!(result.marker, None);
    assert!(!result.authentication && !result.publication);
    held.major = Some("https://github.com/Other/repo/issues/1#issuecomment-2".into());
    assert!(held.assess().is_err());
}

#[test]
fn encoding() {
    let held = request();
    let bytes = serde_json::to_vec(&held).unwrap();
    assert_eq!(Request::parse(&bytes).unwrap(), held);
    let duplicate = String::from_utf8(bytes.clone()).unwrap().replacen(
        '{',
        "{\"schema\":\"plumb.release.semantics/v1\",",
        1,
    );
    assert!(Request::parse(duplicate.as_bytes()).is_err());
    let duplicate = String::from_utf8(bytes.clone())
        .unwrap()
        .replacen('{', "{\"review\":null,", 1);
    assert!(Request::parse(duplicate.as_bytes()).is_err());
    for field in ["schema", "major", "unknown"] {
        let mut value = serde_json::to_value(&held).unwrap();
        value[field] = true.into();
        assert!(Request::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    assert!(Request::parse(b"").is_err());
    assert!(Request::parse(&vec![b' '; Request::LIMIT + 1]).is_err());
}

#[test]
fn material() {
    let held = snapshot().materials.remove(1);
    assert!(held.verify(b"assert original", true).is_ok());
    assert!(held.verify(b"assert original", false).is_err());
    assert!(held.verify(b"different", true).is_err());
    for name in ["../escape", "/absolute", "a//b", "a/./b", "a\\b"] {
        assert!(Material::new(Kind::Schema, name.into(), b"", false).is_err());
    }
}

#[test]
fn inventory() {
    let mut held = request();
    let digest = held.candidate.digest().unwrap();
    held.candidate.materials.reverse();
    assert_eq!(held.candidate.digest().unwrap(), digest);
    assert_eq!(held.assess().unwrap().state, State::Same);
    held.candidate
        .materials
        .push(held.candidate.materials[0].clone());
    assert!(held.assess().is_err());
    let mut held = request();
    held.candidate.coverage.clear();
    assert!(held.assess().is_err());
    let mut held = request();
    held.candidate.coverage[0].case = "missing".into();
    assert!(held.assess().is_err());
    let mut held = request();
    held.candidate.source.repository = "PerishLab/concord".into();
    assert!(held.assess().is_err());
}
