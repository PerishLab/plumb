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
