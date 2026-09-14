use plumb::datum::{Datum, Git, Tree, carried};
use std::path::Path;
use std::process::Command;

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

#[test]
fn metadata() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.name", "test"]);
    git(root, &["config", "user.email", "test@example.invalid"]);
    std::fs::write(root.join("source"), "source").unwrap();
    git(root, &["add", "source"]);
    let datum = Datum::new("v1.2.0", Vec::new());
    git(
        root,
        &[
            "commit",
            "-qm",
            &format!("datum\n\n{}", datum.trailer().unwrap()),
        ],
    );
    let first = git(root, &["rev-parse", "HEAD"]);
    git(root, &["tag", "-a", "marker", "-m", "marker"]);
    assert_eq!(Git(root).at("v1.2.0", &first).unwrap(), Some(datum.clone()));
    assert_eq!(Tree(root).read("v1.2.0").unwrap(), Some(datum.clone()));
    assert!(!root.join(".plumb").exists());
    assert!(Tree(root).capture("").unwrap().is_none());
    git(root, &["checkout", "-qb", "release/v1.2.0"]);
    assert_eq!(Tree(root).capture("").unwrap(), Some(datum.clone()));
    git(root, &["commit", "--allow-empty", "-qm", "picked source"]);
    assert!(Git(root).at("v1.2.0", "HEAD").unwrap().is_none());
    assert_eq!(Tree(root).capture("").unwrap(), Some(datum.clone()));
    assert_eq!(Tree(root).read("v1.2.0").unwrap(), Some(datum.clone()));
    assert!(Tree(root).read("v9.0.0").unwrap().is_none());
    assert_eq!(
        git(root, &["ls-tree", "-r", "--name-only", "HEAD"]),
        "source"
    );
    let clone = tempfile::tempdir().unwrap();
    git(
        root,
        &[
            "clone",
            "--quiet",
            "--no-local",
            "--depth=1",
            "--branch",
            "marker",
            ".",
            clone.path().to_str().unwrap_or(""),
        ],
    );
    assert_eq!(Tree(clone.path()).line(""), Some("v1.2.0".into()));
    assert!(Tree(clone.path()).capture("v9.0.0").is_err());
    assert_eq!(Tree(clone.path()).capture("").unwrap(), Some(datum.clone()));
    assert_eq!(
        Tree(clone.path()).capture("v1.2.0-beta.1").unwrap(),
        Some(datum.clone())
    );
    assert_eq!(Git(clone.path()).at("v1.2.0", "HEAD").unwrap(), Some(datum));
    assert!(!clone.path().join(".plumb").exists());
}

#[test]
fn strict() {
    let datum = Datum::new("v1.2.0", Vec::new());
    let trailer = datum.trailer().unwrap();
    assert_eq!(carried(&trailer).unwrap(), Some(datum.clone()));
    assert!(carried(&format!("{trailer}\n{trailer}")).is_err());
    assert!(carried("Plumb-Datum: invalid").is_err());
    let mut invalid = datum.clone();
    invalid.schema = 99;
    assert!(carried(&invalid.trailer().unwrap()).is_err());
    assert!(carried("ordinary message").unwrap().is_none());
    assert_eq!(datum.digest().unwrap().len(), 64);
}

#[test]
fn identity() {
    let temp = tempfile::tempdir().unwrap();
    let tree = Tree(temp.path());
    for reference in [
        "v1.2.0",
        "v1.2.0-beta.1",
        "v1.2.0-rc.2+build.7",
        "refs/heads/release/v1.2.0",
        "release/v1.2.0-beta.1",
    ] {
        assert_eq!(tree.line(reference), Some("v1.2.0".into()));
    }
    for reference in ["v1.2", "v1.2.0-beta..1", "v1.2.0-", "broken-beta.1"] {
        assert_eq!(tree.line(reference), Some(reference.into()));
    }
    let datum = Datum::new("v1.2.0", Vec::new());
    let path = tree.seat("v1.2.0");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, datum.encode().unwrap()).unwrap();
    let line = tree.line("v1.2.0-beta.1").unwrap();
    assert_eq!(tree.read(&line).unwrap(), Some(datum));
    assert!(tree.read("v1.2.0-beta.1").unwrap().is_none());
}
