use super::Repo;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub(super) fn fixture() -> tempfile::TempDir {
    let held = tempfile::tempdir().expect("repository");
    let root = held.path();
    Repo::git(root, &["init", "-q"]);
    Repo::git(root, &["config", "user.name", "Plumb Test"]);
    Repo::git(root, &["config", "user.email", "plumb@example.invalid"]);
    Repo::git(
        root,
        &[
            "remote",
            "add",
            "origin",
            "https://git.example.invalid/Example/probe.git",
        ],
    );
    std::fs::create_dir(root.join("src")).expect("source");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("manifest");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn answer() -> u8 {\n    42\n}\n",
    )
    .expect("source");
    std::fs::write(
        root.join("plumb.toml"),
        "[workflow.hash.guard]\nrust = [\"Cargo.toml\", \"Cargo.lock\", \"src\"]\n",
    )
    .expect("governance");
    let output = Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(root)
        .output()
        .expect("lock");
    assert!(output.status.success());
    Repo::git(
        root,
        &["add", "Cargo.toml", "Cargo.lock", "src", "plumb.toml"],
    );
    held
}

pub(super) fn run(root: &Path, home: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["guard", ".", "--json"])
        .current_dir(root)
        .env("PLUMB_HOME", home)
        .env("CARGO_TARGET_DIR", root.join("unowned"))
        .output()
        .expect("guard")
}

fn seats(home: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(home.join("cache/guard/cargo"))
        .expect("cache")
        .map(|entry| entry.expect("entry").path())
        .collect()
}

pub(super) fn success(output: &Output) {
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn retained() {
    let fixture = fixture();
    let root = fixture.path();
    let home = super::seat();
    success(&run(root, home.path()));
    let held = seats(home.path());
    assert_eq!(held.len(), 1);
    assert!(held[0].join("debug/deps").is_dir());
    assert!(!root.join("target").exists());
    assert!(!root.join("unowned").exists());
    let retained = held[0].join("retained");
    std::fs::write(&retained, "material, not proof").expect("retained cache");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn answer() -> u8 {\n    43\n}\n",
    )
    .expect("changed source");
    Repo::git(root, &["add", "src/lib.rs"]);
    let changed = run(root, home.path());
    success(&changed);
    assert!(String::from_utf8_lossy(&changed.stderr).contains("guard guard/rust"));
    assert_eq!(seats(home.path()), held);
    assert!(retained.is_file());
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn answer() -> u8 {\n    false\n}\n",
    )
    .expect("invalid source");
    Repo::git(root, &["add", "src/lib.rs"]);
    let refused = run(root, home.path());
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("mismatched types"));
    assert!(retained.is_file());
}

#[test]
fn isolated() {
    let first = fixture();
    let second = fixture();
    let home = super::seat();
    success(&run(first.path(), home.path()));
    std::fs::write(
        second.path().join("src/lib.rs"),
        "pub fn answer() -> u8 {\n    44\n}\n",
    )
    .expect("different input");
    Repo::git(second.path(), &["add", "src/lib.rs"]);
    success(&run(second.path(), home.path()));
    assert_eq!(seats(home.path()).len(), 2);
}
