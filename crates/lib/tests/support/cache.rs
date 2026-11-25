use super::fixture::{Server, ask, bundle, publish};
use plumb::depot::{Rules, v3};
use std::collections::BTreeMap;

#[test]
fn populate() {
    let home = tempfile::tempdir().expect("home");
    let (listener, source) = Server::open();
    let held = bundle(
        "stable",
        "v1.2.3",
        &[
            ("manifest.json", b"{\"nested\":true}\n"),
            ("rules/catalog.toml", b"answer = 42\n"),
        ],
    );
    let mut routes = BTreeMap::new();
    let pointer = publish(
        &mut routes,
        &held,
        v3::Publication {
            source: &source,
            prior: None,
            created: "2026-10-28T00:00:00Z".into(),
        },
    );
    let key = format!(
        "/{}",
        v3::channel("stable", v3::Kind::Configuration).expect("key")
    );
    routes.insert(key, pointer.encode().expect("encode"));
    let server = Server::start(listener, routes);

    let first = ask(&source).cached(home.path()).expect("miss populates");
    assert_eq!(server.hits(), 5);
    assert!(!home.path().join("configurations").exists());
    let dir = home
        .path()
        .join("depot/generations")
        .join(&pointer.generation);
    assert!(dir.join(v3::LEAF).is_file());
    assert!(dir.join("objects/manifest.json").is_file());
    assert!(dir.join("objects/rules/catalog.toml").is_file());
    assert_eq!(first.bundle.bodies["manifest.json"], b"{\"nested\":true}\n");
    let rules = Rules::bundle(&first.bundle).expect("rules");
    assert_eq!(rules.mark(), pointer.generation);

    let second = ask(&source).cached(home.path()).expect("warm hit");
    assert_eq!(server.hits(), 7);
    assert_eq!(second.bundle.bodies, first.bundle.bodies);

    let raw = std::fs::read(dir.join(v3::LEAF)).expect("raw manifest");
    v3::Cache::new(home.path())
        .store(&pointer, &raw, &first.bundle)
        .expect("idempotent store");

    drop(server);
    assert!(ask(&source).cached(home.path()).is_err());
}

#[test]
fn tampered() {
    let home = tempfile::tempdir().expect("home");
    let (listener, source) = Server::open();
    let held = bundle("stable", "v1.2.3", &[("rules/a.toml", b"answer = 42\n")]);
    let mut routes = BTreeMap::new();
    let pointer = publish(
        &mut routes,
        &held,
        v3::Publication {
            source: &source,
            prior: None,
            created: "2026-10-28T00:00:00Z".into(),
        },
    );
    let key = format!(
        "/{}",
        v3::channel("stable", v3::Kind::Configuration).expect("key")
    );
    routes.insert(key, pointer.encode().expect("encode"));
    let _server = Server::start(listener, routes);

    ask(&source).cached(home.path()).expect("seed cache");
    let dir = home
        .path()
        .join("depot/generations")
        .join(&pointer.generation);
    let object = dir.join("objects/rules/a.toml");
    std::fs::write(&object, b"answer = 99\n").expect("tamper object");
    assert!(ask(&source).cached(home.path()).is_err());

    std::fs::write(&object, b"answer = 42\n").expect("restore object");
    ask(&source).cached(home.path()).expect("restored");

    let manifest = dir.join(v3::LEAF);
    std::fs::write(&manifest, b"{}").expect("tamper manifest");
    assert!(ask(&source).cached(home.path()).is_err());
}

#[test]
#[cfg(unix)]
fn link() {
    use std::os::unix::fs::symlink;
    let home = tempfile::tempdir().expect("home");
    let (listener, source) = Server::open();
    let held = bundle("stable", "v1.2.3", &[("rules/a.toml", b"answer = 42\n")]);
    let mut routes = BTreeMap::new();
    let pointer = publish(
        &mut routes,
        &held,
        v3::Publication {
            source: &source,
            prior: None,
            created: "2026-10-28T00:00:00Z".into(),
        },
    );
    let key = format!(
        "/{}",
        v3::channel("stable", v3::Kind::Configuration).expect("key")
    );
    routes.insert(key, pointer.encode().expect("encode"));
    let _server = Server::start(listener, routes);

    ask(&source).cached(home.path()).expect("seed cache");
    let dir = home
        .path()
        .join("depot/generations")
        .join(&pointer.generation);
    let object = dir.join("objects/rules/a.toml");
    let outside = home.path().join("outside.toml");
    std::fs::write(&outside, b"answer = 42\n").expect("outside");
    std::fs::remove_file(&object).expect("remove");
    symlink(&outside, &object).expect("symlink");
    assert!(ask(&source).cached(home.path()).is_err());
}

#[test]
fn standing() {
    let (listener, source) = Server::open();
    let held = bundle("stable", "v1.2.3", &[("rules/a.toml", b"answer = 42\n")]);
    let mut routes = BTreeMap::new();
    let pointer = publish(
        &mut routes,
        &held,
        v3::Publication {
            source: &source,
            prior: None,
            created: "2026-10-28T00:00:00Z".into(),
        },
    );
    let key = format!(
        "/{}",
        v3::channel("stable", v3::Kind::Configuration).expect("key")
    );
    routes.insert(key.clone(), pointer.encode().expect("encode"));
    let latest = format!(
        "/{}",
        v3::latest(v3::Route::new("stable", v3::Kind::Configuration, "v1.2.3")).expect("latest")
    );
    let mut mismatch = pointer.clone();
    mismatch.marker.sha256 = "b".repeat(64);
    routes.insert(latest.clone(), mismatch.encode().expect("mismatch"));
    let _server = Server::start(listener, routes);
    assert!(ask(&source).fetch().is_err());

    let (listener, source) = Server::open();
    let mut routes = BTreeMap::new();
    let pointer = publish(
        &mut routes,
        &held,
        v3::Publication {
            source: &source,
            prior: None,
            created: "2026-10-28T00:00:00Z".into(),
        },
    );
    routes.insert(key.clone(), pointer.encode().expect("encode"));
    let mut conflict = pointer.clone();
    conflict.manifest.sha256 = "c".repeat(64);
    routes.insert(latest, conflict.encode().expect("conflict"));
    let _conflict = Server::start(listener, routes);
    assert!(ask(&source).fetch().is_err());

    let (listener, source) = Server::open();
    let mut routes = BTreeMap::new();
    let pointer = publish(
        &mut routes,
        &held,
        v3::Publication {
            source: &source,
            prior: None,
            created: "2026-10-28T00:00:00Z".into(),
        },
    );
    routes.insert(key, pointer.encode().expect("encode"));
    let path = pointer
        .manifest
        .url
        .strip_prefix(&source)
        .expect("manifest path")
        .to_string();
    routes.insert(path, b"{}".to_vec());
    let _server = Server::start(listener, routes);
    assert!(ask(&source).fetch().is_err());
}
