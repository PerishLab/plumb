use super::{Fixture, Seat, State, isolated, work};
use serde_json::json;

#[test]
fn moved() {
    let fixture = Fixture::new(json!({}));
    let source = fixture.repository("pub fn answer() -> u8 { 42 }\n");
    let seat = Seat::fixture(fixture.root.path());
    let original = work::git(&source, &["rev-parse", "HEAD"]).unwrap();
    let state = State {
        schema: "plumb.auto-state/v1".into(),
        repository: "Example/probe".into(),
        issue: 17,
        worktree: fixture.root.path().join("worktree"),
        pushed: Some(original.clone()),
        candidate: Some(original.clone()),
        ..Default::default()
    };
    work::Work {
        source: &source,
        state: &state,
    }
    .open()
    .unwrap();
    seat.write(&state).unwrap();
    let tree = work::git(&source, &["rev-parse", "HEAD^{tree}"]).unwrap();
    let foreign = work::git(
        &source,
        &[
            "commit-tree",
            &tree,
            "-p",
            &original,
            "-m",
            "valuable peer commit",
        ],
    )
    .unwrap();
    work::git(
        &source,
        &["push", "origin", &format!("{foreign}:refs/heads/auto/17")],
    )
    .unwrap();
    let input = super::super::super::Input {
        root: source.clone(),
        github: fixture.command.clone(),
        json: true,
    };
    let mut current = seat.read("Example/probe").unwrap();
    let error =
        super::super::super::resume(&input, &fixture.provider(), &seat, &mut current).unwrap_err();
    assert!(error.contains("remote head moved"), "{error}");
    assert_eq!(
        work::git(&source, &["rev-parse", "HEAD"]).unwrap(),
        original
    );
    assert_eq!(
        work::git(&state.worktree, &["rev-parse", "HEAD"]).unwrap(),
        original
    );
    assert_eq!(fixture.provider().remote(17).unwrap(), Some(foreign));
    assert_eq!(
        seat.read("Example/probe").unwrap().candidate,
        Some(original)
    );
    assert!(fixture.calls().iter().all(|call| call["payload"].is_null()));
}

fn input(fixture: &Fixture, source: &std::path::Path) -> super::super::super::Input {
    super::super::super::Input {
        root: source.into(),
        github: fixture.command.clone(),
        json: true,
    }
}

fn publish(fixture: &Fixture, source: &std::path::Path) -> String {
    std::fs::write(source.join("Cargo.toml"), "[package]\nname='fixture'\nversion='0.1.0'\nedition='2024'\ndescription='retained delta'\n").unwrap();
    work::git(source, &["add", "Cargo.toml"]).unwrap();
    work::git(source, &["commit", "-q", "-m", "Refs #17."]).unwrap();
    let head = work::git(source, &["rev-parse", "HEAD"]).unwrap();
    std::fs::write(fixture.root.path().join("pushfault"), "once").unwrap();
    let error = fixture
        .provider()
        .publish(
            17,
            super::super::super::pull::Publication {
                root: source,
                head: &head,
                previous: None,
            },
        )
        .unwrap_err();
    assert!(error.contains("interrupted after branch push"), "{error}");
    assert!(!fixture.root.path().join("pull.json").exists());
    head
}

#[test]
fn cold() {
    if !isolated("refusal::cold") {
        return;
    }
    plumb::depot::carry(crate::catalog::carried::FILES);
    let fixture = Fixture::new(json!({}));
    let source = fixture.repository("pub fn answer() -> u8 {\n    false\n}\n");
    let head = publish(&fixture, &source);
    let (source, seat) = fixture.cold(&source);
    assert!(work::git(&source, &["cat-file", "-e", &head]).is_err());
    let mut state = seat.read("Example/probe").unwrap();
    assert_eq!(state.issue, 0);
    let error = super::super::super::resume(
        &input(&fixture, &source),
        &fixture.provider(),
        &seat,
        &mut state,
    )
    .unwrap_err();
    assert!(error.contains("failed with"), "{error}");
    assert_eq!(state.issue, 17);
    assert_eq!(state.pull, 18);
    assert_eq!(state.pushed.as_ref(), Some(&head));
    assert!(
        std::fs::read_to_string(state.worktree.join("Cargo.toml"))
            .unwrap()
            .contains("retained delta")
    );
    assert!(state.merged.is_none());
    let writes = fixture
        .calls()
        .iter()
        .filter(|call| {
            call["endpoint"] == "repos/Example/probe/pulls" && !call["payload"].is_null()
        })
        .count();
    assert_eq!(writes, 1);
    let (source, seat) = fixture.cold(&source);
    let mut state = seat.read("Example/probe").unwrap();
    assert_eq!(state.issue, 0);
    let error = super::super::super::resume(
        &input(&fixture, &source),
        &fixture.provider(),
        &seat,
        &mut state,
    )
    .unwrap_err();
    assert!(error.contains("failed with"), "{error}");
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
fn writer() {
    let fixture = Fixture::new(json!({}));
    let source = fixture.repository("pub fn answer() -> u8 { 42 }\n");
    let head = publish(&fixture, &source);
    let path = fixture.root.path().join("comments.json");
    let mut comments: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    comments[0]["user"]["id"] = 2.into();
    std::fs::write(path, comments.to_string()).unwrap();
    let (source, seat) = fixture.cold(&source);
    let before = fixture.calls().len();
    let mut state = seat.read("Example/probe").unwrap();
    let error = super::super::super::resume(
        &input(&fixture, &source),
        &fixture.provider(),
        &seat,
        &mut state,
    )
    .unwrap_err();
    assert!(error.contains("another operation writer"), "{error}");
    assert_eq!(fixture.provider().remote(17).unwrap(), Some(head));
    assert!(
        fixture.calls()[before..]
            .iter()
            .all(|call| call["payload"].is_null())
    );
}

#[test]
fn coldstop() {
    for reason in ["label", "orphan", "moved"] {
        let fixture = Fixture::new(json!({}));
        let source = fixture.repository("pub fn answer() -> u8 { 42 }\n");
        publish(&fixture, &source);
        block(&fixture, &source, reason);
        let (source, seat) = fixture.cold(&source);
        let before = fixture.calls().len();
        let mut state = seat.read("Example/probe").unwrap();
        let error = super::super::super::resume(
            &input(&fixture, &source),
            &fixture.provider(),
            &seat,
            &mut state,
        )
        .unwrap_err();
        let expected = match reason {
            "label" => "labels",
            "orphan" => "unrecorded",
            _ => "remote head moved",
        };
        assert!(error.contains(expected), "{reason}: {error}");
        assert!(
            fixture.calls()[before..]
                .iter()
                .all(|call| call["payload"].is_null())
        );
    }
}

fn block(fixture: &Fixture, source: &std::path::Path, reason: &str) {
    match reason {
        "label" => {
            let path = fixture.root.path().join("issue.json");
            let mut issue: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            issue["labels"] = json!([{"name":"needs:revalidation"}]);
            std::fs::write(path, issue.to_string()).unwrap();
        }
        "orphan" => {
            std::fs::write(fixture.root.path().join("comments.json"), "[]").unwrap();
        }
        _ => {
            let head = work::git(source, &["rev-parse", "HEAD"]).unwrap();
            let tree = work::git(source, &["rev-parse", "HEAD^{tree}"]).unwrap();
            let foreign = work::git(
                source,
                &["commit-tree", &tree, "-p", &head, "-m", "peer payload"],
            )
            .unwrap();
            work::git(
                source,
                &["push", "origin", &format!("{foreign}:refs/heads/auto/17")],
            )
            .unwrap();
        }
    }
}
