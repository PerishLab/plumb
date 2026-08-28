use super::super::world::{Fixture, run};
use super::marker::{seeded, stamp};
use std::process::Command;

#[test]
fn snapshot() {
    let temp = tempfile::tempdir().expect("temp root");
    let bare = tempfile::tempdir().expect("bare root");
    let tools = temp.path().join("tools");
    std::fs::create_dir(&tools).expect("tool root");
    let fixture = Fixture {
        root: temp.path(),
        tools: &tools,
    };
    let commit = seeded(&fixture, bare.path());
    stamp(temp.path(), "v1.2.0-beta.1", &commit, true);
    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "remote",
        "set-url",
        "origin",
        "file:///missing/PerishLab/probe.git",
    ]));

    let held = fixture
        .command()
        .current_dir(fixture.root)
        .args([
            "release",
            "marker",
            "show",
            "--marker",
            "v1.2.0-beta.1",
            "--held",
        ])
        .output()
        .expect("plumb should run");
    assert!(
        held.status.success(),
        "a held snapshot does not consult its remote: {}",
        String::from_utf8_lossy(&held.stderr)
    );
    let refreshed = fixture
        .command()
        .current_dir(fixture.root)
        .args(["release", "marker", "show", "--marker", "v1.2.0-beta.1"])
        .output()
        .expect("plumb should run");
    assert!(!refreshed.status.success());
}
