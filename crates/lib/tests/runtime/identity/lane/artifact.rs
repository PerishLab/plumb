use plumb::lane::{Artifact, Build, Digest, Source};
use serde_json::{Value, json};

fn digest(byte: char) -> Digest {
    Digest::read(&byte.to_string().repeat(64)).unwrap()
}

pub(super) fn sample() -> Artifact {
    Artifact::new(
        digest('a'),
        Source::new("PerishLab/crest".into(), "1".repeat(40), "2".repeat(40)).unwrap(),
        None,
    )
}

#[test]
fn hashes() {
    let value = digest('a');
    assert_eq!(value.value(), "a".repeat(64));
    assert_eq!(serde_json::to_value(&value).unwrap(), json!(value.value()));
    assert_eq!(
        serde_json::from_value::<Digest>(json!(value.value())).unwrap(),
        value
    );
    for text in [
        "a".repeat(63),
        "a".repeat(65),
        "A".repeat(64),
        "g".repeat(64),
        "é".repeat(32),
        format!("{}\n", "a".repeat(63)),
        "sha256:latest".into(),
    ] {
        assert!(Digest::read(&text).is_err());
        assert!(serde_json::from_value::<Digest>(json!(text)).is_err());
    }
    for value in [json!(null), json!(1), json!({}), json!([])] {
        assert!(serde_json::from_value::<Digest>(value).is_err());
    }
}

#[test]
fn provenance() {
    let plain = sample();
    assert!(plain.build().is_none());
    assert_eq!(plain.source().repository(), "PerishLab/crest");
    assert_eq!(plain.source().commit(), "1".repeat(40));
    assert_eq!(plain.source().tree(), "2".repeat(40));
    let build = Build::new(digest('b'), digest('c'));
    assert_eq!(build.inputs().value(), "b".repeat(64));
    assert_eq!(build.world().value(), "c".repeat(64));
    let built = Artifact::new(plain.digest().clone(), plain.source().clone(), Some(build));
    assert_eq!(built.digest(), plain.digest());
    assert_ne!(built, plain);
    for artifact in [plain, built] {
        let value = serde_json::to_value(&artifact).unwrap();
        assert_eq!(serde_json::from_value::<Artifact>(value).unwrap(), artifact);
    }
}

#[test]
fn separation() {
    let original = sample();
    let source = Source::new("PerishLab/design".into(), "3".repeat(40), "4".repeat(40));
    let other = Artifact::new(original.digest().clone(), source.unwrap(), None);
    assert_eq!(other.digest(), original.digest());
    assert_ne!(other.source(), original.source());
    assert_ne!(other, original);
    let bytes = Artifact::new(digest('b'), original.source().clone(), None);
    assert_eq!(bytes.source(), original.source());
    assert_ne!(bytes.digest(), original.digest());
}

#[test]
fn refusals() {
    let original = serde_json::to_value(sample()).unwrap();
    for (field, value) in [
        ("repository", json!("../crest")),
        ("commit", json!("main")),
        ("commit", json!("1".repeat(39))),
        ("tree", json!("A".repeat(40))),
        ("tree", json!(null)),
        ("token", json!("not-a-permit")),
    ] {
        let mut held = original.clone();
        held["source"][field] = value;
        assert!(serde_json::from_value::<Artifact>(held).is_err(), "{field}");
    }
    for field in ["repository", "commit", "tree"] {
        let mut held = original.clone();
        held["source"].as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<Artifact>(held).is_err());
    }
    for field in ["url", "stamp", "deployment", "credentials"] {
        let mut held = original.clone();
        held[field] = json!("not-part-of-content");
        assert!(serde_json::from_value::<Artifact>(held).is_err());
    }
    for build in [
        json!({"inputs":"b".repeat(64)}),
        json!({"inputs":"b".repeat(64),"world":"latest"}),
        json!({"inputs":"b".repeat(64),"world":"c".repeat(64),"stamp":"v1.2.3"}),
    ] {
        let mut held = original.clone();
        held["build"] = build;
        assert!(serde_json::from_value::<Artifact>(held).is_err());
    }
    for field in ["digest", "source"] {
        let mut held = original.clone();
        held.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<Artifact>(held).is_err());
    }
    let repeated = format!(
        r#"{{"repository":"PerishLab/crest","commit":"{}","tree":"{}","tree":"{}"}}"#,
        "1".repeat(40),
        "2".repeat(40),
        "3".repeat(40)
    );
    assert!(serde_json::from_str::<Source>(&repeated).is_err());
    let value: Value =
        json!({"repository":"PerishLab/crest","commit":"main","tree":"2".repeat(40)});
    assert!(serde_json::from_value::<Source>(value).is_err());
    assert!(Source::new("PerishLab/crest".into(), "main".into(), "2".repeat(40)).is_err());
}
