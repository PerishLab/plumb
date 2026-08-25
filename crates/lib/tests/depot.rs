use plumb::depot::{FORMAT, Manifest, Metadata, Object, Pointer, Schema, Seat, sha, supports};
use semver::Version;
use std::path::Path;

fn derivative() -> plumb::depot::v2::Manifest {
    use plumb::depot::v2::{FORMAT, Kind, Release, Seal, Snapshot};
    let bytes = b"answer = 42\n";
    plumb::depot::v2::Manifest {
        format: FORMAT,
        source: "https://depot.example.test".to_string(),
        derivative: Kind::Configuration,
        release: Release {
            product: "probe".to_string(),
            channel: "stable".to_string(),
            version: "v1.2.3".to_string(),
            commit: "0123456789abcdef0123456789abcdef01234567".to_string(),
            seal: Seal {
                url: "https://releases.example.test/v1.2.3/seal.json".to_string(),
                sha256: "a".repeat(64),
            },
        },
        snapshot: Snapshot {
            timestamp: "20260825T010203Z".to_string(),
            commit: "abcdef0123456789abcdef0123456789abcdef01".to_string(),
        },
        objects: vec![Object {
            path: "rules/probe.toml".to_string(),
            sha256: sha(bytes),
            size: bytes.len() as u64,
        }],
    }
}

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

#[test]
fn prerelease() {
    let beta = Version::parse("0.31.0-beta.3").expect("beta");
    let stable = Version::parse("0.31.0").expect("stable");
    let next = Version::parse("0.32.0").expect("next");
    let newer = Version::parse("0.31.0-beta.4").expect("newer beta");

    assert!(supports(&beta, &stable));
    assert!(!supports(&beta, &next));
    assert!(!supports(&beta, &newer));
}

#[test]
fn sealed() {
    let held = derivative();
    let text = held.encode().expect("manifest");
    let parsed = plumb::depot::v2::Manifest::parse(&text).expect("parsed manifest");
    let pointer = plumb::depot::v2::Pointer::new(&parsed, text.as_bytes()).expect("pointer");
    pointer.bind(&parsed, text.as_bytes()).expect("bound");
    assert!(!pointer.advance(&pointer).expect("idempotent"));
    let mut next = pointer.clone();
    next.snapshot.timestamp = "20260825T010204Z".to_string();
    assert!(pointer.advance(&next).expect("advance"));
    assert!(next.advance(&pointer).is_err());
    let mut drift = parsed;
    drift.snapshot.commit = "1111111".to_string();
    assert!(pointer.bind(&drift, text.as_bytes()).is_err());
}

#[test]
fn routes() {
    let held = derivative();
    assert_eq!(
        plumb::depot::v2::snapshots(&held.release, held.derivative, &held.snapshot.timestamp)
            .expect("snapshot path"),
        "v2/products/probe/derivatives/configuration/releases/stable/v1.2.3/snapshots/20260825T010203Z"
    );
    assert_eq!(
        plumb::depot::v2::latest("probe", plumb::depot::v2::Kind::Changelog, "stable")
            .expect("latest path"),
        "v2/products/probe/derivatives/changelog/channels/stable/latest.json"
    );
}

#[test]
fn strict() {
    let text = derivative().encode().expect("manifest");
    assert!(plumb::depot::v2::Manifest::parse(&text.replace("configuration", "artifact")).is_err());
    assert!(plumb::depot::v2::Manifest::parse(&text.replace(&"a".repeat(64), "abc")).is_err());
    assert!(
        plumb::depot::v2::Manifest::parse(&text.replace("20260825T010203Z", "latest")).is_err()
    );
}

#[test]
fn staged() {
    let root = tempfile::tempdir().expect("snapshot");
    let mut held = derivative();
    held.release.product = "plumb".to_string();
    std::fs::create_dir_all(root.path().join("rules")).expect("rules");
    std::fs::write(root.path().join("rules/probe.toml"), "answer = 42\n").expect("rule");
    std::fs::write(
        root.path().join(plumb::depot::v2::LEAF),
        held.encode().expect("manifest"),
    )
    .expect("manifest");
    let seat = plumb::depot::Rules::staged(root.path(), "v1.2.3").expect("exact seat");
    assert_eq!(
        seat.read("rules/probe.toml").expect("rule"),
        "answer = 42\n"
    );
    assert!(plumb::depot::Rules::staged(root.path(), "v1.2.4").is_err());
}
