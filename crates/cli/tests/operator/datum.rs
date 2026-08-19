use super::forgejo::{Court, serve};
use super::stable::{command, repo, run};
use std::path::Path;
use std::process::Command;

pub fn lined(root: &Path, origin: &str, bare: &Path, line: &str) -> String {
    repo(root, origin);
    run(Command::new("git").args(["init", "-q", "--bare"]).arg(bare));
    let stand = format!("url.{}.pushInsteadOf", bare.display());
    run(Command::new("git")
        .args(["config", &stand, origin])
        .current_dir(root));
    run(Command::new("git")
        .args(["config", "user.email", "probe@test"])
        .current_dir(root));
    run(Command::new("git")
        .args(["config", "user.name", "probe"])
        .current_dir(root));
    run(Command::new("git").args(["add", "-A"]).current_dir(root));
    run(Command::new("git")
        .args(["commit", "-q", "-m", "Stand the probe up"])
        .current_dir(root));
    run(Command::new("git")
        .args(["push", "-q", "origin", &format!("HEAD:refs/heads/{line}")])
        .current_dir(root));
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .expect("git");
    let head = String::from_utf8_lossy(&head.stdout).trim().to_string();
    run(Command::new("git")
        .args(["update-ref", "refs/remotes/origin/main", &head])
        .current_dir(root));
    head
}

#[test]
fn protection() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    let cut = fixture.path().join("cut");
    let (url, _) = serve(Court::Prepare(true, cut.clone()), 7);
    let origin = format!("{url}/test/probe.git");
    let head = lined(fixture.path(), &origin, bare.path(), "release/v1.2.0");
    std::fs::write(&cut, &head).expect("cut");
    let output = command(
        fixture.path(),
        &["release", "prepare", "--version", "1.2.0"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stood = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(
        stood.contains("prepared release/v1.2.0 from main"),
        "{stood}"
    );
    assert!(
        stood.contains(".plumb/releases/v1.2.0/datum.toml"),
        "{stood}"
    );
    let shown = Command::new("git")
        .args(["show", "release/v1.2.0:.plumb/releases/v1.2.0/datum.toml"])
        .current_dir(bare.path())
        .output()
        .expect("git");
    assert!(
        shown.status.success(),
        "{}",
        String::from_utf8_lossy(&shown.stderr)
    );
    let datum = String::from_utf8_lossy(&shown.stdout).to_string();
    assert!(datum.contains("schema = 1"), "{datum}");
    assert!(datum.contains("version = \"v1.2.0\""), "{datum}");
}

#[test]
fn sweeps() {
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
    .expect("stale datum");
    let head = lined(root, &origin, bare.path(), "release/v1.2.0");
    std::fs::write(&cut, &head).expect("cut");

    let output = command(root, &["release", "prepare", "--version", "1.2.0"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("recorded"),
        "a seat holding a stray leaf is not already recorded"
    );
    let listed = Command::new("git")
        .args([
            "ls-tree",
            "-r",
            "--name-only",
            "release/v1.2.0",
            "--",
            ".plumb/releases/v1.2.0",
        ])
        .current_dir(bare.path())
        .output()
        .expect("git");
    let listed = String::from_utf8_lossy(&listed.stdout).to_string();
    assert!(listed.contains("datum.toml"), "{listed}");
    assert!(
        !listed.contains("datum.json"),
        "the datum commit owns its seat: {listed}"
    );
}
