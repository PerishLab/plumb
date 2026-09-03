use super::super::world::Fixture;

#[test]
fn proof() {
    let temp = tempfile::tempdir().expect("temp root");
    let bare = tempfile::tempdir().expect("bare root");
    let tools = temp.path().join("tools");
    std::fs::create_dir(&tools).expect("tool root");
    let fixture = Fixture {
        root: temp.path(),
        tools: &tools,
    };
    super::marker::seeded(&fixture, bare.path());
    let dry = fixture
        .command()
        .current_dir(fixture.root)
        .args([
            "release",
            "stamp",
            "--version",
            "v1.2.0-beta.1",
            "--dry-run",
        ])
        .output()
        .expect("dry release stamp");
    assert!(!dry.status.success());
    assert!(
        String::from_utf8_lossy(&dry.stderr).contains("unproved tree"),
        "{}",
        String::from_utf8_lossy(&dry.stderr)
    );
}
