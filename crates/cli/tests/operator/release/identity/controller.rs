use super::super::world::{Fixture, run};
use std::process::Command;

#[test]
fn local() {
    let temp = tempfile::tempdir().expect("temp root");
    let bare = tempfile::tempdir().expect("bare root");
    let blind = tempfile::tempdir().expect("blind depot");
    let tools = temp.path().join("tools");
    std::fs::create_dir(&tools).expect("tool root");
    let fixture = Fixture {
        root: temp.path(),
        tools: &tools,
    };
    super::marker::seeded(&fixture, bare.path());
    let manifest = std::fs::read_to_string(fixture.root.join("plumb.toml")).expect("manifest");
    std::fs::write(
        fixture.root.join("plumb.toml"),
        manifest.replace("product = \"probe\"", "product = \"plumb\""),
    )
    .expect("plumb controller");
    run(Command::new("git")
        .arg("-C")
        .arg(fixture.root)
        .args(["commit", "-qam", "identify plumb"]));
    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "push",
        "-q",
        "origin",
        "HEAD:refs/heads/release/v1.2.0",
    ]));
    let output = fixture
        .command()
        .current_dir(fixture.root)
        .env("PLUMB_HOME", blind.path())
        .args([
            "release",
            "stamp",
            "--version",
            "v1.2.0-beta.1",
            "--dry-run",
        ])
        .output()
        .expect("release stamp");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("git tag -a v1.2.0-beta.1"));

    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "tag",
        "-a",
        "v1.2.0-beta.1",
        "-m",
        "plumb v1.2.0-beta.1",
    ]));
    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "push",
        "-q",
        "origin",
        "refs/tags/v1.2.0-beta.1",
    ]));
    let verified = fixture
        .command()
        .current_dir(fixture.root)
        .env("PLUMB_HOME", blind.path())
        .args(["release", "verify", "--marker", "v1.2.0-beta.1", "--held"])
        .output()
        .expect("release verify");
    assert!(!verified.status.success());
    let error = String::from_utf8_lossy(&verified.stderr);
    assert!(!error.contains("plumb depot seat is unreadable"), "{error}");
    assert!(error.contains("has no valid guard proof"), "{error}");
}
