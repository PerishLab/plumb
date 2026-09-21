use plumb::depot::{Object, Rules, sha};
use std::collections::BTreeMap;

fn fixture() -> (tempfile::TempDir, Rules) {
    let root = tempfile::tempdir().unwrap();
    let bodies: BTreeMap<String, Vec<u8>> = [
        ("rules/products.toml", b"locked catalog".to_vec()),
        ("rules/seat.toml", b"locked probes".to_vec()),
        ("profiles/identity.toml", b"locked profile".to_vec()),
        ("assets/retired.bin", vec![255]),
    ]
    .into_iter()
    .map(|(path, bytes)| (path.into(), bytes))
    .collect();
    let objects = bodies
        .iter()
        .map(|(path, bytes)| Object {
            path: path.clone(),
            sha256: sha(bytes),
            size: bytes.len() as u64,
        })
        .collect();
    let manifest = plumb::guard::Configuration::new(
        "v1.2.3".into(),
        plumb::guard::Validator {
            version: "v1.2.2".into(),
            release: "a".repeat(64),
            artifact: "b".repeat(64),
        },
        objects,
    )
    .unwrap();
    manifest.install(root.path(), &bodies).unwrap();
    let rules = Rules::guard(root.path(), "v1.2.3").unwrap();
    (root, rules)
}

#[test]
fn inherited() {
    let (_root, rules) = fixture();
    let resources = BTreeMap::from([("assets/current.bin".into(), vec![254])]);
    let bodies = rules.inherit(rules.mark(), resources).unwrap();
    assert_eq!(bodies.len(), 4);
    assert_eq!(bodies["rules/products.toml"], b"locked catalog");
    assert_eq!(bodies["rules/seat.toml"], b"locked probes");
    assert_eq!(bodies["profiles/identity.toml"], b"locked profile");
    assert_eq!(bodies["assets/current.bin"], [254]);
    assert!(!bodies.contains_key("assets/retired.bin"));
}

#[test]
fn refused() {
    let (_root, rules) = fixture();
    for path in [
        "rules/seat.toml",
        "profiles/identity.toml",
        "rules/new.toml",
    ] {
        let resources = BTreeMap::from([(path.into(), b"source policy".to_vec())]);
        assert!(
            rules
                .inherit(rules.mark(), resources)
                .unwrap_err()
                .contains("cannot replace Depot policy")
        );
    }
}

#[test]
fn generation() {
    let (_root, rules) = fixture();
    assert!(
        rules
            .inherit(&"f".repeat(64), BTreeMap::new())
            .unwrap_err()
            .contains("generation differs")
    );
}

#[test]
fn path() {
    let (_root, rules) = fixture();
    for path in [
        "./rules/seat.toml",
        "assets/../rules/seat.toml",
        "rules\\seat.toml",
    ] {
        let resources = BTreeMap::from([(path.into(), vec![])]);
        assert!(
            rules
                .inherit(rules.mark(), resources)
                .unwrap_err()
                .contains("not anchored")
        );
    }
}

#[test]
fn drift() {
    let (root, rules) = fixture();
    std::fs::write(root.path().join("rules/seat.toml"), "changed probes").unwrap();
    assert!(rules.inherit(rules.mark(), BTreeMap::new()).is_err());
}
