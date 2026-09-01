#[test]
fn temporary() {
    let bytes = b"answer = 42\n";
    let bodies =
        std::collections::BTreeMap::from([("rules/probe.toml".to_string(), bytes.to_vec())]);
    let manifest = plumb::guard::Configuration::new(
        "v1.2.3".into(),
        plumb::guard::Validator {
            version: "v1.2.2".into(),
            marker: "a".repeat(64),
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
