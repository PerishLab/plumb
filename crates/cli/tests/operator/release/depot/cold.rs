use super::super::super::world::run;
use plumb::depot::v3::Pointer;
use std::path::Path;
use std::process::Command;

#[path = "world.rs"]
mod world;
use world::{MARKER, World};

fn git(root: &Path, args: &[&str]) -> String {
    String::from_utf8(run(Command::new("git").arg("-C").arg(root).args(args)).stdout)
        .unwrap()
        .trim()
        .to_string()
}

#[test]
fn empty() {
    let world = World::new();
    let fresh = tempfile::tempdir().unwrap();
    let old = std::fs::read(world.home.path().join("configurations/latest.json")).unwrap();
    let original = git(world.bare.path(), &["rev-parse", MARKER]);
    run(&mut world.install(fresh.path()));
    assert_eq!(git(world.bare.path(), &["rev-parse", MARKER]), original);
    let pointer =
        Pointer::parse(&std::fs::read(fresh.path().join("configurations/latest.json")).unwrap())
            .unwrap();
    assert_eq!(
        pointer.generation,
        world.bundle.manifest.generation().unwrap()
    );
    assert_eq!(pointer.marker, world.bundle.manifest.marker);
    assert_eq!(
        std::fs::read(world.home.path().join("configurations/latest.json")).unwrap(),
        old
    );
    assert!(!world.root.path().join("plumb.toml").exists());
    assert!(!world.root.path().join(".git/hooks/pre-commit").exists());
    let repeated = world.install(fresh.path()).output().unwrap();
    assert!(!repeated.status.success());
    assert!(String::from_utf8_lossy(&repeated.stderr).contains("new isolated --path"));
}

#[test]
fn drift() {
    let mut world = World::new();
    world.bundle.manifest.marker.sha256 = "b".repeat(64);
    world.publish();
    let fresh = tempfile::tempdir().unwrap();
    let output = world.install(fresh.path()).output().unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("does not bind the selected release marker")
    );
    assert!(!fresh.path().join("configurations").exists());
}

#[test]
fn corrupted() {
    let mut world = World::new();
    world
        .bundle
        .bodies
        .insert("rules/products.toml".into(), b"corrupted".to_vec());
    world.publish();
    let fresh = tempfile::tempdir().unwrap();
    let output = world.install(fresh.path()).output().unwrap();
    assert!(!output.status.success());
    assert!(!fresh.path().join("configurations").exists());
}

#[test]
fn bridge() {
    let world = World::new();
    let seat = tempfile::tempdir().unwrap();
    let scripts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.forgejo/scripts");
    let binding = serde_json::json!({"marker":world.bundle.manifest.marker,
        "generation":world.bundle.manifest.generation().unwrap()});
    run(Command::new("python3")
        .arg(scripts.join("tests/acceptance.py"))
        .args([
            "--binary",
            env!("CARGO_BIN_EXE_plumb"),
            "--binding",
            &binding.to_string(),
            "--source",
            &world.source,
        ])
        .arg("--root")
        .arg(world.root.path())
        .arg("--seat")
        .arg(seat.path()));
}

#[test]
fn bound() {
    let world = World::bound();
    let fresh = tempfile::tempdir().unwrap();
    run(&mut world.install(fresh.path()));
    let previous = Pointer::parse(
        &std::fs::read(world.home.path().join("configurations/latest.json")).unwrap(),
    )
    .unwrap();
    let route = plumb::depot::v3::Route::new(&previous.channel, previous.kind, &previous.version);
    let path = world
        .root
        .path()
        .join("depot")
        .join(plumb::depot::v3::generation(route, &previous.generation).unwrap())
        .join("manifest.json");
    std::fs::remove_file(path).unwrap();
    let another = tempfile::tempdir().unwrap();
    let output = world.install(another.path()).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("has no manifest"));
    assert!(!another.path().join("configurations").exists());
}
