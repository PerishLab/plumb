use super::{Fixture, command};
use std::path::Path;

pub fn before(fixture: &Fixture<'_>, path: &Path, generation: &str, expected: &str) {
    let original = std::fs::read(path).unwrap();
    let unknown = command(fixture)
        .env("FAKE_DEPOT_UNKNOWN", "true")
        .args(["--promote", generation, "--expect", expected])
        .output()
        .unwrap();
    assert!(!unknown.status.success());
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("Connection broken"));
    assert_eq!(std::fs::read(path).unwrap(), original);
    let mut other = plumb::depot::v3::Pointer::parse(&original).unwrap();
    other.created = "2026-09-16T00:00:00Z".into();
    let bytes = other.encode().unwrap();
    let race = fixture.root.join("raced.json");
    std::fs::write(&race, &bytes).unwrap();
    let raced = command(fixture)
        .env("FAKE_DEPOT_RACE", &race)
        .args(["--promote", generation, "--expect", expected])
        .output()
        .unwrap();
    assert!(!raced.status.success());
    assert!(String::from_utf8_lossy(&raced.stderr).contains("changed while advancing"));
    assert_eq!(std::fs::read(path).unwrap(), bytes);
    std::fs::write(path, original).unwrap();
    let base = path.parent().unwrap().join("generations");
    let manifest = base.join(generation).join("manifest.json");
    let mut manifest =
        plumb::depot::v3::Manifest::parse(&std::fs::read(manifest).unwrap()).unwrap();
    manifest.marker.sha256 = "c".repeat(64);
    let foreign = manifest.generation().unwrap();
    std::fs::create_dir(base.join(&foreign)).unwrap();
    std::fs::write(
        base.join(&foreign).join("manifest.json"),
        manifest.encode().unwrap(),
    )
    .unwrap();
    let wrong = command(fixture)
        .args(["--promote", &foreign, "--expect", expected])
        .output()
        .unwrap();
    assert!(!wrong.status.success());
    assert!(
        String::from_utf8_lossy(&wrong.stderr)
            .contains("does not bind the selected release marker")
    );
    assert_eq!(plumb::depot::sha(&std::fs::read(path).unwrap()), expected);
}
