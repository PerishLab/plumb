use plumb::depot::v3;
use std::collections::BTreeMap;
use std::path::Path;

fn install(root: &Path, version: &str) {
    let stock = super::super::command::support::depot(&[]);
    let seat = plumb::depot::Seat::at(&stock.path().join("configurations")).unwrap();
    let mut bodies = BTreeMap::new();
    let objects = seat
        .manifest()
        .objects
        .iter()
        .map(|object| {
            bodies.insert(
                object.path.clone(),
                seat.read(&object.path).unwrap().into_bytes(),
            );
            v3::Object {
                path: object.path.clone(),
                sha256: object.sha256.clone(),
                size: object.size,
                media: "text/plain".into(),
                executable: false,
            }
        })
        .collect();
    let manifest = v3::Manifest::new(
        v3::Identity {
            product: "plumb".into(),
            channel: "beta".into(),
            version: version.into(),
            marker: v3::Marker {
                name: version.into(),
                sha256: "a".repeat(64),
            },
            kind: v3::Kind::Configuration,
        },
        objects,
    )
    .unwrap();
    let pointer = v3::Pointer::new(
        &manifest,
        v3::Publication {
            source: "https://depot.example",
            prior: None,
            created: "2026-09-08T00:00:00Z".into(),
        },
    )
    .unwrap();
    v3::install(
        &root.join("configurations"),
        &pointer,
        &v3::Bundle { manifest, bodies },
    )
    .unwrap();
}

#[test]
fn related() {
    let fixture = tempfile::tempdir().unwrap();
    let bare = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let running = plumb::version!("PLUMB").to_string();
    let version = format!("{}-beta.1", running.split('-').next().unwrap());
    install(home.path(), &version);
    super::marked(
        fixture.path(),
        bare.path(),
        "https://git.perish.top",
        "v1.2.0-nightly.9",
    );
    let output = super::super::command::plumb(
        fixture.path(),
        &[
            "ship",
            "dispatch",
            "--marker",
            "v1.2.0-nightly.9",
            "--dry-run",
        ],
    )
    .env("PLUMB_HOME", home.path())
    .output()
    .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains(&format!(r#""configuration":"{version}""#)),
        "{text}"
    );
    assert!(text.contains(&format!(r#""plumb":"{running}""#)), "{text}");
    assert!(text.contains(r#""marker":"v1.2.0-nightly.9""#), "{text}");
}

#[test]
fn unrelated() {
    let fixture = tempfile::tempdir().unwrap();
    let bare = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    install(home.path(), "v99.0.0-beta.1");
    super::marked(
        fixture.path(),
        bare.path(),
        "https://git.perish.top",
        "v1.2.0-nightly.9",
    );
    let output = super::super::command::plumb(
        fixture.path(),
        &[
            "ship",
            "dispatch",
            "--marker",
            "v1.2.0-nightly.9",
            "--dry-run",
        ],
    )
    .env("PLUMB_HOME", home.path())
    .output()
    .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains("requires that marker binary or its release base"),
        "{error}"
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("/dispatches"));
}
