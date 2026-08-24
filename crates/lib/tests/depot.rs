use plumb::depot::{FORMAT, Manifest, Metadata, Object, Pointer, Schema, Seat, sha};
use std::path::Path;

fn stock(root: &Path, body: &str) -> (Manifest, Pointer) {
    let metadata = Metadata {
        version: "29990101T000000Z".to_string(),
        source: "fixture".to_string(),
        channel: "stable".to_string(),
        commit: "abc123".to_string(),
    };
    let manifest = Manifest {
        schema: Schema {
            format: FORMAT,
            version: "v0.0.1".to_string(),
        },
        metadata: metadata.clone(),
        objects: vec![Object {
            path: "rules/probe.toml".to_string(),
            sha256: sha(body.as_bytes()),
            size: body.len() as u64,
        }],
    };
    let pointer = Pointer::new(&metadata, "plumb");
    let base = root.join(&metadata.version);
    std::fs::create_dir_all(base.join("rules")).expect("rule seat");
    std::fs::write(base.join("rules/probe.toml"), body).expect("rule");
    std::fs::write(
        base.join("plumb.toml"),
        manifest.encode().expect("manifest"),
    )
    .expect("manifest");
    std::fs::write(
        root.join("metadata.json"),
        pointer.encode().expect("pointer"),
    )
    .expect("pointer");
    (manifest, pointer)
}

#[test]
fn verification() {
    let root = tempfile::tempdir().expect("depot");
    stock(root.path(), "answer = 42\n");
    let seat = Seat::at(root.path()).expect("seat");
    assert_eq!(
        seat.read("rules/probe.toml").expect("rule"),
        "answer = 42\n"
    );
    std::fs::write(
        root.path().join("29990101T000000Z/rules/probe.toml"),
        "answer = 41\n",
    )
    .expect("drift");
    assert!(
        seat.read("rules/probe.toml")
            .unwrap_err()
            .contains("object drift")
    );
}

#[test]
fn binding() {
    let root = tempfile::tempdir().expect("depot");
    let (_, mut pointer) = stock(root.path(), "answer = 42\n");
    pointer.commit = "different".to_string();
    std::fs::write(
        root.path().join("metadata.json"),
        pointer.encode().expect("pointer"),
    )
    .expect("pointer");
    let error = match Seat::at(root.path()) {
        Ok(_) => panic!("unbound seat was accepted"),
        Err(error) => error,
    };
    assert!(error.contains("does not bind"));
}

#[test]
fn paths() {
    let (mut manifest, _) = stock(tempfile::tempdir().expect("depot").path(), "answer = 42\n");
    manifest.objects[0].path = "../rules.toml".to_string();
    assert!(Manifest::parse(&manifest.encode().expect("manifest")).is_err());
    manifest.objects[0].path = "rules/probe.toml".to_string();
    manifest.objects.push(manifest.objects[0].clone());
    assert!(
        Manifest::parse(&manifest.encode().expect("manifest"))
            .unwrap_err()
            .contains("repeats")
    );
}
