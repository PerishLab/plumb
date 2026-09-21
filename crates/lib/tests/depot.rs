#[path = "support/generation.rs"]
mod generation;

#[test]
fn skill() {
    let root = tempfile::tempdir().expect("skill seat");
    std::fs::create_dir_all(root.path().join("home/.claude/skills")).expect("agent seat");
    let kit = plumb::skill::Kit {
        name: "probe".to_string(),
        home: root.path().join("home"),
        state: root.path().join("state.json"),
        url: "https://releases.example.test".to_string(),
    };
    let source = kit.depot("https://depot.example.test", "probe", "v1.2.3");
    let held = source.install(&plumb::skill::Ask {
        channel: "stable".to_string(),
        version: Some("v1.2.2".to_string()),
        ..plumb::skill::Ask::default()
    });
    assert!(matches!(held, Err(plumb::skill::Error::Version(_))));
}
