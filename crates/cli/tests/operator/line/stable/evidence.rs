use super::{Fixture, git};
use std::process::Command;

#[test]
fn retained() {
    let mut fixture = Fixture::new();
    prepared(&mut fixture);
    fixture.accepted();
    let reference = format!("refs/plumb/picks/{}", fixture.source);
    assert_eq!(
        git(fixture.bare.path(), &["rev-parse", &reference]),
        fixture.source
    );
    let fresh = tempfile::tempdir().unwrap();
    let root = fresh.path().join("source");
    git(
        fresh.path(),
        &[
            "clone",
            "--no-local",
            "--single-branch",
            "--branch",
            "release/v1.0.0",
            &format!("file://{}", fixture.bare.path().display()),
            root.to_str().unwrap(),
        ],
    );
    let absent = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["cat-file", "-e", &fixture.source])
        .output()
        .unwrap();
    assert!(!absent.status.success());
    let head = git(&root, &["rev-parse", "HEAD"]);
    git(
        &root,
        &[
            "fetch",
            "origin",
            "refs/heads/main:refs/remotes/origin/main",
        ],
    );
    let cut = fresh.path().join("cut");
    std::fs::write(&cut, &head).unwrap();
    let (forge, _) = super::super::super::world::serve(
        super::super::super::world::Court::Freeze(cut.clone()),
        2,
    );
    git(&root, &["config", "plumb.test-forgejo-url", &forge]);
    let frozen = super::super::super::command::plumb(
        &root,
        &["version", "freeze", "--version", "v1.0.0", "--dry-run"],
    )
    .env("PLUMB_HOME", fixture.home.path())
    .env_remove("PLUMB_GUARD_CONFIGURATION")
    .output()
    .unwrap();
    assert!(
        !frozen.status.success()
            && String::from_utf8_lossy(&frozen.stderr).contains("no published exact seal"),
        "{}",
        String::from_utf8_lossy(&frozen.stderr)
    );
    let repeated = fixture.command().current_dir(&root).output().unwrap();
    assert!(
        repeated.status.success(),
        "{}",
        String::from_utf8_lossy(&repeated.stderr)
    );
    assert_eq!(git(&root, &["rev-parse", "HEAD"]), head);
    assert_eq!(git(&root, &["rev-parse", "FETCH_HEAD"]), fixture.source);
    assert!(String::from_utf8_lossy(&repeated.stdout).contains("nothing moved"));
    git(fixture.bare.path(), &["update-ref", "-d", &reference]);
    let (forge, _) =
        super::super::super::world::serve(super::super::super::world::Court::Freeze(cut), 2);
    git(&root, &["config", "plumb.test-forgejo-url", &forge]);
    let missing = super::super::super::command::plumb(
        &root,
        &["version", "freeze", "--version", "v1.0.0", "--dry-run"],
    )
    .env("PLUMB_HOME", fixture.home.path())
    .env_remove("PLUMB_GUARD_CONFIGURATION")
    .output()
    .unwrap();
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("no retained remote evidence"));
}

fn prepared(fixture: &mut Fixture) {
    let root = fixture.root.path();
    git(
        root,
        &[
            "remote",
            "set-url",
            "origin",
            &format!("file://{}", fixture.bare.path().display()),
        ],
    );
    git(root, &["push", "origin", "HEAD:refs/heads/main"]);
    let cut = fixture.home.path().join("cut");
    std::fs::write(&cut, &fixture.base).unwrap();
    let (forge, _) =
        super::super::super::world::serve(super::super::super::world::Court::Prepare(true, cut), 8);
    git(root, &["config", "plumb.test-forgejo-url", &forge]);
    let output =
        super::super::super::command::plumb(root, &["version", "prepare", "--version", "v1.0.0"])
            .env("PLUMB_HOME", fixture.home.path())
            .env_remove("PLUMB_GUARD_CONFIGURATION")
            .output()
            .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    git(root, &["pull", "--ff-only", "origin", "release/v1.0.0"]);
    fixture.base = git(root, &["rev-parse", "HEAD"]);
}

#[test]
fn collision() {
    let fixture = Fixture::new();
    let reference = format!("refs/plumb/picks/{}", fixture.source);
    git(
        fixture.bare.path(),
        &["update-ref", &reference, &fixture.base],
    );
    let output = fixture.command().output().unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("pick source evidence"), "{error}");
    assert!(error.contains("disagrees"), "{error}");
    assert_eq!(fixture.remote(), fixture.base);
    assert_eq!(
        git(fixture.bare.path(), &["rev-parse", &reference]),
        fixture.base
    );
}

#[test]
fn backfill() {
    let fixture = Fixture::new();
    fixture.accepted();
    let reference = format!("refs/plumb/picks/{}", fixture.source);
    git(fixture.bare.path(), &["update-ref", "-d", &reference]);
    let before = fixture.remote();
    let output = fixture.command().output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fixture.remote(), before);
    assert_eq!(
        git(fixture.bare.path(), &["rev-parse", &reference]),
        fixture.source
    );
}
