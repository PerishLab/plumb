use super::{Fixture, git};
use std::os::unix::fs::PermissionsExt as _;

#[test]
fn push() {
    let fixture = Fixture::new();
    let hook = fixture.bare.path().join("hooks/pre-receive");
    std::fs::write(&hook, "#!/bin/sh\nexit 1\n").unwrap();
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
    let output = fixture.command().output().unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("push release line failed"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fixture.remote(), fixture.base);
    assert_ne!(
        git(fixture.root.path(), &["rev-parse", "HEAD"]),
        fixture.base
    );
    plumb::guard::commit(fixture.root.path(), "HEAD").unwrap();
    std::fs::remove_file(hook).unwrap();
    fixture.accepted();
}

#[test]
fn batch() {
    let fixture = Fixture::new();
    git(fixture.root.path(), &["switch", "-q", "source"]);
    std::fs::write(
        fixture.root.path().join("src/lib.rs"),
        "pub fn value() -> u8 {\n    2\n}\n",
    )
    .unwrap();
    git(fixture.root.path(), &["commit", "-qam", "second source"]);
    let second = git(fixture.root.path(), &["rev-parse", "HEAD"]);
    git(fixture.root.path(), &["switch", "-q", "release/v1.0.0"]);
    git(
        fixture.root.path(),
        &["cherry-pick", "-x", &fixture.source, &second],
    );
    let command = || {
        fixture
            .command()
            .args(["--commit", &second])
            .output()
            .unwrap()
    };
    let first = command();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let head = fixture.remote();
    assert_eq!(
        git(fixture.root.path(), &["rev-parse", "HEAD^^"]),
        fixture.base
    );
    let repeated = command();
    assert!(
        repeated.status.success(),
        "{}",
        String::from_utf8_lossy(&repeated.stderr)
    );
    assert!(String::from_utf8_lossy(&repeated.stdout).contains("nothing moved"));
    assert_eq!(fixture.remote(), head);
}

#[test]
fn raced() {
    let fixture = Fixture::new();
    git(
        fixture.bare.path(),
        &[
            "fetch",
            "-q",
            fixture.root.path().to_str().unwrap(),
            "source",
        ],
    );
    let hook = fixture.root.path().join(".git/hooks/pre-push");
    std::fs::write(&hook, "#!/bin/sh\nexec git -C \"$PICK_REMOTE\" update-ref refs/heads/release/v1.0.0 \"$PICK_SOURCE\"\n").unwrap();
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
    let output = fixture
        .command()
        .env("PICK_REMOTE", fixture.bare.path())
        .env("PICK_SOURCE", &fixture.source)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("push release line failed"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fixture.remote(), fixture.source);
    assert_ne!(
        git(fixture.root.path(), &["rev-parse", "HEAD"]),
        fixture.base
    );
}
