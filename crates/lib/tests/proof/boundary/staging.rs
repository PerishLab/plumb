use plumb::integration::staging::Claim;
use std::path::Path;

fn git(root: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
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
}

fn repository() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "-q", "-b", "topic"]);
    git(root.path(), &["config", "user.name", "Plumb Test"]);
    git(
        root.path(),
        &["config", "user.email", "plumb@example.invalid"],
    );
    std::fs::write(root.path().join("source"), "held\n").unwrap();
    git(root.path(), &["add", "source"]);
    git(root.path(), &["commit", "-qm", "base"]);
    root
}

#[test]
fn retained() {
    let root = repository();
    let home = tempfile::tempdir().unwrap();
    let claim = Claim::retain(home.path(), root.path()).unwrap();
    let path = claim.root.clone();
    git(
        root.path(),
        &[
            "worktree",
            "add",
            "--detach",
            "--quiet",
            path.to_str().unwrap(),
        ],
    );
    assert!(claim.verify());
    let stamp = std::fs::metadata(path.join("source"))
        .unwrap()
        .modified()
        .unwrap();
    let lease = std::fs::File::options()
        .read(true)
        .write(true)
        .open(path.with_extension("lock"))
        .unwrap();
    assert!(lease.try_lock().is_err(), "the source lease is exclusive");
    drop(claim);
    lease.try_lock().unwrap();
    lease.unlock().unwrap();
    let claim = Claim::retain(home.path(), root.path()).unwrap();
    assert_eq!(claim.root, path);
    assert!(claim.verify());
    assert_eq!(
        std::fs::metadata(path.join("source"))
            .unwrap()
            .modified()
            .unwrap(),
        stamp
    );
    git(
        root.path(),
        &["worktree", "remove", "--force", path.to_str().unwrap()],
    );
}

#[test]
fn disagreement() {
    let root = repository();
    let home = tempfile::tempdir().unwrap();
    let claim = Claim::retain(home.path(), root.path()).unwrap();
    let record = claim.root.with_extension("lock");
    drop(claim);
    std::fs::write(&record, "{\"schema\":\"foreign\"}").unwrap();
    assert!(Claim::retain(home.path(), root.path()).is_err());
    assert_eq!(
        std::fs::read_to_string(record).unwrap(),
        "{\"schema\":\"foreign\"}"
    );
}

#[test]
#[cfg(windows)]
fn verbatim() {
    use std::path::{Component, Prefix};
    let root = repository();
    let home = tempfile::tempdir().unwrap();
    let base = home.path().join("with spaces");
    std::fs::create_dir(&base).unwrap();
    let extended = base.canonicalize().unwrap();
    let claim = Claim::retain(&extended, root.path()).unwrap();
    let Some(Component::Prefix(prefix)) = claim.root.components().next() else {
        panic!("absolute Windows source seat");
    };
    assert!(!matches!(
        prefix.kind(),
        Prefix::VerbatimDisk(_) | Prefix::VerbatimUNC(_, _) | Prefix::Verbatim(_)
    ));
    git(
        root.path(),
        &[
            "worktree",
            "add",
            "--detach",
            "--quiet",
            claim.root.to_str().unwrap(),
        ],
    );
    assert!(claim.verify());
    git(
        root.path(),
        &[
            "worktree",
            "remove",
            "--force",
            claim.root.to_str().unwrap(),
        ],
    );
}
