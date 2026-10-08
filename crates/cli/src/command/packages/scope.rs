use super::super::super::scope;
use super::{Fixture, work};
use serde_json::json;
use std::path::Path;

fn commit(root: &Path) -> String {
    work::git(root, &["add", "."]).unwrap();
    work::git(root, &["commit", "-q", "-m", "fixture delta"]).unwrap();
    work::git(root, &["rev-parse", "HEAD"]).unwrap()
}

fn put(root: &Path, path: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, "fixture payload\n").unwrap();
}

#[test]
fn nested() {
    let fixture = Fixture::new(json!({}));
    let root = fixture.repository("pub fn answer() -> u8 { 42 }\n");
    let base = work::git(&root, &["rev-parse", "HEAD"]).unwrap();
    for path in [
        "package.json",
        "pnpm-lock.yaml",
        "crates/probe/Cargo.toml",
        "crates/probe/Cargo.lock",
        "packages/probe/package.json",
        "packages/probe/pnpm-lock.yaml",
    ] {
        put(&root, path);
    }
    let head = commit(&root);
    scope::check(&root, &base, &head).unwrap();
    work::git(
        &root,
        &["mv", "crates/probe/Cargo.lock", "crates/probe/other.lock"],
    )
    .unwrap();
    let foreign = commit(&root);
    assert!(
        scope::check(&root, &head, &foreign)
            .unwrap_err()
            .contains("other.lock")
    );
}

#[test]
fn removed() {
    let fixture = Fixture::new(json!({}));
    let root = fixture.repository("pub fn answer() -> u8 { 42 }\n");
    put(&root, "crates/probe/Cargo.lock");
    let base = commit(&root);
    work::git(
        &root,
        &[
            "mv",
            "crates/probe/Cargo.lock",
            "crates/probe/pnpm-lock.yaml",
        ],
    )
    .unwrap();
    let renamed = commit(&root);
    scope::check(&root, &base, &renamed).unwrap();
    work::git(&root, &["rm", "crates/probe/pnpm-lock.yaml"]).unwrap();
    let removed = commit(&root);
    scope::check(&root, &renamed, &removed).unwrap();
}

#[test]
fn projected() {
    let fixture = Fixture::new(json!({}));
    let root = fixture.repository("pub fn answer() -> u8 { 42 }\n");
    let base = work::git(&root, &["rev-parse", "HEAD"]).unwrap();
    put(&root, "crates/probe/Cargo.lock");
    let source = commit(&root);
    let tree = work::git(&root, &["rev-parse", "HEAD^{tree}"]).unwrap();
    let candidate = work::git(
        &root,
        &[
            "commit-tree",
            &tree,
            "-p",
            &base,
            "-m",
            "different projected narrative",
        ],
    )
    .unwrap();
    assert_ne!(candidate, source);
    scope::check(&root, &base, &candidate).unwrap();
    assert_eq!(work::git(&root, &["rev-parse", "HEAD"]).unwrap(), source);
    assert_eq!(work::git(&root, &["status", "--porcelain"]).unwrap(), "");
    put(&root, "keep.txt");
    assert!(
        scope::check(&root, &base, &candidate)
            .unwrap_err()
            .contains("preserved local payload")
    );
    assert_eq!(
        std::fs::read_to_string(root.join("keep.txt")).unwrap(),
        "fixture payload\n"
    );
}

#[test]
fn source() {
    let fixture = Fixture::new(json!({}));
    let root = fixture.repository("pub fn answer() -> u8 { 42 }\n");
    let base = work::git(&root, &["rev-parse", "HEAD"]).unwrap();
    work::git(&root, &["rm", "Cargo.toml"]).unwrap();
    put(&root, "Cargo.toml/src.rs");
    let head = commit(&root);
    assert!(
        scope::check(&root, &base, &head)
            .unwrap_err()
            .contains("Cargo.toml/src.rs")
    );
}
