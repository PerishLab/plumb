use crate::command::cache::transport::Context;
use crate::command::cache::transport::record::Hint;
use crate::command::cache::transport::target::{Target, triple};

#[test]
fn triples() {
    for name in [
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
        "x86_64-pc-windows-msvc",
    ] {
        assert!(triple(name), "{name}");
    }
    for name in [
        "debug",
        "doc",
        "package",
        "x86_64",
        "a--b",
        "Upper-case-name",
        "a-b-c-d-e",
    ] {
        assert!(!triple(name), "{name}");
    }
}

#[test]
fn host() {
    let root = tempfile::tempdir().unwrap();
    for name in ["debug", "release", "tmp", "x86_64-unknown-linux-gnu/debug"] {
        std::fs::create_dir_all(root.path().join(name)).unwrap();
    }
    assert!(Target(root.path()).supported().unwrap());
    let before = Target(root.path()).seal().unwrap();
    std::fs::write(
        root.path().join("x86_64-unknown-linux-gnu/debug/unit"),
        "lock",
    )
    .unwrap();
    let after = Target(root.path()).seal().unwrap();
    assert_eq!(
        before.sha, after.sha,
        "per-target output stays outside the seal"
    );
    std::fs::create_dir(root.path().join("doc")).unwrap();
    assert!(!Target(root.path()).supported().unwrap());
}

#[test]
fn reasons() {
    let root = tempfile::tempdir().unwrap();
    let context = Context {
        image: format!("sha256:{}", "a".repeat(64)),
    };
    assert_eq!(
        Hint::refusal(root.path(), &context).unwrap(),
        Some("snapshot-missing")
    );
    assert!(!Hint::ready(root.path(), &context).unwrap());
}
