use super::super::super::super::closure;
use super::super::super::super::provider::Intent;
use super::{Fixture, Seat, State, advance, commit, engine, fixture, isolated, work};
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn settled(fixture: &Fixture, source: &Path, seat: &Seat, old: &str) {
    let provider = fixture.provider();
    let current = provider.issue(17).unwrap();
    assert_eq!(current["state"], "closed");
    assert!(!current["body"].as_str().unwrap().contains("- [ ]"));
    let pull: Value =
        serde_json::from_slice(&std::fs::read(fixture.root.path().join("pull.json")).unwrap())
            .unwrap();
    assert_eq!(pull["state"], "closed");
    assert_eq!(pull["head"]["sha"], old);
    assert!(provider.remote(17).unwrap().is_none());
    assert_eq!(seat.read("Example/probe").unwrap().issue, 0);
    assert!(!fixture.root.path().join("worktree").exists());
    let receipts = provider
        .pages("repos/Example/probe/issues/17/comments?per_page=100")
        .unwrap()
        .into_iter()
        .filter(|comment| {
            comment["body"]
                .as_str()
                .unwrap()
                .contains("<!-- plumb.auto-absorbed/v1\n")
        })
        .count();
    assert_eq!(receipts, 1);
    let receipt = provider.absorption(17).unwrap().unwrap();
    assert_eq!((receipt.pull, receipt.head.as_str()), (18, old));
    assert_eq!(
        receipt.main,
        work::git(source, &["rev-parse", "origin/main"]).unwrap()
    );
    assert!(
        fixture
            .calls()
            .iter()
            .all(|call| call["endpoint"] != "merge" && call["payload"]["merge_method"].is_null())
    );
}

pub(super) fn intent(fixture: &Fixture, source: &Path, state: &State) {
    let head = state.pushed.clone().unwrap();
    let held = Intent {
        schema: "plumb.auto-push/v1".into(),
        repository: "Example/probe".into(),
        issue: 17,
        branch: "auto/17".into(),
        previous: None,
        base: work::git(source, &["merge-base", "origin/main", &head]).unwrap(),
        tree: work::git(source, &["rev-parse", &format!("{head}^{{tree}}")]).unwrap(),
        head,
    };
    fixture.provider().intent(17, &held).unwrap();
}

fn stopped(fixture: &Fixture, source: &Path, seat: &Seat) -> (State, String) {
    let mut state = seat.read("Example/probe").unwrap();
    let error = engine::advance(source, &fixture.provider(), seat, &mut state).unwrap_err();
    assert!(error.contains("failed with"), "{error}");
    let state = seat.read("Example/probe").unwrap();
    let old = state.pushed.clone().unwrap();
    let text = std::fs::read_to_string(state.worktree.join("Cargo.toml")).unwrap();
    commit(source, "Cargo.toml", &text);
    advance(source);
    (state, old)
}

fn held() -> (Fixture, std::path::PathBuf, Seat) {
    plumb::depot::carry(crate::catalog::carried::FILES);
    let (fixture, source, mut state) = fixture();
    state.schema = "plumb.auto-state/v1".into();
    let seat = Seat::fixture(fixture.root.path());
    intent(&fixture, &source, &state);
    seat.write(&state).unwrap();
    (fixture, source, seat)
}

#[test]
fn interrupted() {
    if !isolated("base::absorbed::interrupted") {
        return;
    }
    let (fixture, source, seat) = held();
    let (mut state, old) = stopped(&fixture, &source, &seat);
    std::fs::write(fixture.root.path().join("closefault"), "once").unwrap();
    let error = engine::advance(&source, &fixture.provider(), &seat, &mut state).unwrap_err();
    assert!(error.contains("interrupted after closure write"), "{error}");
    let mut state = seat.read("Example/probe").unwrap();
    assert_eq!(state.issue, 17);
    assert!(closure::resume(&source, &fixture.provider(), &seat, &mut state).unwrap());
    settled(&fixture, &source, &seat, &old);
}

#[test]
fn foreign() {
    if !isolated("base::absorbed::foreign") {
        return;
    }
    let (fixture, source, seat) = held();
    let (mut state, old) = stopped(&fixture, &source, &seat);
    let path = fixture.root.path().join("pull.json");
    let mut pull: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    pull["head"]["sha"] = work::git(&source, &["rev-parse", "origin/main"])
        .unwrap()
        .into();
    std::fs::write(&path, pull.to_string()).unwrap();
    let error = engine::advance(&source, &fixture.provider(), &seat, &mut state).unwrap_err();
    assert!(error.contains("not its own exact unmerged head"), "{error}");
    let provider = fixture.provider();
    assert_eq!(provider.issue(17).unwrap()["state"], "open");
    assert_eq!(provider.remote(17).unwrap(), Some(old));
    assert!(provider.absorption(17).unwrap().is_none());
}

#[test]
fn shadowed() {
    if !isolated("base::absorbed::shadowed") {
        return;
    }
    let (fixture, source, seat) = held();
    let (mut state, old) = stopped(&fixture, &source, &seat);
    std::fs::write(fixture.root.path().join("receiptfault"), "once").unwrap();
    let error = engine::advance(&source, &fixture.provider(), &seat, &mut state).unwrap_err();
    assert!(
        error.contains("interrupted after absorption receipt"),
        "{error}"
    );
    let path = fixture.root.path().join("comments.json");
    let mut comments: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let receipt = comments
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|comment| {
            comment["body"]
                .as_str()
                .unwrap()
                .contains("plumb.auto-absorbed/v1")
        })
        .unwrap();
    receipt["user"] = json!({"id":2,"login":"foreign-operator"});
    let body = receipt["body"].clone();
    std::fs::write(&path, comments.to_string()).unwrap();
    let mut state = seat.read("Example/probe").unwrap();
    let message = engine::advance(&source, &fixture.provider(), &seat, &mut state).unwrap();
    assert!(message.contains("was absorbed by main"), "{message}");
    let comments: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let authors = comments
        .as_array()
        .unwrap()
        .iter()
        .filter(|comment| comment["body"] == body)
        .map(|comment| comment["user"]["id"].as_u64().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(authors, [2, 1]);
    let provider = fixture.provider();
    assert_eq!(provider.absorption(17).unwrap().unwrap().head, old);
    assert_eq!(provider.issue(17).unwrap()["state"], "closed");
    assert!(provider.remote(17).unwrap().is_none());
    assert_eq!(seat.read("Example/probe").unwrap().issue, 0);
}
