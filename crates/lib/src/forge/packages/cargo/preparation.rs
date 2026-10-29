use std::fs;

#[test]
fn fetch() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("src")).unwrap();
    let manifest = "[package]\nname='fixture'\nversion='0.1.0'\nedition='2024'\n[workspace]\n";
    let lock = "version=4\n[[package]]\nname='fixture'\nversion='0.1.0'\n";
    fs::write(root.path().join("Cargo.toml"), manifest).unwrap();
    fs::write(root.path().join("Cargo.lock"), lock).unwrap();
    fs::write(root.path().join("src/lib.rs"), "pub fn value() {}\n").unwrap();
    fs::write(
        root.path().join("build.rs"),
        "fn main() { std::fs::write(\"built\", b\"ran\").unwrap(); }\n",
    )
    .unwrap();
    super::fetch(root.path()).unwrap();
    assert_eq!(
        fs::read_to_string(root.path().join("Cargo.toml")).unwrap(),
        manifest
    );
    assert_eq!(
        fs::read_to_string(root.path().join("Cargo.lock")).unwrap(),
        lock
    );
    assert!(!root.path().join("built").exists());
    assert!(!root.path().join("target").exists());
}
