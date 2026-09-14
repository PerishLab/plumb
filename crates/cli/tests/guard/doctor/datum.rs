use std::path::Path;
use std::process::Command;

fn line(root: &Path, name: &str) {
    let status = Command::new("git")
        .args(["symbolic-ref", "HEAD", &format!("refs/heads/{name}")])
        .current_dir(root)
        .status()
        .expect("git should run");
    assert!(status.success(), "fixture should stand on {name}");
}

fn record(root: &Path, version: &str, text: &str) {
    let seat = root.join(".plumb/releases").join(version);
    std::fs::create_dir_all(&seat).expect("seat should be made");
    std::fs::write(seat.join("datum.toml"), text).expect("datum should be written");
}

#[test]
fn absent() {
    let fixture = super::fixture();
    let root = fixture.path();
    line(root, "release/v1.2.0");
    let out = super::run(&["doctor", root.to_str().expect("path should be utf8")]);
    assert!(
        out.contains("release line v1.2.0 records no datum to judge against"),
        "{out}"
    );
}

#[test]
fn recorded() {
    let fixture = super::fixture();
    let root = fixture.path();
    line(root, "release/v1.2.0");
    record(root, "v1.2.0", "schema = 1\nversion = \"v1.2.0\"\n");
    let out = super::run(&["doctor", root.to_str().expect("path should be utf8")]);
    assert!(!out.contains("records no datum"), "{out}");
    assert!(out.contains("true to the skeleton"), "{out}");
}

#[test]
fn disagrees() {
    let fixture = super::fixture();
    let root = fixture.path();
    line(root, "release/v1.2.0");
    record(root, "v1.2.0", "schema = 1\nversion = \"v1.3.0\"\n");
    let out = super::run(&["doctor", root.to_str().expect("path should be utf8")]);
    assert!(out.contains("names release v1.3.0, not v1.2.0"), "{out}");
}

#[test]
fn ambient() {
    let fixture = super::fixture();
    let root = fixture.path();
    line(root, "release/v1.2.0");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", root.to_str().expect("path should be utf8")])
        .env("PLUMB_RELEASE_VERSION", "release/v9.9.9")
        .output()
        .expect("plumb should run");
    let said = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(
        said.contains("release line v9.9.9 records no datum"),
        "a declared release version outranks the branch it runs on: {said}"
    );
}

#[test]
fn ordinary() {
    let fixture = super::fixture();
    let root = fixture.path();
    let out = super::run(&["doctor", root.to_str().expect("path should be utf8")]);
    assert!(!out.contains("records no datum"), "{out}");
}

#[test]
fn marker() {
    let fixture = super::fixture();
    let home = super::super::support::depot(&[]);
    let root = fixture.path();
    record(root, "v1.2.0", "schema = 1\nversion = \"v1.2.0\"\n");
    for version in ["v1.2.0-beta.1", "v1.2.0", "v1.3.0-beta.1"] {
        let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args(["doctor", root.to_str().unwrap()])
            .env("PLUMB_HOME", home.path())
            .env("PLUMB_RELEASE_VERSION", version)
            .output()
            .unwrap();
        let said = String::from_utf8_lossy(&output.stdout);
        if version.starts_with("v1.2.0") {
            assert!(output.status.success(), "{said}");
            assert!(!said.contains("records no datum"), "{said}");
        } else {
            assert!(!output.status.success(), "{said}");
            assert!(
                said.contains("release line v1.3.0 records no datum"),
                "{said}"
            );
        }
    }
}
