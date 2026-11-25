use super::fixture::{Server, ask, bundle, publish};
use plumb::depot::{Rules, sha, v3};
use std::collections::BTreeMap;

#[test]
fn discover() {
    assert_eq!(
        v3::channel("stable", v3::Kind::Configuration).expect("route"),
        "channels/stable/configurations/latest.json"
    );
    let (listener, source) = Server::open();
    let held = bundle(
        "stable",
        "v1.2.3",
        &[("rules/catalog.toml", b"answer = 42\n")],
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
    let _server = Server::start(listener, routes);
    let found = ask(&source).fetch().expect("discover");
    assert_eq!(found.pointer, pointer);
    let rules = Rules::bundle(&found.bundle).expect("rules");
    assert_eq!(
        rules.mark(),
        found.bundle.manifest.generation().expect("generation")
    );
    assert_eq!(
        rules.read("rules/catalog.toml").expect("read"),
        "answer = 42\n"
    );

    let mut wrong = found.bundle;
    wrong.manifest.kind = v3::Kind::Skill;
    assert!(Rules::bundle(&wrong).is_err());
    wrong.manifest.kind = v3::Kind::Configuration;
    wrong
        .bodies
        .insert("rules/catalog.toml".into(), b"tampered".to_vec());
    assert!(Rules::bundle(&wrong).is_err());
}

#[test]
fn rollback() {
    let key = format!(
        "/{}",
        v3::channel("stable", v3::Kind::Configuration).expect("key")
    );
    let (listener, source) = Server::open();
    let mut routes = BTreeMap::new();
    let first = bundle("stable", "v1.2.0", &[("rules/a.toml", b"v = 1\n")]);
    let newer = bundle("stable", "v1.3.0", &[("rules/a.toml", b"v = 2\n")]);
    let standing = publish(
        &mut routes,
        &first,
        v3::Publication {
            source: &source,
            prior: None,
            created: "2026-10-28T00:00:00Z".into(),
        },
    );
    let replaced = publish(
        &mut routes,
        &newer,
        v3::Publication {
            source: &source,
            prior: Some(standing.generation.clone()),
            created: "2026-10-28T01:00:00Z".into(),
        },
    );
    let rolled = v3::Pointer::new(
        &first.manifest,
        v3::Publication {
            source: &source,
            prior: Some(replaced.generation),
            created: "2026-10-28T02:00:00Z".into(),
        },
    )
    .expect("cross rollback");
    routes.insert(key.clone(), rolled.encode().expect("encode"));
    let _server = Server::start(listener, routes);
    let found = ask(&source).fetch().expect("cross-version rollback");
    assert_eq!(found.pointer.generation, standing.generation);

    let (listener, source) = Server::open();
    let mut routes = BTreeMap::new();
    let first = bundle("stable", "v1.2.0", &[("rules/a.toml", b"v = 1\n")]);
    let second = bundle("stable", "v1.2.0", &[("rules/a.toml", b"v = 2\n")]);
    let older = publish(
        &mut routes,
        &first,
        v3::Publication {
            source: &source,
            prior: None,
            created: "2026-10-28T00:00:00Z".into(),
        },
    );
    let latest = publish(
        &mut routes,
        &second,
        v3::Publication {
            source: &source,
            prior: Some(older.generation.clone()),
            created: "2026-10-28T01:00:00Z".into(),
        },
    );
    let intra = v3::Pointer::new(
        &first.manifest,
        v3::Publication {
            source: &source,
            prior: Some(latest.generation),
            created: "2026-10-28T02:00:00Z".into(),
        },
    )
    .expect("intra rollback");
    routes.insert(key, intra.encode().expect("encode"));
    let _server = Server::start(listener, routes);
    let found = ask(&source).fetch().expect("intra-version rollback");
    assert_eq!(found.pointer.generation, older.generation);
    assert_eq!(found.bundle.bodies["rules/a.toml"], b"v = 1\n");
}

#[test]
fn closed() {
    let (listener, source) = Server::open();
    let _empty = Server::start(listener, BTreeMap::new());
    assert!(ask(&source).fetch().is_err());

    let key = format!(
        "/{}",
        v3::channel("stable", v3::Kind::Configuration).expect("key")
    );
    let (listener, source) = Server::open();
    let held = bundle("stable", "v1.2.0", &[("rules/a.toml", b"v = 1\n")]);
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
    let latest = format!(
        "/{}",
        v3::latest(v3::Route::new("stable", v3::Kind::Configuration, "v1.2.0")).expect("latest")
    );
    routes.remove(&latest);
    let _missing = Server::start(listener, routes);
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
    let object = pointer
        .manifest
        .url
        .strip_prefix(&source)
        .expect("path")
        .replace(v3::LEAF, "objects/rules/a.toml");
    routes.insert(object, b"tampered".to_vec());
    let _tampered = Server::start(listener, routes);
    assert!(ask(&source).fetch().is_err());
}

#[test]
fn rejected() {
    for (channel, version) in [
        ("stable", "v1.2.3-rc.1"),
        ("rc", "v1.2.3"),
        ("alpha", "v1.2.3-beta.1"),
        ("beta", "v1.2.3-beta.0"),
        ("unknown", "v1.2.3"),
    ] {
        let identity = v3::Identity {
            product: "plumb".into(),
            channel: channel.into(),
            version: version.into(),
            marker: v3::Marker {
                name: version.into(),
                sha256: "a".repeat(64),
            },
            kind: v3::Kind::Configuration,
        };
        let object = v3::Object {
            path: "rules/a.toml".into(),
            sha256: sha(b"a = 1\n"),
            size: 6,
            media: "application/toml".into(),
            executable: false,
        };
        assert!(v3::Manifest::new(identity, vec![object]).is_err());
    }
    for bad in [
        "",
        "/a.toml",
        "a.toml/",
        "a//b.toml",
        "a/./b.toml",
        "a/../b.toml",
        "a\\b.toml",
    ] {
        let identity = v3::Identity {
            product: "plumb".into(),
            channel: "stable".into(),
            version: "v1.2.3".into(),
            marker: v3::Marker {
                name: "v1.2.3".into(),
                sha256: "a".repeat(64),
            },
            kind: v3::Kind::Configuration,
        };
        let object = v3::Object {
            path: bad.into(),
            sha256: sha(b"a = 1\n"),
            size: 6,
            media: "application/toml".into(),
            executable: false,
        };
        assert!(v3::Manifest::new(identity, vec![object]).is_err(), "{bad}");
    }
}
