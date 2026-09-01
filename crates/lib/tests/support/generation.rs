use plumb::depot::{sha, v3};

fn manifest() -> v3::Manifest {
    let bytes = b"answer = 42\n";
    v3::Manifest::new(
        v3::Identity {
            product: "probe".to_string(),
            channel: "stable".to_string(),
            version: "v1.2.3".to_string(),
            marker: v3::Marker {
                name: "v1.2.3".to_string(),
                sha256: "a".repeat(64),
            },
            kind: v3::Kind::Configuration,
        },
        vec![v3::Object {
            path: "rules/probe.toml".to_string(),
            sha256: sha(bytes),
            size: bytes.len() as u64,
            media: "text/plain".to_string(),
            executable: false,
        }],
    )
    .expect("manifest")
}

#[test]
fn identity() {
    let held = manifest();
    let generation = held.generation().expect("generation");
    let body = held.encode().expect("manifest body");
    let parsed = v3::Manifest::parse(&body).expect("parsed manifest");
    assert_eq!(parsed, held);
    assert_eq!(parsed.generation().expect("parsed generation"), generation);
    let route = v3::Route::new("stable", v3::Kind::Configuration, "v1.2.3");
    assert_eq!(
        v3::generation(route, &generation).expect("generation route"),
        format!("channels/stable/configurations/versions/v1.2.3/generations/{generation}")
    );
    assert_eq!(
        v3::latest(route).expect("latest route"),
        "channels/stable/configurations/versions/v1.2.3/latest.json"
    );
}

#[test]
fn pointer() {
    let held = manifest();
    let body = held.encode().expect("manifest body");
    let pointer = v3::Pointer::new(
        &held,
        v3::Publication {
            source: "https://depot.example.test",
            prior: None,
            created: "2026-09-01T01:02:03Z".to_string(),
        },
    )
    .expect("pointer");
    pointer.bind(&held, &body).expect("pointer binding");
    assert!(!pointer.advance(&pointer).expect("idempotent pointer"));

    let mut changed = held.clone();
    changed.objects[0].sha256 = "b".repeat(64);
    let next = v3::Pointer::new(
        &changed,
        v3::Publication {
            source: "https://depot.example.test",
            prior: Some(pointer.generation.clone()),
            created: "2026-09-01T01:03:03Z".to_string(),
        },
    )
    .expect("next pointer");
    assert!(pointer.advance(&next).expect("pointer advance"));

    let mut skipped = next.clone();
    skipped.prior = Some("c".repeat(64));
    assert!(pointer.advance(&skipped).is_err());
    let mut drift = next;
    drift.manifest.url = "https://depot.example.test/manifest.json".to_string();
    assert!(drift.encode().is_err());
}

#[test]
fn source() {
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt as _;

    let root = tempfile::tempdir().expect("source");
    std::fs::create_dir(root.path().join("rules")).expect("rules");
    std::fs::write(root.path().join("rules/probe.toml"), "answer = 42\n").expect("rule");
    std::fs::write(root.path().join("SKILL.md"), "# Probe\n").expect("skill");
    #[cfg(unix)]
    std::fs::set_permissions(
        root.path().join("SKILL.md"),
        std::fs::Permissions::from_mode(0o755),
    )
    .expect("mode");
    let bundle = v3::Bundle::read(
        root.path(),
        v3::Identity {
            product: "probe".to_string(),
            channel: "stable".to_string(),
            version: "v1.2.3".to_string(),
            marker: v3::Marker {
                name: "v1.2.3".to_string(),
                sha256: "a".repeat(64),
            },
            kind: v3::Kind::Skill,
        },
    )
    .expect("bundle");
    assert_eq!(
        bundle.bodies.keys().cloned().collect::<Vec<_>>(),
        ["SKILL.md".to_string(), "rules/probe.toml".to_string()]
    );
    assert_eq!(bundle.manifest.objects[0].media, "text/plain");
    #[cfg(unix)]
    assert!(bundle.manifest.objects[0].executable);
    let repeated = v3::Bundle::read(root.path(), bundle.manifest.identity()).expect("repeat");
    assert_eq!(
        repeated.manifest.generation().expect("repeated generation"),
        bundle.manifest.generation().expect("generation")
    );
}

#[test]
fn installation() {
    let source = tempfile::tempdir().expect("source");
    std::fs::create_dir(source.path().join("rules")).expect("rules");
    std::fs::write(source.path().join("rules/probe.toml"), "answer = 42\n").expect("rule");
    let mut identity = manifest().identity();
    identity.product = "plumb".to_string();
    let bundle = v3::Bundle::read(source.path(), identity).expect("bundle");
    let pointer = v3::Pointer::new(
        &bundle.manifest,
        v3::Publication {
            source: "https://depot.example.test",
            prior: None,
            created: "2026-09-01T01:02:03Z".to_string(),
        },
    )
    .expect("pointer");
    let root = tempfile::tempdir().expect("installation");
    let placed = v3::install(root.path(), &pointer, &bundle).expect("install");
    assert_eq!(
        std::fs::read_to_string(placed.join("rules/probe.toml")).expect("installed rule"),
        "answer = 42\n"
    );
    assert_eq!(
        v3::Pointer::parse(&std::fs::read(root.path().join(v3::POINTER)).expect("local pointer"))
            .expect("pointer"),
        pointer
    );
    let rules = plumb::depot::Rules::at(root.path(), "v1.2.3").expect("installed rules");
    assert_eq!(rules.mark(), pointer.generation);
    assert_eq!(rules.version(), Some("v1.2.3"));
    assert_eq!(
        rules.read("rules/probe.toml").expect("verified rule"),
        "answer = 42\n"
    );
    assert!(plumb::depot::Rules::at(root.path(), "v1.2.4").is_err());
}

#[test]
#[cfg(unix)]
fn symbolic() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().expect("source");
    std::fs::write(root.path().join("target"), "held\n").expect("target");
    symlink("target", root.path().join("alias")).expect("link");
    let mut identity = manifest().identity();
    identity.kind = v3::Kind::Changelog;
    assert!(
        v3::Bundle::read(root.path(), identity)
            .unwrap_err()
            .contains("special object")
    );
}
