use std::path::Path;
use std::process::Command;

#[test]
fn empty() {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "-q"]);
    let expected = git(root.path(), &["mktree"]);
    assert_eq!(plumb::guard::tree(root.path()).unwrap(), expected);
    assert!(!root.path().join(".git/index").exists());
}

#[test]
fn snapshot() {
    for split in [false, true] {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path();
        git(root, &["init", "-q"]);
        std::fs::write(root.join("item"), "staged").unwrap();
        git(root, &["add", "item"]);
        git(root, &["update-index", "--chmod=+x", "item"]);
        std::fs::write(root.join("intent"), "not staged").unwrap();
        git(root, &["add", "--intent-to-add", "intent"]);
        if split {
            git(root, &["update-index", "--split-index"]);
        }
        let expected = git(root, &["write-tree"]);
        let index = root.join(".git/index");
        let before = std::fs::read(&index).unwrap();
        std::fs::write(root.join("item"), "unstaged").unwrap();
        let lock = root.join(".git/index.lock");
        std::fs::write(&lock, "owned by another operation").unwrap();
        std::thread::scope(|scope| {
            let readers = (0..8)
                .map(|_| scope.spawn(|| plumb::guard::tree(root)))
                .collect::<Vec<_>>();
            for reader in readers {
                assert_eq!(reader.join().unwrap().unwrap(), expected);
            }
        });
        assert_eq!(std::fs::read(index).unwrap(), before);
        assert_eq!(
            std::fs::read_to_string(lock).unwrap(),
            "owned by another operation"
        );
    }
}

#[test]
fn corrupt() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path();
    git(root, &["init", "-q"]);
    std::fs::write(root.join(".git/index"), "invalid index").unwrap();
    assert!(plumb::guard::tree(root).is_err());
    assert_eq!(
        std::fs::read_to_string(root.join(".git/index")).unwrap(),
        "invalid index"
    );
}

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
    String::from_utf8_lossy(&output.stdout).trim().into()
}
