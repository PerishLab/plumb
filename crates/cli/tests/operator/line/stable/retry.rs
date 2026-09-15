use super::{Fixture, git};
use std::os::unix::fs::PermissionsExt as _;

#[test]
fn historical() {
    let fixture = Fixture::new();
    let home = super::support::depot(&[("help/version.txt", "Earlier version guidance.\n")]);
    let root = home.path().join("configurations");
    let pointer = root.join("metadata.json");
    let held: plumb::depot::Pointer =
        serde_json::from_slice(&std::fs::read(&pointer).unwrap()).unwrap();
    let previous = root.join(&held.version);
    let mut manifest: plumb::depot::Manifest =
        toml::from_str(&std::fs::read_to_string(previous.join("plumb.toml")).unwrap()).unwrap();
    manifest.metadata.version = "29990101T000001Z".into();
    let next = root.join(&manifest.metadata.version);
    std::fs::rename(previous, &next).unwrap();
    std::fs::write(next.join("plumb.toml"), manifest.encode().unwrap()).unwrap();
    std::fs::write(
        pointer,
        plumb::depot::Pointer::new(&manifest.metadata, "plumb")
            .encode()
            .unwrap(),
    )
    .unwrap();
    let first = fixture
        .command()
        .env("PLUMB_HOME", home.path())
        .output()
        .unwrap();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let head = fixture.remote();
    assert_eq!(
        plumb::guard::commit(fixture.root.path(), &head)
            .unwrap()
            .depot,
        "29990101T000001Z",
    );
    let repeated = fixture.command().output().unwrap();
    assert!(
        repeated.status.success(),
        "{}",
        String::from_utf8_lossy(&repeated.stderr)
    );
    assert!(String::from_utf8_lossy(&repeated.stdout).contains("nothing moved"));
    assert!(!String::from_utf8_lossy(&repeated.stderr).contains("guard guard/"));
    assert_eq!(fixture.remote(), head);
}

#[test]
fn unproved() {
    let fixture = Fixture::new();
    fixture.picked();
    git(
        fixture.root.path(),
        &["push", "origin", "HEAD:release/v1.0.0"],
    );
    fixture.refused("must carry exactly one Plumb-Guard-Proof:");
}

#[test]
fn push() {
    let fixture = Fixture::new();
    let hook = fixture.bare.path().join("hooks/pre-receive");
    std::fs::write(&hook, "#!/bin/sh\nwhile read old new ref; do\n  case \"$ref\" in refs/heads/release/*) exit 1;; esac\ndone\n").unwrap();
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
