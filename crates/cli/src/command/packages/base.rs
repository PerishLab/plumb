use super::super::super::refresh;
use super::super::super::scope;
use super::{Fixture, Seat, State, engine, isolated, work};
use serde_json::json;
use std::path::Path;

fn commit(root: &Path, path: &str, text: &str) -> String {
    std::fs::write(root.join(path), text).unwrap();
    work::git(root, &["add", path]).unwrap();
    work::git(root, &["commit", "-q", "-m", "fixture delta"]).unwrap();
    work::git(root, &["rev-parse", "HEAD"]).unwrap()
}

fn fixture() -> (Fixture, std::path::PathBuf, State) {
    let fixture = Fixture::new(json!({}));
    let source = fixture.repository("pub fn answer() -> u8 {\n    false\n}\n");
    let mut state = State {
        repository: "Example/probe".into(),
        issue: 17,
        worktree: fixture.root.path().join("worktree"),
        ..Default::default()
    };
    work::Work {
        source: &source,
        state: &state,
    }
    .open()
    .unwrap();
    let text = std::fs::read_to_string(state.worktree.join("Cargo.toml")).unwrap();
    state.pushed = Some(commit(
        &state.worktree,
        "Cargo.toml",
        &format!("{text}description='owned delta'\n"),
    ));
    work::git(&source, &["push", "-q", "origin", "auto/17"]).unwrap();
    (fixture, source, state)
}

fn advance(source: &Path) {
    work::git(source, &["push", "-q", "origin", "main"]).unwrap();
    work::git(source, &["fetch", "origin"]).unwrap();
}

#[test]
fn incorporated() {
    let (_fixture, source, state) = fixture();
    let old = state.pushed.as_deref().unwrap();
    let text = std::fs::read_to_string(state.worktree.join("Cargo.toml")).unwrap();
    commit(&source, "Cargo.toml", &text);
    commit(&source, "src/lib.rs", "pub fn answer() -> u8 { 43 }\n");
    advance(&source);
    let base = work::git(&source, &["rev-parse", "origin/main"]).unwrap();
    assert!(scope::check(&state.worktree, &base, old).is_err());
    let candidate = refresh::prepare(&state).unwrap().unwrap();
    refresh::apply(&state.worktree, &candidate).unwrap();
    assert_eq!(
        work::git(&state.worktree, &["merge-base", "HEAD", &base]).unwrap(),
        base
    );
    assert_eq!(
        work::git(&state.worktree, &["merge-base", "HEAD", old]).unwrap(),
        old
    );
    assert_eq!(
        work::git(&state.worktree, &["diff", "--name-only", &base, "HEAD"]).unwrap(),
        ""
    );
    assert_eq!(
        std::fs::read_to_string(state.worktree.join("src/lib.rs")).unwrap(),
        "pub fn answer() -> u8 { 43 }\n"
    );
    assert_eq!(
        work::git(&source, &["rev-parse", "origin/auto/17"]).unwrap(),
        old
    );
    assert!(refresh::prepare(&state).unwrap().is_none());
}

#[test]
fn retained() {
    let (_fixture, source, state) = fixture();
    commit(&source, "src/lib.rs", "pub fn answer() -> u8 { 43 }\n");
    advance(&source);
    let candidate = refresh::prepare(&state).unwrap().unwrap();
    refresh::apply(&state.worktree, &candidate).unwrap();
    assert!(
        std::fs::read_to_string(state.worktree.join("Cargo.toml"))
            .unwrap()
            .contains("owned delta")
    );
    assert_eq!(
        work::git(
            &state.worktree,
            &["diff", "--name-only", "origin/main", "HEAD"]
        )
        .unwrap(),
        "Cargo.toml"
    );
}

#[test]
fn payload() {
    let (_fixture, source, state) = fixture();
    commit(&source, "src/lib.rs", "pub fn answer() -> u8 { 43 }\n");
    advance(&source);
    let head = state.pushed.as_deref().unwrap();
    for path in ["Cargo.toml", "keep.txt"] {
        let original = std::fs::read(state.worktree.join(path)).ok();
        std::fs::write(state.worktree.join(path), "valuable local payload\n").unwrap();
        assert!(
            refresh::prepare(&state)
                .unwrap_err()
                .contains("preserved local payload")
        );
        assert_eq!(
            work::git(&state.worktree, &["rev-parse", "HEAD"]).unwrap(),
            head
        );
        assert_eq!(
            std::fs::read_to_string(state.worktree.join(path)).unwrap(),
            "valuable local payload\n"
        );
        match original {
            Some(bytes) => std::fs::write(state.worktree.join(path), bytes).unwrap(),
            None => std::fs::remove_file(state.worktree.join(path)).unwrap(),
        }
    }
}

#[test]
fn foreign() {
    let (_fixture, source, state) = fixture();
    let head = commit(&state.worktree, "keep.txt", "foreign committed payload\n");
    commit(&source, "src/lib.rs", "pub fn answer() -> u8 { 43 }\n");
    advance(&source);
    assert!(
        refresh::prepare(&state)
            .unwrap_err()
            .contains("local head moved")
    );
    assert_eq!(
        work::git(&state.worktree, &["rev-parse", "HEAD"]).unwrap(),
        head
    );
}

#[test]
fn conflict() {
    let (_fixture, source, state) = fixture();
    commit(&source, "Cargo.toml", "conflicting ordinary main payload\n");
    advance(&source);
    assert!(refresh::prepare(&state).is_err());
    assert_eq!(
        work::git(&state.worktree, &["rev-parse", "HEAD"]).unwrap(),
        state.pushed.as_deref().unwrap()
    );
    assert_eq!(
        work::git(&state.worktree, &["status", "--porcelain"]).unwrap(),
        ""
    );
}

#[test]
fn scope() {
    let (_fixture, source, mut state) = fixture();
    state.pushed = Some(commit(
        &state.worktree,
        "src/lib.rs",
        "valuable foreign source\n",
    ));
    commit(&source, "Cargo.toml", "ordinary main manifest\n");
    advance(&source);
    assert!(refresh::prepare(&state).is_err());
    assert_eq!(
        std::fs::read_to_string(state.worktree.join("src/lib.rs")).unwrap(),
        "valuable foreign source\n"
    );
}

#[test]
fn advanced() {
    if !isolated("base::advanced") {
        return;
    }
    plumb::depot::carry(crate::catalog::carried::FILES);
    let (fixture, source, mut state) = fixture();
    state.schema = "plumb.auto-state/v1".into();
    let seat = Seat::fixture(fixture.root.path());
    let error = engine::advance(&source, &fixture.provider(), &seat, &mut state).unwrap_err();
    assert!(error.contains("failed with"), "{error}");
    state = seat.read("Example/probe").unwrap();
    let old = state.pushed.clone().unwrap();
    let text = std::fs::read_to_string(state.worktree.join("Cargo.toml")).unwrap();
    commit(&source, "Cargo.toml", &text);
    commit(
        &source,
        "src/lib.rs",
        "pub fn answer() -> u8 {\n    42\n}\n",
    );
    advance(&source);
    let error = engine::advance(&source, &fixture.provider(), &seat, &mut state).unwrap_err();
    assert!(error.contains("carries no released authority"), "{error}");
    let recovered = seat.read("Example/probe").unwrap();
    assert_eq!(recovered.pull, 18);
    let head = work::git(&state.worktree, &["rev-parse", "HEAD"]).unwrap();
    let base = work::git(&state.worktree, &["rev-parse", "origin/main"]).unwrap();
    scope::check(&state.worktree, &base, &head).unwrap();
    assert_eq!(
        work::git(&state.worktree, &["merge-base", &head, &old]).unwrap(),
        old
    );
    assert_eq!(fixture.provider().remote(17).unwrap(), recovered.candidate);
    assert_eq!(fixture.provider().issue(17).unwrap()["state"], "open");
    assert!(recovered.merged.is_none());
    assert_eq!(
        fixture
            .calls()
            .iter()
            .filter(|call| call["endpoint"] == "repos/Example/probe/pulls"
                && !call["payload"].is_null())
            .count(),
        1
    );
}
