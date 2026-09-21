#[test]
fn hookless() {
    let fixture = super::fixture();
    let root = fixture.path();
    std::fs::write(root.join("plumb.toml"), "[layout]\n").expect("governance");
    std::fs::remove_file(root.join(".git/hooks/pre-commit")).expect("remove projected hook");
    let held = super::run(&["doctor", root.to_str().expect("path should be utf8")]);
    assert!(held.contains("pre-commit is absent"), "{held}");
    assert!(held.contains("run plumb configuration install"), "{held}");
}
