use super::{Fixture, Seat, State, work};
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
