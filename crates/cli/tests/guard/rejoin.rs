#[path = "../../src/shape/pair/identity.rs"]
mod identity;
use std::process::Command;

#[test]
fn settled() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    initialize(root);
    let base = commit(root, "base", None, &[]);
    let release = commit(root, "release", Some(&base), &[]);
    let rejoin = commit(root, "rejoin", Some(&base), &[&release]);
    run(root, ["reset", "--hard", &rejoin]);

    assert_eq!(identity::Seat(root).blind("plumb", Some(&release)), None);
}

#[test]
fn projection() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    initialize(root);
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n",
    )
    .expect("fixture manifest should be written");
    run(root, ["add", "Cargo.toml"]);
    let common = commit(root, "common", None, &[]);

    std::fs::write(root.join("source"), "changed").expect("fixture file should be written");
    run(root, ["add", "source"]);
    let main = commit(root, "main source", Some(&common), &[]);

    run(root, ["reset", "--hard", &common]);
    std::fs::write(root.join("source"), "changed").expect("fixture file should be written");
    run(root, ["add", "source"]);
    let picked = commit(root, "picked source", Some(&common), &[]);
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.2.0\"\n",
    )
    .expect("fixture manifest should be written");
    std::fs::create_dir_all(root.join(".plumb/releases/v0.2.0"))
        .expect("datum seat should be made");
    std::fs::write(
        root.join(".plumb/releases/v0.2.0/datum.toml"),
        "version = \"v0.2.0\"\n",
    )
    .expect("datum should be written");
    run(root, ["add", "Cargo.toml", ".plumb"]);
    let release = commit(root, "release", Some(&picked), &[]);

    run(root, ["reset", "--hard", &main]);
    let rejoin = commit(root, "rejoin", Some(&main), &[&release]);
    run(root, ["reset", "--hard", &rejoin]);

    assert_eq!(identity::Seat(root).blind("plumb", Some(&release)), None);
}

#[test]
fn advanced() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    initialize(root);
    let base = commit(root, "base", None, &[]);
    std::fs::write(root.join("source"), "changed").expect("fixture file should be written");
    run(root, ["add", "source"]);
    let head = commit(root, "head", Some(&base), &[]);
    run(root, ["reset", "--hard", &head]);

    assert!(identity::Seat(root).blind("plumb", Some(&base)).is_some());
}

#[test]
fn drifted() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    initialize(root);
    let base = commit(root, "base", None, &[]);
    let release = commit(root, "release", Some(&base), &[]);
    std::fs::write(root.join("source"), "drift").expect("fixture file should be written");
    run(root, ["add", "source"]);
    let drift = commit(root, "drift", Some(&base), &[]);
    let rejoin = commit(root, "rejoin", Some(&drift), &[&release]);
    run(root, ["reset", "--hard", &rejoin]);

    assert!(
        identity::Seat(root)
            .blind("plumb", Some(&release))
            .is_some()
    );
}

#[test]
fn source() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    initialize(root);
    let base = commit(root, "base", None, &[]);
    std::fs::write(root.join("source"), "release drift").expect("fixture file should be written");
    run(root, ["add", "source"]);
    let release = commit(root, "release", Some(&base), &[]);
    run(root, ["reset", "--hard", &base]);
    let rejoin = commit(root, "rejoin", Some(&base), &[&release]);
    run(root, ["reset", "--hard", &rejoin]);

    assert!(
        identity::Seat(root)
            .blind("plumb", Some(&release))
            .is_some()
    );
}

fn initialize(root: &std::path::Path) {
    run(root, ["init", "-q"]);
    run(root, ["config", "user.name", "Plumb"]);
    run(root, ["config", "user.email", "plumb@example.invalid"]);
    std::fs::write(root.join("source"), "base").expect("fixture file should be written");
    run(root, ["add", "source"]);
}

fn commit(root: &std::path::Path, message: &str, parent: Option<&str>, merges: &[&str]) -> String {
    let written = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("write-tree")
        .output()
        .expect("git should run");
    assert!(written.status.success(), "git write-tree should succeed");
    let tree = String::from_utf8_lossy(&written.stdout).trim().to_string();
    let mut command = Command::new("git");
    command.arg("-C").arg(root).args(["commit-tree", &tree]);
    if let Some(parent) = parent {
        command.args(["-p", parent]);
    }
    for merge in merges {
        command.args(["-p", merge]);
    }
    command.args(["-m", message]);
    let output = command.output().expect("git should run");
    assert!(output.status.success(), "git commit-tree should succeed");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn run<const N: usize>(root: &std::path::Path, args: [&str; N]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("git should run");
    assert!(output.status.success(), "git command should succeed");
}
