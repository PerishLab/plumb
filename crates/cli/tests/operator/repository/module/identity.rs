use super::projection::Workload;
use flate2::{Compression, GzBuilder, read::GzDecoder};
use semver::Version;
use std::collections::BTreeMap;
use std::io::Read;

fn repack(
    bytes: &[u8],
    output: &std::path::Path,
    package: &str,
    version: &Version,
) -> Result<(), String> {
    Workload::new(bytes).bind(output, package, version)
}

fn archive(duplicate: bool) -> Vec<u8> {
    let mut archive = tar::Builder::new(
        GzBuilder::new()
            .mtime(0)
            .write(Vec::new(), Compression::default()),
    );
    let manifest = br#"{"name":"held","version":"0.0.0","main":"index.js"}"#;
    let mut files: Vec<(&str, &[u8])> = vec![
        ("package/package.json", manifest),
        ("package/index.js", b"export const held = 1;"),
    ];
    if duplicate {
        files.push(("package/package.json", manifest));
    }
    for (name, body) in files {
        let mut header = tar::Header::new_gnu();
        header.set_path(name).unwrap();
        header.set_size(body.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        archive.append(&header, body).unwrap();
    }
    archive.into_inner().unwrap().finish().unwrap()
}

fn files(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut archive = tar::Archive::new(GzDecoder::new(bytes));
    archive
        .entries()
        .unwrap()
        .map(|entry| {
            let mut entry = entry.unwrap();
            let path = entry.path().unwrap().to_string_lossy().into_owned();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            (path, bytes)
        })
        .collect()
}

#[test]
fn binding() {
    let seat = tempfile::tempdir().unwrap();
    let original = archive(false);
    let mut previous = None;
    for version in ["1.0.0-beta.1", "1.0.0-beta.2"] {
        let output = seat.path().join(version);
        repack(
            &original,
            &output,
            "held",
            &Version::parse(version).unwrap(),
        )
        .unwrap();
        let bytes = std::fs::read(output).unwrap();
        let held = files(&bytes);
        assert_eq!(
            held["package/index.js"],
            files(&original)["package/index.js"]
        );
        let manifest: serde_json::Value =
            serde_json::from_slice(&held["package/package.json"]).unwrap();
        assert_eq!(manifest["version"], version);
        if let Some(previous) = previous {
            assert_ne!(bytes, previous);
        }
        previous = Some(bytes);
    }
}

#[test]
fn idempotent() {
    let seat = tempfile::tempdir().unwrap();
    let output = seat.path().join("package.tgz");
    let version = Version::parse("1.0.0-beta.1").unwrap();
    repack(&archive(false), &output, "held", &version).unwrap();
    let before = std::fs::read(&output).unwrap();
    repack(&before, &output, "held", &version).unwrap();
    assert_eq!(std::fs::read(&output).unwrap(), before);
}

#[test]
fn ownership() {
    let seat = tempfile::tempdir().unwrap();
    let output = seat.path().join("package.tgz");
    assert!(repack(&archive(false), &output, "another", &Version::new(0, 0, 0)).is_err());
    assert!(!output.exists());
}

#[test]
fn duplicate() {
    let seat = tempfile::tempdir().unwrap();
    let output = seat.path().join("package.tgz");
    assert!(repack(&archive(true), &output, "held", &Version::new(0, 0, 0)).is_err());
    assert!(!output.exists());
}
