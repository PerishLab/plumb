use super::super::{Seat, State, closure, engine, provider::Intent, work};
use super::{Fixture, alone, issue};
use serde_json::{Value, json};
use std::path::Path;

impl Fixture {
    fn held(&self) -> (std::path::PathBuf, Seat, State) {
        plumb::depot::carry(crate::catalog::carried::FILES);
        let root = self.repository("pub fn answer() -> u8 { 42 }\n");
        self.provider()
            .patch(
                "repos/Example/probe/issues/17",
                &json!({"body":issue::legacy("Example/probe")}),
            )
            .unwrap();
        let seat = Seat::fixture(self.root.path());
        let state = State {
            schema: "plumb.auto-state/v1".into(),
            repository: "Example/probe".into(),
            issue: 17,
            worktree: self.root.path().join("worktree"),
            ..Default::default()
        };
        (root, seat, state)
    }

    fn intent(&self, root: &Path, head: &str) -> Intent {
        Intent {
            schema: "plumb.auto-push/v1".into(),
            repository: "Example/probe".into(),
            issue: 17,
            branch: "auto/17".into(),
            previous: None,
            head: head.into(),
            base: head.into(),
            tree: work::git(root, &["rev-parse", &format!("{head}^{{tree}}")]).unwrap(),
        }
    }

    fn publish(&self, root: &Path, state: &mut State) -> String {
        let head = work::git(root, &["rev-parse", "origin/main"]).unwrap();
        self.provider()
            .intent(17, &self.intent(root, &head))
            .unwrap();
        work::git(
            root,
            &["push", "origin", &format!("{head}:refs/heads/auto/17")],
        )
        .unwrap();
        state.pushed = Some(head.clone());
        head
    }

    fn completed(&self, root: &Path, seat: &Seat) {
        let provider = self.provider();
        let current = provider.issue(17).unwrap();
        assert_eq!(current["state"], "closed");
        assert_eq!(
            current["body"],
            issue::body("Example/probe").replace("- [ ]", "- [x]")
        );
        assert_eq!(seat.read("Example/probe").unwrap().issue, 0);
        assert!(provider.remote(17).unwrap().is_none());
        assert!(provider.find(17).unwrap().is_none());
        let comments = provider
            .pages("repos/Example/probe/issues/17/comments?per_page=100")
            .unwrap();
        let receipts = comments
            .iter()
            .filter(|comment| {
                comment["body"]
                    .as_str()
                    .unwrap()
                    .contains("plumb.auto-noop/v1")
            })
            .collect::<Vec<_>>();
        assert_eq!(receipts.len(), 1);
        assert!(
            receipts[0]["body"]
                .as_str()
                .unwrap()
                .contains("No dependency change, pull, merge or Guard was produced")
        );
        assert!(!self.root.path().join("worktree").exists());
        assert!(
            self.calls()
                .iter()
                .all(|call| !call["endpoint"].as_str().unwrap().contains("check-runs"))
        );
        assert!(
            work::git(root, &["status", "--porcelain"])
                .unwrap()
                .is_empty()
        );
    }
}
impl Fixture {
    fn refuse(&self, root: &Path, state: &mut State, variant: &str) {
        let head = work::git(root, &["rev-parse", "origin/main"]).unwrap();
        match variant {
            "unknown" => {
                std::fs::write(self.root.path().join("comments.json"), "[]").unwrap();
                state.pushed = None;
            }
            "history" => {
                let mut held = self.intent(root, &head);
                held.previous = Some(head);
                self.provider().intent(17, &held).unwrap();
            }
            "foreign" => {
                let path = self.root.path().join("comments.json");
                let mut comments: Value =
                    serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                comments[0]["user"]["id"] = json!(2);
                std::fs::write(path, comments.to_string()).unwrap();
            }
            "nonempty" | "moved" => {
                std::fs::write(root.join("src/lib.rs"), "pub fn answer() -> u8 { 43 }\n").unwrap();
                work::git(root, &["add", "."]).unwrap();
                work::git(root, &["commit", "-qm", "valuable foreign source"]).unwrap();
                let changed = work::git(root, &["rev-parse", "HEAD"]).unwrap();
                work::git(
                    root,
                    &["push", "origin", &format!("{changed}:refs/heads/auto/17")],
                )
                .unwrap();
                if variant == "nonempty" {
                    let mut held = self.intent(root, &changed);
                    held.base = head;
                    self.provider().intent(17, &held).unwrap();
                    state.pushed = Some(changed);
                }
            }
            "pull" => {
                state.pull = 18;
            }
            "payload" => {
                work::Work {
                    source: root,
                    state,
                }
                .open()
                .unwrap();
                std::fs::write(state.worktree.join("valuable.txt"), "preserve me").unwrap();
            }
            _ => unreachable!(),
        }
    }
}
#[test]
fn qualified() {
    let fixture = Fixture::new(json!({}));
    let (root, seat, mut state) = fixture.held();
    let main = work::git(&root, &["rev-parse", "origin/main"]).unwrap();
    let message = engine::advance(&root, &fixture.provider(), &seat, &mut state).unwrap();
    assert!(message.contains("qualified no-op"));
    assert_eq!(
        work::git(&root, &["rev-parse", "origin/main"]).unwrap(),
        main
    );
    fixture.completed(&root, &seat);
}

#[test]
fn interrupted() {
    let fixture = Fixture::new(json!({}));
    let (root, seat, mut state) = fixture.held();
    let head = fixture.publish(&root, &mut state);
    std::fs::write(fixture.root.path().join("commentfault"), "once").unwrap();
    let error = engine::advance(&root, &fixture.provider(), &seat, &mut state).unwrap_err();
    assert!(
        error.contains("interrupted before no-op receipt"),
        "{error}"
    );
    assert!(fixture.provider().remote(17).unwrap().is_none());
    assert_eq!(fixture.provider().issue(17).unwrap()["state"], "open");
    std::fs::write(fixture.root.path().join("closefault"), "once").unwrap();
    let error = engine::advance(&root, &fixture.provider(), &seat, &mut state).unwrap_err();
    assert!(error.contains("interrupted after closure write"), "{error}");
    assert_eq!(fixture.provider().issue(17).unwrap()["state"], "closed");
    assert!(closure::resume(&root, &fixture.provider(), &seat, &mut state).unwrap());
    fixture.completed(&root, &seat);
    state = State {
        schema: "plumb.auto-state/v1".into(),
        repository: "Example/probe".into(),
        issue: 17,
        pushed: Some(head),
        ..Default::default()
    };
    assert!(closure::resume(&root, &fixture.provider(), &seat, &mut state).unwrap());
    fixture.completed(&root, &seat);
}

#[test]
fn refused() {
    for variant in [
        "foreign", "nonempty", "moved", "pull", "payload", "unknown", "history",
    ] {
        let fixture = Fixture::new(json!({}));
        let (root, seat, mut state) = fixture.held();
        fixture.publish(&root, &mut state);
        fixture.refuse(&root, &mut state, variant);
        let before = fixture.provider().remote(17).unwrap();
        assert!(
            engine::advance(&root, &fixture.provider(), &seat, &mut state).is_err(),
            "{variant}"
        );
        assert_eq!(fixture.provider().remote(17).unwrap(), before, "{variant}");
        assert_eq!(
            fixture.provider().issue(17).unwrap()["state"],
            "open",
            "{variant}"
        );
        if variant == "payload" {
            assert_eq!(
                std::fs::read_to_string(state.worktree.join("valuable.txt")).unwrap(),
                "preserve me"
            );
        }
    }
}

#[test]
fn concurrent() {
    if !alone("noop::concurrent") {
        return;
    }
    let fixture = Fixture::new(json!({}));
    let (root, seat, mut state) = fixture.held();
    fixture.publish(&root, &mut state);
    work::git(
        &root,
        &["commit", "--allow-empty", "-qm", "Concurrent main"],
    )
    .unwrap();
    let moved = work::git(&root, &["rev-parse", "HEAD"]).unwrap();
    work::git(
        &root,
        &["push", "origin", &format!("{moved}:refs/heads/pending")],
    )
    .unwrap();
    std::fs::write(fixture.root.path().join("move"), moved).unwrap();
    let error = engine::advance(&root, &fixture.provider(), &seat, &mut state).unwrap_err();
    assert!(error.contains("current main moved"), "{error}");
    assert_eq!(fixture.provider().issue(17).unwrap()["state"], "open");
    assert!(fixture.provider().remote(17).unwrap().is_none());
    work::git(&root, &["fetch", "origin"]).unwrap();
    engine::advance(&root, &fixture.provider(), &seat, &mut state).unwrap();
    assert_eq!(fixture.provider().issue(17).unwrap()["state"], "closed");
}

#[test]
fn stranded() {
    if !alone("noop::stranded") {
        return;
    }
    for variant in ["clean", "dirty", "commit"] {
        let fixture = Fixture::new(json!({}));
        let (root, seat, mut state) = fixture.held();
        let head = fixture.publish(&root, &mut state);
        std::fs::write(root.join("README.md"), "later main\n").unwrap();
        work::git(&root, &["add", "README.md"]).unwrap();
        work::git(&root, &["commit", "-qm", "Later main"]).unwrap();
        work::git(&root, &["push", "-q", "origin", "HEAD:main"]).unwrap();
        engine::advance(&root, &fixture.provider(), &seat, &mut state).unwrap();
        fixture.completed(&root, &seat);
        state.issue = 17;
        state.pushed = Some(head);
        state.worktree = fixture.root.path().join("worktree");
        let worktree = state.worktree.clone();
        work::Work {
            source: &root,
            state: &state,
        }
        .open()
        .unwrap();
        if variant != "clean" {
            std::fs::write(worktree.join("valuable.txt"), "preserve me").unwrap();
        }
        if variant == "commit" {
            work::git(&worktree, &["add", "valuable.txt"]).unwrap();
            work::git(&worktree, &["commit", "-qm", "valuable local commit"]).unwrap();
        }
        let resumed = closure::resume(&root, &fixture.provider(), &seat, &mut state);
        if variant == "clean" {
            assert!(resumed.unwrap());
            assert!(!worktree.exists());
            assert_eq!(seat.read("Example/probe").unwrap().issue, 0);
        } else {
            let error = resumed.unwrap_err();
            assert!(error.contains("preserves valuable"), "{variant}: {error}");
            let kept = std::fs::read_to_string(worktree.join("valuable.txt")).unwrap();
            assert_eq!(kept, "preserve me");
        }
    }
}
