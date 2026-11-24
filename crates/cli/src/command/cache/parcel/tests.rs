use super::{Parcel, Plan};
use std::path::Path;

fn put(root: &Path, name: &str, bytes: &[u8]) {
    let path = root.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn timestamps() {
    let root = tempfile::tempdir().unwrap();
    let cargo = root.path().join("cargo");
    put(&cargo, "debug/deps/material", b"compiled inputs");
    put(&cargo, "lease", b"must not travel");
    put(&cargo, "plumb.json", b"local ownership");
    put(&cargo, "tmp/output", b"test output, not compilation");
    let input = cargo.join("debug/deps/material");
    let stamp = std::time::UNIX_EPOCH + std::time::Duration::new(123456, 987654321);
    std::fs::File::options()
        .read(true)
        .write(true)
        .open(&input)
        .unwrap()
        .set_modified(stamp)
        .unwrap();
    let original = std::fs::metadata(&input).unwrap().modified().unwrap();
    let path = root.path().join("material.tar");
    let receipt = Parcel(&path)
        .write(Plan {
            identity: "same world;producer-a".into(),
            cargo: Some(cargo),
            compiler: None,
            timing: None,
        })
        .unwrap();
    assert!(receipt.cargo);
    let snapshot = Parcel(&path).read("same world;").unwrap().unwrap();
    let restored = snapshot.root.path().join("cargo/debug/deps/material");
    assert_eq!(std::fs::read(&restored).unwrap(), b"compiled inputs");
    assert_eq!(
        std::fs::metadata(restored).unwrap().modified().unwrap(),
        original
    );
    assert!(!snapshot.root.path().join("cargo/lease").exists());
    assert!(!snapshot.root.path().join("cargo/plumb.json").exists());
    assert!(!snapshot.root.path().join("cargo/tmp").exists());
    assert_eq!(snapshot.manifest.identity, "same world;producer-a");
    assert!(Parcel(&path).read("different world;").is_err());
    assert!(Parcel(&path).read("same world;producer-b").is_err());
}

#[test]
fn empty() {
    let root = tempfile::tempdir().unwrap();
    let compiler = root.path().join("compiler");
    std::fs::create_dir_all(compiler.join("a/b")).unwrap();
    let path = root.path().join("material.tar");
    let receipt = Parcel(&path)
        .write(Plan {
            identity: "empty".into(),
            cargo: None,
            compiler: Some(compiler),
            timing: None,
        })
        .unwrap();
    assert!(!receipt.any());
    assert!(
        !path.exists(),
        "an empty backend cannot claim an immutable cache key"
    );
}

#[test]
fn traversal() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("material.tar");
    let encoder = zstd::Encoder::new(std::fs::File::create(&path).unwrap(), 1).unwrap();
    let mut builder = tar::Builder::new(encoder);
    let mut header = tar::Header::new_gnu();
    header.as_mut_bytes()[..10].copy_from_slice(b"../escaped");
    header.set_mode(0o644);
    header.set_size(1);
    header.set_cksum();
    builder.append(&header, b"x".as_slice()).unwrap();
    builder.into_inner().unwrap().finish().unwrap();
    let error = Parcel(&path).read("world").err().unwrap();
    assert!(error.contains("relative"), "{error}");
    assert!(!root.path().join("escaped").exists());
}

#[test]
fn timing() {
    let root = tempfile::tempdir().unwrap();
    let compiler = root.path().join("compiler");
    put(&compiler, "entry", b"compiled");
    let timing = root.path().join("timing");
    put(&timing, "record.json", b"timings");
    let path = root.path().join("material.tar");
    let receipt = Parcel(&path)
        .write(Plan {
            identity: "world;producer".into(),
            cargo: None,
            compiler: Some(compiler),
            timing: Some(timing.clone()),
        })
        .unwrap();
    assert!(receipt.compiler);
    let snapshot = Parcel(&path).read("world;").unwrap().unwrap();
    assert!(snapshot.manifest.timing);
    assert_eq!(
        std::fs::read(snapshot.root.path().join("timing/record.json")).unwrap(),
        b"timings"
    );
    let alone = root.path().join("alone.tar");
    let receipt = Parcel(&alone)
        .write(Plan {
            identity: "world;producer".into(),
            cargo: None,
            compiler: None,
            timing: Some(timing),
        })
        .unwrap();
    assert!(!receipt.any());
    assert!(!alone.exists(), "timings alone cannot claim a cache key");
}

#[test]
fn legacy() {
    let manifest: super::Manifest = serde_json::from_str(
        r#"{"schema":"plumb.guard-parcel/v1","identity":"world;","cargo":false,"compiler":true,"times":{}}"#,
    )
    .unwrap();
    assert!(manifest.compiler);
    assert!(!manifest.timing);
}
