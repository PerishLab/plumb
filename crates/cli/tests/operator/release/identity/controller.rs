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
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(!error.contains("plumb depot seat is unreadable"), "{error}");
    assert!(error.contains("cannot resolve release head"), "{error}");
}
