use super::datum::lined;
use super::forgejo::{Court, serve};
use super::stable::{command, run};
use std::process::Command;

#[test]
fn late() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    let root = fixture.path();
    let cut = root.join("cut");
    let (url, _) = serve(Court::Prepare(true, cut.clone()), 7);
    let origin = format!("{url}/test/probe.git");
    let head = lined(root, &origin, bare.path(), "release/v1.3.0");
    std::fs::write(&cut, &head).expect("cut");

    std::fs::write(root.join("stray"), "released, never rejoined\n").expect("stray");
    run(Command::new("git").args(["add", "-A"]).current_dir(root));
    run(Command::new("git")
        .args([
            "commit",
            "-q",
            "-m",
            "Carry work only the release line holds",
        ])
        .current_dir(root));
    run(Command::new("git")
        .args(["tag", "v1.2.0", "HEAD"])
        .current_dir(root));
    run(Command::new("git")
        .args(["update-ref", "refs/remotes/origin/main", &head])
        .current_dir(root));

    let output = command(root, &["stable", "prepare", "--version", "1.3.0"]);
    assert!(!output.status.success());
    let said = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(said.contains("stable v1.2.0 stands at"), "{said}");
    assert!(
        said.contains("plumb stable rejoin --version v1.2.0"),
        "{said}"
    );
}

#[test]
fn migrated() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    let root = fixture.path();
    let cut = root.join("cut");
    let (url, _) = serve(Court::Prepare(true, cut.clone()), 9);
    let origin = format!("{url}/test/probe.git");
    std::fs::create_dir_all(root.join(".plumb/releases/v1.2.0")).expect("seat");
    std::fs::write(
        root.join(".plumb/releases/v1.2.0/datum.json"),
        "{\"schema\":1}\n",
    )
    .expect("an older Plumb's datum");
    let head = lined(root, &origin, bare.path(), "release/v1.2.0");
    std::fs::write(&cut, &head).expect("cut");
    let output = command(root, &["stable", "prepare", "--version", "1.2.0"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let listed = Command::new("git")
        .args(["log", "--format=%H", "release/v1.2.0"])
        .current_dir(bare.path())
        .output()
        .expect("git");
    let recorded = String::from_utf8_lossy(&listed.stdout)
        .lines()
        .next()
        .expect("datum commit")
        .to_string();
    let shown = Command::new("git")
        .args(["show", "--name-only", "--format=", &recorded])
        .current_dir(bare.path())
        .output()
        .expect("git");
    let touched = String::from_utf8_lossy(&shown.stdout).to_string();
    assert!(touched.contains("datum.toml"), "{touched}");
    assert!(
        touched.contains("datum.json"),
        "the recording commit drops what it swept, so freeze sees one seat commit: {touched}"
    );
}
