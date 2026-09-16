#[test]
fn sources() {
    use plumb::depot::v3;
    assert!(v3::check("https://depot.example.test").is_ok());
    assert!(v3::check("http://127.0.0.1:1234").is_ok());
    assert!(v3::check("http://localhost:1234/path").is_ok());
    for invalid in [
        "http://localhost:1234@remote.test",
        "http://127.0.0.1:bad",
        "http://localhost:0",
    ] {
        assert!(v3::check(invalid).is_err());
    }
    assert!(v3::check("http://depot.example.test").is_err());
    assert!(v3::check("https://depot.example.test/").is_err());
    assert!(v3::check("https://depot.example.test source").is_err());
}

#[test]
fn temporary() {
    let bytes = b"answer = 42\n";
    let bodies =
        std::collections::BTreeMap::from([("rules/probe.toml".to_string(), bytes.to_vec())]);
    let manifest = plumb::guard::Configuration::new(
        "v1.2.3".into(),
        plumb::guard::Validator {
            version: "v1.2.2".into(),
            release: "a".repeat(64),
            artifact: "b".repeat(64),
        },
        vec![plumb::depot::Object {
            path: "rules/probe.toml".into(),
            sha256: plumb::depot::sha(bytes),
            size: bytes.len() as u64,
        }],
    )
    .expect("guard configuration");
    let root = tempfile::tempdir().expect("guard seat");
    manifest
        .install(root.path(), &bodies)
        .expect("installation");
    let held = plumb::guard::Configuration::open(root.path(), "v1.2.3").expect("open");
    assert_eq!(held.digest(), manifest.digest());
    assert_eq!(
        held.read(root.path(), "rules/probe.toml").expect("rule"),
        "answer = 42\n"
    );
    assert!(plumb::guard::Configuration::open(root.path(), "v1.2.4").is_err());
}

#[test]
fn bootstrap() {
    let mut evidence = plumb::guard::Bootstrap {
        marker: plumb::depot::v3::Marker {
            name: "v1.2.3".into(),
            sha256: "a".repeat(64),
        },
        generation: "b".repeat(64),
        controller: plumb::guard::Bootstrap::executable().unwrap(),
        configuration: "c".repeat(64),
        validator: plumb::guard::Validator {
            version: "v1.2.3".into(),
            release: "d".repeat(64),
            artifact: "e".repeat(64),
        },
    };
    evidence.current().unwrap();
    let proof = plumb::guard::Descriptor {
        schema: plumb::guard::SCHEMA.into(),
        repository: "PerishFire/plumb".into(),
        tree: "a".repeat(40),
        plumb: "v1.2.2@source-controller".into(),
        depot: evidence.generation.clone(),
        platform: "linux-arm64".into(),
        actions: vec![plumb::guard::Action {
            name: "guard/rust".into(),
            input: "a".repeat(64),
            world: "b".repeat(64),
        }],
        bootstrap: None,
        digest: String::new(),
    }
    .bootstrap(evidence.clone())
    .unwrap();
    assert_eq!(
        plumb::guard::Descriptor::decode(&proof.encode().unwrap()).unwrap(),
        proof
    );
    for field in ["generation", "controller", "configuration"] {
        let mut value = serde_json::to_value(&proof).unwrap();
        value["bootstrap"][field] = serde_json::Value::String("0".repeat(64));
        let changed: plumb::guard::Descriptor = serde_json::from_value(value).unwrap();
        assert!(changed.validate().unwrap_err().contains("digest disagrees"));
    }
    evidence.validator.version = "v1.2.4".into();
    assert!(evidence.validate().is_err());
    evidence.validator.version = "v1.2.3".into();
    evidence.controller = "f".repeat(64);
    assert!(
        evidence
            .current()
            .unwrap_err()
            .contains("another controller")
    );
    evidence.generation = "a".repeat(40);
    assert!(evidence.validate().is_err());
}
