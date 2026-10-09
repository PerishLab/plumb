use super::super::super::scope;
use super::{Fixture, isolated, work};
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

#[test]
fn cold() {
    if !isolated("scope::cold") {
        return;
    }
    plumb::depot::carry(crate::catalog::carried::FILES);
    let fixture = Fixture::new(json!({}));
    let source = fixture.repository("pub fn answer() -> u8 {\n    42\n}\n");
    let manifest = source.join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(manifest, format!("{text}description='cold merge'\n")).unwrap();
    work::git(&source, &["add", "Cargo.toml"]).unwrap();
    work::git(&source, &["commit", "-q", "-m", "Refs #17."]).unwrap();
    let proof = crate::command::guard::precommit::branch::renew(&source).unwrap();
    let parent = work::git(&source, &["rev-parse", "origin/main"]).unwrap();
    let message = format!(
        "Refs #17.\n\n{} {}",
        plumb::guard::TRAILER,
        proof.encode().unwrap()
    );
    let candidate = work::git(
        &source,
        &["commit-tree", &proof.tree, "-p", &parent, "-m", &message],
    )
    .unwrap();
    let merge = work::git(
        &source,
        &[
            "commit-tree",
            &proof.tree,
            "-p",
            &parent,
            "-m",
            &format!("Merged\n{message}"),
        ],
    )
    .unwrap();
    work::git(
        &source,
        &[
            "push",
            "origin",
            &format!("{merge}:refs/heads/main"),
            &format!("{candidate}:refs/pull/18/head"),
        ],
    )
    .unwrap();
    std::fs::write(fixture.root.path().join("pull.json"), json!({"number":18,"state":"closed","merged_at":"2026-10-09T00:00:00Z","merge_commit_sha":merge,"head":{"ref":"auto/17","sha":candidate,"repo":{"full_name":"Example/probe"}},"base":{"ref":"main"},"body":"Refs #17."}).to_string()).unwrap();
    std::fs::write(fixture.root.path().join("guard.json"), json!({"name":"Guard","app":{"slug":"github-actions"},"head_sha":candidate,"status":"completed","conclusion":"success","details_url":"https://github.com/Example/probe/actions/runs/17"}).to_string()).unwrap();
    std::fs::write(
        fixture.root.path().join("run.json"),
        json!({"head_sha":candidate,"path":".github/workflows/guard.yml","event":"pull_request"})
            .to_string(),
    )
    .unwrap();
    let (source, seat) = fixture.cold(&source);
    assert!(work::git(&source, &["cat-file", "-e", &candidate]).is_err());
    assert!(fixture.provider().remote(17).unwrap().is_none());
    let input = super::super::super::Input {
        root: source.clone(),
        github: fixture.command.clone(),
        json: true,
    };
    let mut state = seat.read("Example/probe").unwrap();
    let result =
        super::super::super::resume(&input, &fixture.provider(), &seat, &mut state).unwrap();
    assert!(result.contains("completed via pull #18"), "{result}");
    assert_eq!(fixture.provider().issue(17).unwrap()["state"], "closed");
    assert_eq!(state.issue, 0);
    let result =
        super::super::super::resume(&input, &fixture.provider(), &seat, &mut state).unwrap();
    assert!(result.contains("already current"), "{result}");
    assert_eq!(
        fixture
            .calls()
            .iter()
            .filter(|call| call["endpoint"] == "repos/Example/probe/pulls"
                && !call["payload"].is_null())
            .count(),
        0
    );
}

#[test]
fn issueonly() {
    plumb::depot::carry(crate::catalog::carried::FILES);
    let fixture = Fixture::new(json!({}));
    let source = fixture.repository("pub fn answer() -> u8 { 42 }\n");
    std::fs::write(source.join("package.json"), "{").unwrap();
    work::git(&source, &["add", "package.json"]).unwrap();
    work::git(&source, &["commit", "-q", "-m", "fixture resolver blocker"]).unwrap();
    work::git(&source, &["push", "origin", "main"]).unwrap();
    let (source, seat) = fixture.cold(&source);
    let input = super::super::super::Input {
        root: source.clone(),
        github: fixture.command.clone(),
        json: true,
    };
    let mut state = seat.read("Example/probe").unwrap();
    let error =
        super::super::super::resume(&input, &fixture.provider(), &seat, &mut state).unwrap_err();
    assert!(error.contains("EOF"), "{error}");
    assert_eq!(state.issue, 17);
    assert!(!fixture.root.path().join("pull.json").exists());
    assert_eq!(fixture.provider().issue(17).unwrap()["state"], "open");
    assert!(fixture.calls().iter().all(|call| call["payload"].is_null()));
}

#[test]
fn stopped() {
    for reason in ["failure", "closed"] {
        let fixture = Fixture::new(json!({}));
        let source = fixture.repository("pub fn answer() -> u8 { 42 }\n");
        let head = work::git(&source, &["rev-parse", "HEAD"]).unwrap();
        fixture
            .provider()
            .publish(
                17,
                super::super::super::pull::Publication {
                    root: &source,
                    head: &head,
                    previous: None,
                },
            )
            .unwrap();
        stop(&fixture, reason, &head);
        let (source, seat) = fixture.cold(&source);
        let before = fixture.calls().len();
        let input = super::super::super::Input {
            root: source.clone(),
            github: fixture.command.clone(),
            json: true,
        };
        let mut state = seat.read("Example/probe").unwrap();
        let error = super::super::super::resume(&input, &fixture.provider(), &seat, &mut state)
            .unwrap_err();
        let expected = if reason == "failure" {
            "Guard failed"
        } else {
            "closed without merge"
        };
        assert!(error.contains(expected), "{reason}: {error}");
        assert!(
            fixture.calls()[before..]
                .iter()
                .all(|call| call["payload"].is_null())
        );
    }
}

fn stop(fixture: &Fixture, reason: &str, head: &str) {
    if reason == "failure" {
        std::fs::write(fixture.root.path().join("guard.json"), json!({"name":"Guard","app":{"slug":"github-actions"},"head_sha":head,"status":"completed","conclusion":"failure","details_url":"https://github.com/Example/probe/actions/runs/17"}).to_string()).unwrap();
    } else {
        let path = fixture.root.path().join("pull.json");
        let mut pull: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        pull["state"] = "closed".into();
        std::fs::write(path, pull.to_string()).unwrap();
    }
}
