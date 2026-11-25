use super::fixture::{Mock, bundle, dead, invoke, publish};
use plumb::depot::v3;
use serde_json::Value;
use std::collections::BTreeMap;

#[test]
fn authority() {
    let home = tempfile::tempdir().expect("home");
    let unreachable = dead();
    let output = invoke(
        home.path(),
        &["release", "authority", "--json"],
        Some(&unreachable),
        None,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("authority json");
    assert_eq!(report["producer"], plumb::version!("PLUMB"));
    assert!(report.get("depot").is_none(), "{report}");
    assert_eq!(report.as_object().expect("object").len(), 1);

    let text = invoke(
        home.path(),
        &["release", "authority"],
        Some(&unreachable),
        None,
    );
    assert!(text.status.success());
    let stdout = String::from_utf8_lossy(&text.stdout);
    assert_eq!(
        stdout.trim(),
        format!("producer {}", plumb::version!("PLUMB"))
    );
}

#[test]
fn floating() {
    let (listener, source) = Mock::open();
    let held = bundle("v0.163.0", Some(b"v1\n"));
    let mut routes = BTreeMap::new();
    let pointer = publish(&mut routes, &source, &held);
    let key = format!(
        "/{}",
        v3::channel("stable", v3::Kind::Configuration).expect("channel")
    );
    routes.insert(key.clone(), pointer.encode().expect("pointer"));
    let server = Mock::start(listener, routes);

    let home = tempfile::tempdir().expect("home");
    let output = invoke(
        home.path(),
        &["rule", "list", "--json"],
        Some(&source),
        None,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let cached = home
        .path()
        .join("depot/generations")
        .join(&pointer.generation)
        .join(v3::LEAF);
    assert!(cached.is_file());

    drop(server);
    let offline = invoke(
        home.path(),
        &["rule", "list", "--json"],
        Some(&source),
        None,
    );
    assert!(!offline.status.success());

    let (listener, bad) = Mock::open();
    let mut broken = BTreeMap::new();
    let pointer = publish(&mut broken, &bad, &held);
    broken.insert(key, pointer.encode().expect("pointer"));
    let path = pointer
        .manifest
        .url
        .strip_prefix(&bad)
        .expect("path")
        .to_string();
    broken.insert(path, b"{}".to_vec());
    let _server = Mock::start(listener, broken);
    let fresh = tempfile::tempdir().expect("fresh");
    let tampered = invoke(fresh.path(), &["rule", "list", "--json"], Some(&bad), None);
    assert!(!tampered.status.success());
}

#[test]
fn pinned() {
    let (listener, source) = Mock::open();
    let first = bundle("v0.163.0", Some(b"first\n"));
    let second = bundle("v0.164.0", Some(b"second\n"));
    let mut routes = BTreeMap::new();
    let kept = publish(&mut routes, &source, &first);
    let drifted = publish(&mut routes, &source, &second);
    let key = format!(
        "/{}",
        v3::channel("stable", v3::Kind::Configuration).expect("channel")
    );
    routes.insert(key, drifted.encode().expect("drifted"));
    let server = Mock::start(listener, routes);

    let pin = v3::Pinned::pointer(&kept)
        .expect("pin")
        .encode()
        .expect("encode");
    let home = tempfile::tempdir().expect("home");
    let output = invoke(
        home.path(),
        &["rule", "list", "--json"],
        Some(&source),
        Some(&pin),
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let generations = home.path().join("depot/generations");
    assert!(generations.join(&kept.generation).join(v3::LEAF).is_file());
    assert!(!generations.join(&drifted.generation).exists());

    drop(server);
    let hit = invoke(
        home.path(),
        &["rule", "list", "--json"],
        Some(&source),
        Some(&pin),
    );
    assert!(
        hit.status.success(),
        "{}",
        String::from_utf8_lossy(&hit.stderr)
    );

    let foreign = pin.replace(&source, "https://depot.other.perish.uk");
    let unknown = pin.replacen('{', "{\"extra\":1,", 1);
    let badsha = pin.replace(&kept.manifest.sha256, &"0".repeat(64));
    for bad in ["", "{}", &foreign, &unknown, &badsha] {
        let refused = invoke(
            home.path(),
            &["rule", "list", "--json"],
            Some(&source),
            Some(bad),
        );
        assert!(!refused.status.success(), "expected refusal for {bad}");
    }
}

#[test]
fn recovery() {
    let home = tempfile::tempdir().expect("home");
    let unreachable = dead();
    let output = invoke(
        home.path(),
        &[
            "depot",
            "consign",
            "--version",
            "v0.1.0",
            "--kind",
            "configuration",
            "--dry-run",
        ],
        Some(&unreachable),
        None,
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("cannot resolve Plumb rules"), "{stderr}");
}
