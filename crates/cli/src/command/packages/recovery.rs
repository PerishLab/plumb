use super::super::{Seat, State, closure, engine, work};
use super::{Fixture, isolated, issue};
use serde_json::json;

#[path = "fixture.rs"]
mod fixture;

#[path = "authority.rs"]
mod authority;

#[path = "refusal.rs"]
mod refusal;
#[path = "scope.rs"]
mod scope;

#[path = "base.rs"]
mod base;

#[test]
fn interrupted() {
    if !isolated("interrupted") {
        return;
    }
    plumb::depot::carry(crate::catalog::carried::FILES);
    let fixture = Fixture::new(json!({}));
    let source = fixture.repository("pub fn answer() -> u8 {\n    false\n}\n");
    let seat = Seat::fixture(fixture.root.path());
    let mut state = State {
        schema: "plumb.auto-state/v1".into(),
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
    let manifest = state.worktree.join("Cargo.toml");
    let current = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(
        manifest,
        format!("{current}description='bounded fixture'\n"),
    )
    .unwrap();
    work::git(&state.worktree, &["add", "Cargo.toml"]).unwrap();
    work::git(
        &state.worktree,
        &["commit", "-q", "-m", "fixture manifest delta"],
    )
    .unwrap();
    std::fs::write(fixture.root.path().join("fault"), "once").unwrap();
    let fault = engine::advance(&source, &fixture.provider(), &seat, &mut state).unwrap_err();
    assert!(
        fault.contains("interrupted after provider write"),
        "{fault}"
    );
    state = seat.read("Example/probe").unwrap();
    assert_eq!(state.pull, 0);
    assert!(
        state.candidate.is_some(),
        "candidate is durable before provider write"
    );
    for _ in 0..2 {
        let error = engine::advance(&source, &fixture.provider(), &seat, &mut state).unwrap_err();
        assert!(error.contains("failed with"), "{error}");
        state = seat.read("Example/probe").unwrap();
        assert_eq!(state.pull, 18);
        assert!(state.merged.is_none());
    }
    assert_eq!(fixture.provider().issue(17).unwrap()["state"], "open");
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

#[test]
fn closed() {
    if !isolated("closed") {
        return;
    }
    plumb::depot::carry(crate::catalog::carried::FILES);
    let fixture = Fixture::new(json!({}));
    let source = fixture.repository("pub fn answer() -> u8 {\n    42\n}\n");
    let seat = Seat::fixture(fixture.root.path());
    let mut state = State {
        schema: "plumb.auto-state/v1".into(),
        repository: "Example/probe".into(),
        issue: 17,
        pull: 18,
        worktree: fixture.root.path().join("worktree"),
        ..Default::default()
    };
    work::Work {
        source: &source,
        state: &state,
    }
    .open()
    .unwrap();
    let manifest = state.worktree.join("Cargo.toml");
    let current = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(
        manifest,
        format!("{current}description='bounded fixture'\n"),
    )
    .unwrap();
    work::git(&state.worktree, &["add", "Cargo.toml"]).unwrap();
    work::git(
        &state.worktree,
        &["commit", "-q", "-m", "fixture manifest delta"],
    )
    .unwrap();
    let proof = crate::command::guard::precommit::branch::renew(&state.worktree).unwrap();
    let parent = work::git(&source, &["rev-parse", "origin/main"]).unwrap();
    let token = proof.encode().unwrap();
    let message = format!("Refs #17.\n\n{} {token}", plumb::guard::TRAILER);
    let candidate = work::git(
        &source,
        &["commit-tree", &proof.tree, "-p", &parent, "-m", &message],
    )
    .unwrap();
    let merged = work::git(
        &source,
        &[
            "commit-tree",
            &proof.tree,
            "-p",
            &parent,
            "-m",
            &format!("Fixture merge\n\n{} {token}", plumb::guard::TRAILER),
        ],
    )
    .unwrap();
    work::git(
        &source,
        &[
            "push",
            "origin",
            &format!("{candidate}:refs/heads/auto/17"),
            &format!("{merged}:refs/heads/main"),
        ],
    )
    .unwrap();
    state.candidate = Some(candidate);
    state.merged = Some(merged);
    state.guard = Some("https://github.com/Example/probe/actions/runs/17".into());
    seat.write(&state).unwrap();
    let owned = state.worktree.clone();
    let payload = owned.join("keep.txt");
    std::fs::write(&payload, "valuable interrupted payload\n").unwrap();
    assert!(closure::close(&source, &fixture.provider(), &seat, &mut state).is_err());
    assert_eq!(fixture.provider().issue(17).unwrap()["state"], "open");
    assert_eq!(
        std::fs::read_to_string(&payload).unwrap(),
        "valuable interrupted payload\n"
    );
    std::fs::remove_file(payload).unwrap();
    std::fs::write(fixture.root.path().join("closefault"), "once").unwrap();
    assert!(
        closure::close(&source, &fixture.provider(), &seat, &mut state)
            .unwrap_err()
            .contains("interrupted after closure write")
    );
    state = seat.read("Example/probe").unwrap();
    closure::close(&source, &fixture.provider(), &seat, &mut state).unwrap();
    let issue = fixture.provider().issue(17).unwrap();
    assert_eq!(issue["state"], "closed");
    assert_eq!(
        issue["body"],
        issue::body("Example/probe").replace("- [ ]", "- [x]")
    );
    assert_eq!(seat.read("Example/probe").unwrap().issue, 0);
    assert!(!owned.exists());
    let comments: Vec<serde_json::Value> =
        serde_json::from_slice(&std::fs::read(fixture.root.path().join("comments.json")).unwrap())
            .unwrap();
    assert_eq!(
        comments
            .iter()
            .filter(|comment| comment["body"]
                .as_str()
                .unwrap()
                .contains("plumb.auto-closure/v1"))
            .count(),
        1
    );
    assert!(fixture.provider().remote(17).unwrap().is_none());
}

#[test]
fn current() {
    plumb::depot::carry(crate::catalog::carried::FILES);
    let fixture = Fixture::new(json!({}));
    let source = fixture.repository("pub fn answer() -> u8 {\n    42\n}\n");
    let seat = Seat::fixture(fixture.root.path());
    let mut state = State {
        schema: "plumb.auto-state/v1".into(),
        repository: "Example/probe".into(),
        ..Default::default()
    };
    let input = super::super::Input {
        root: source.clone(),
        github: fixture.command.clone(),
        json: true,
    };
    let result = super::super::resume(&input, &fixture.provider(), &seat, &mut state).unwrap();
    assert!(result.contains("already current"));
    assert_eq!(state.issue, 0);
    assert!(fixture.calls().iter().all(|call| call["payload"].is_null()));
    assert_eq!(work::git(&source, &["status", "--porcelain"]).unwrap(), "");
    assert!(!fixture.root.path().join("pull.json").exists());
}

#[test]
fn foreign() {
    let fixture = Fixture::new(json!({}));
    let source = fixture.repository("pub fn answer() -> u8 { 42 }\n");
    let state = State {
        repository: "Example/probe".into(),
        issue: 17,
        worktree: fixture.root.path().join("foreign"),
        ..Default::default()
    };
    work::git(
        &source,
        &[
            "worktree",
            "add",
            "-b",
            "auto/17",
            state.worktree.to_str().unwrap(),
        ],
    )
    .unwrap();
    std::fs::write(
        state.worktree.join("Cargo.toml"),
        "valuable foreign payload\n",
    )
    .unwrap();
    let error = work::Work {
        source: &source,
        state: &state,
    }
    .open()
    .unwrap_err();
    assert!(error.contains("ownership marker"), "{error}");
    assert_eq!(
        std::fs::read_to_string(state.worktree.join("Cargo.toml")).unwrap(),
        "valuable foreign payload\n"
    );
    let dir = work::git(&state.worktree, &["rev-parse", "--absolute-git-dir"]).unwrap();
    assert!(!std::path::Path::new(&dir).join("plumb-auto.json").exists());
}
