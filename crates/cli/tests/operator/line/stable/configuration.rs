use plumb::depot::v3;
use std::collections::BTreeMap;
use std::path::Path;

fn install(root: &Path, version: &str, repository: &Path) {
    let source = std::fs::read_to_string(repository.join("plumb.toml")).unwrap();
    let binding = super::super::marker::configuration(root, &source);
    let base = root
        .join("configurations/generations")
        .join(binding["configuration"].as_str().unwrap());
    let source = v3::Manifest::parse(&std::fs::read(base.join(v3::LEAF)).unwrap()).unwrap();
    let mut bodies = BTreeMap::new();
    let objects = source
        .objects
        .iter()
        .map(|object| {
            bodies.insert(
                object.path.clone(),
                std::fs::read(base.join(&object.path)).unwrap(),
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
    super::marked(
        fixture.path(),
        bare.path(),
        "https://git.perish.top",
        "v1.2.0-nightly.9",
    );
    install(home.path(), &version, fixture.path());
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
    let inputs: serde_json::Value = serde_json::from_str(
        text.split_once("inputs=")
            .unwrap()
            .1
            .trim()
            .trim_end_matches(')'),
    )
    .unwrap();
    let declaration: serde_json::Value =
        serde_json::from_str(inputs["declaration"].as_str().unwrap()).unwrap();
    let controller = &declaration["graph"]["nodes"][0]["execution"]["payload"]["controller"];
    assert_eq!(controller["marker"]["name"], version);
    assert_eq!(controller["generation"].as_str().unwrap().len(), 64);
    assert!(inputs.get("configuration").is_none());
    assert!(text.contains(&format!(r#""plumb":"{running}""#)), "{text}");
    assert!(text.contains(r#""marker":"v1.2.0-nightly.9""#), "{text}");
}

#[test]
fn unrelated() {
    let fixture = tempfile::tempdir().unwrap();
    let bare = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    super::marked(
        fixture.path(),
        bare.path(),
        "https://git.perish.top",
        "v1.2.0-nightly.9",
    );
    install(home.path(), "v99.0.0-beta.1", fixture.path());
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
    let held = String::from_utf8_lossy(&output.stdout);
    assert!(held.contains("/dispatches"), "{held}");
    assert!(held.contains("v99.0.0-beta.1"), "{held}");
    assert!(
        held.contains(&format!(r#""plumb":"{}""#, plumb::version!("PLUMB"))),
        "{held}"
    );
}
