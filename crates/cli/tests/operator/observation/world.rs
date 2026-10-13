use std::path::Path;
use std::process::{Command, Output};

pub struct Repo;

impl Repo {
    pub fn git(root: &Path, args: &[&str]) -> Output {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }
}

pub fn fixture() -> tempfile::TempDir {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path();
    Repo::git(root, &["init", "-q", "-b", "topic"]);
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
    std::fs::write(
        root.join("package.json"),
        r#"{"dependencies":{"@perishlab/probe":"0"}}"#,
    )
    .unwrap();
    fixture
}

pub fn success(output: &Output) {
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
