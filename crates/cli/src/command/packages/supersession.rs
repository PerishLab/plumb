use super::super::super::{Seat, State, closure, issue, recovery, work};
use super::{Fixture, isolated};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const REPO: &str = "Example/probe";
const RUN: &str = "https://github.com/Example/probe/actions/runs/17";

struct Trial {
    fixture: Fixture,
    source: PathBuf,
    seat: Seat,
    state: State,
    head: String,
    comments: Vec<Value>,
}

impl Trial {
    fn new() -> Self {
        plumb::depot::carry(crate::catalog::carried::FILES);
        let fixture = Fixture::new(json!({}));
        let source = fixture.repository("pub fn answer() -> u8 { 42 }\n");
        let parent = fixture.root.path();
        let remote = parent.join("Example/probe.git");
        std::fs::create_dir(remote.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(parent.join("origin.git"), &remote).unwrap();
        work::git(
            &source,
            &["remote", "set-url", "origin", remote.to_str().unwrap()],
        )
        .unwrap();
        let seat = Seat::fixture(parent);
        let mut state = State {
            schema: "plumb.auto-state/v1".into(),
            repository: REPO.into(),
            issue: 17,
            pull: 18,
            worktree: parent.join("worktree"),
            ..Default::default()
        };
        work::Work {
            source: &source,
            state: &state,
        }
        .open()
        .unwrap();
        let manifest = state.worktree.join("Cargo.toml");
        let text = std::fs::read_to_string(&manifest).unwrap();
        std::fs::write(manifest, format!("{text}description='old Auto delta'\n")).unwrap();
        work::git(&state.worktree, &["add", "Cargo.toml"]).unwrap();
        work::git(&state.worktree, &["commit", "-qm", "first publication"]).unwrap();
        let pushed = work::git(&state.worktree, &["rev-parse", "HEAD"]).unwrap();
        let tree = work::git(&state.worktree, &["rev-parse", "HEAD^{tree}"]).unwrap();
        let base = work::git(&source, &["rev-parse", "origin/main"]).unwrap();
        let proof = descriptor(&source, &tree, 'a');
        let current = descriptor(&source, &tree, 'b');
        let candidate = commit(&source, &base, &proof, "stale candidate");
        let head = commit(&source, &base, &current, "superseding candidate");
        let merged = commit(&source, &base, &current, "squash");
        work::git(
            &source,
            &[
                "push",
                "origin",
                &format!("{head}:refs/pull/18/head"),
                &format!("{merged}:refs/heads/main"),
            ],
        )
        .unwrap();

        let provider = fixture.provider();
        provider
            .patch(
                "repos/Example/probe/issues/17",
                &json!({
                    "state":"closed","state_reason":"completed",
                    "body":issue::body(REPO).replace("- [ ]", "- [x]")
                }),
            )
            .unwrap();
        state.plan = Some(plumb::delivery::Plan {
            schema: "plumb.delivery-plan/v2".into(),
            root: state.worktree.clone(),
            repository: REPO.into(),
            issue: issue::snapshot(&provider, 17).unwrap(),
            observed: 0,
            base: "main".into(),
            target: base.clone(),
            branch: "auto/17".into(),
            projection: candidate.clone(),
            source: pushed.clone(),
            candidate: candidate.clone(),
            pull: plumb::delivery::Narrative {
                title: "Follow".into(),
                body: "Refs #17.".into(),
            },
            guard: plumb::landing::Guard {
                schema: proof.schema,
                tree: tree.clone(),
                digest: proof.digest,
            },
        });
        state.pushed = Some(pushed.clone());
        state.candidate = Some(candidate);
        seat.write(&state).unwrap();
        put(
            parent,
            "pull.json",
            &json!({
                "node_id":"fixture-pull","number":18,"state":"closed","merged":true,
                "merged_at":"2026-10-15T14:05:43Z","merge_commit_sha":merged,"body":"Refs #17.",
                "head":{"ref":"auto/17","sha":head,"repo":{"full_name":REPO}},
                "base":{"ref":"main","sha":base,"repo":{"full_name":REPO}}
            }),
        );
        put(
            parent,
            "run.json",
            &json!({
                "head_sha":head,"path":".github/workflows/guard.yml","event":"pull_request",
                "status":"completed","conclusion":"success"
            }),
        );
        let comments = vec![
            publication(None, &pushed, &base, &tree),
            publication(Some(&pushed), &head, &base, &tree),
            json!({"user":{"id":1},"body":closure::acceptance(REPO, &merged, RUN, &head)}),
        ];
        Self {
            fixture,
            source,
            seat,
            state,
            head,
            comments,
        }
    }

    fn write(&self, name: &str, value: &Value) {
        put(self.fixture.root.path(), name, value);
    }

    fn check(&self, result: &str) {
        self.write(
            "guard.json",
            &json!({
                "name":"Guard","app":{"slug":"github-actions"},"head_sha":self.head,
                "status":"completed","conclusion":result,"details_url":RUN
            }),
        );
    }

    fn settle(&self) -> Result<String, String> {
        recovery::settle(&self.source, &self.fixture.provider(), &self.seat, 17)
    }

    fn refused(&self, text: &str) {
        let before = std::fs::read(self.fixture.root.path().join("state.json")).unwrap();
        let error = self.settle().unwrap_err();
        assert!(error.contains(text), "{error}");
        assert_eq!(
            std::fs::read(self.fixture.root.path().join("state.json")).unwrap(),
            before
        );
        assert!(!self.fixture.root.path().join("recovered-17.json").exists());
    }
}

fn put(root: &Path, name: &str, value: &Value) {
    std::fs::write(root.join(name), value.to_string()).unwrap();
}

fn descriptor(root: &Path, tree: &str, world: char) -> plumb::guard::Descriptor {
    plumb::guard::Descriptor::new(
        root,
        tree.into(),
        vec![plumb::guard::Action {
            name: "guard/rust".into(),
            input: "1".repeat(64),
            world: world.to_string().repeat(64),
        }],
    )
    .unwrap()
}

fn commit(root: &Path, base: &str, proof: &plumb::guard::Descriptor, subject: &str) -> String {
    let message = format!(
        "{subject}\n\n{} {}",
        plumb::guard::TRAILER,
        proof.encode().unwrap()
    );
    work::git(
        root,
        &["commit-tree", &proof.tree, "-p", base, "-m", &message],
    )
    .unwrap()
}

fn publication(previous: Option<&str>, head: &str, base: &str, tree: &str) -> Value {
    let intent = json!({
        "schema":"plumb.auto-push/v1","repository":REPO,"issue":17,"branch":"auto/17",
        "previous":previous,"head":head,"base":base,"tree":tree
    });
    json!({"user":{"id":1},"body":format!("<!-- plumb.auto-push/v1\n{intent}\n-->")})
}

#[test]
fn qualification() {
    if !isolated("supersession::qualification") {
        return;
    }
    let trial = Trial::new();
    trial.write("comments.json", &json!(&trial.comments));
    trial.write("viewer", &json!({"databaseId":2,"login":"foreign"}));
    trial.refused("writer");
    trial.write("viewer", &json!({"databaseId":1,"login":"fixture-writer"}));
    trial.write("comments.json", &json!(&trial.comments[1..]));
    trial.refused("discontinuous");
    let mut foreign = trial.comments.clone();
    foreign[1]["user"]["id"] = json!(2);
    trial.write("comments.json", &json!(foreign));
    trial.refused("another operation writer");
    trial.write("comments.json", &json!(&trial.comments));
    trial.refused("no run on the exact Auto head");
    trial.check("failure");
    trial.refused("Guard failed");
    trial.check("success");

    let worktree = &trial.state.worktree;
    let payload = worktree.join("keep.txt");
    std::fs::write(&payload, "keep").unwrap();
    trial.refused("uncommitted local payload");
    assert_eq!(std::fs::read_to_string(&payload).unwrap(), "keep");
    std::fs::remove_file(payload).unwrap();
    work::git(worktree, &["commit", "--allow-empty", "-qm", "moved"]).unwrap();
    trial.refused("local branch moved");
    work::git(
        worktree,
        &["reset", "--hard", trial.state.pushed.as_deref().unwrap()],
    )
    .unwrap();
    work::git(
        &trial.source,
        &[
            "push",
            "origin",
            &format!("{}:refs/heads/auto/17", trial.head),
        ],
    )
    .unwrap();
    trial.refused("remote branch to be absent");
    work::git(&trial.source, &["push", "origin", ":refs/heads/auto/17"]).unwrap();

    let original = std::fs::read_to_string(trial.fixture.root.path().join("state.json")).unwrap();
    assert!(trial.settle().unwrap().contains("recovered at"));
    let receipt = trial.fixture.root.path().join("recovered-17.json");
    let bytes = std::fs::read(&receipt).unwrap();
    let observed: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(observed["state"], original);
    assert_eq!(observed["head"], trial.head);
    assert_eq!(
        observed["candidate"]["digest"],
        trial.state.plan.as_ref().unwrap().guard.digest
    );
    assert_eq!(trial.seat.read(REPO).unwrap().issue, 0);
    assert_eq!(
        work::git(worktree, &["rev-parse", "HEAD"]).unwrap(),
        trial.state.pushed.as_deref().unwrap()
    );

    trial.seat.write(&trial.state).unwrap();
    assert!(trial.settle().unwrap().contains("recovered at"));
    assert_eq!(trial.seat.read(REPO).unwrap().issue, 0);
    assert_eq!(std::fs::read(&receipt).unwrap(), bytes);

    let mut active = State {
        issue: 19,
        ..trial.seat.read(REPO).unwrap()
    };
    active.worktree = recovery::checkout(&trial.source, &trial.seat, &active).unwrap();
    work::Work {
        source: &trial.source,
        state: &active,
    }
    .open()
    .unwrap();
    trial.seat.write(&active).unwrap();
    let before = trial.fixture.calls().len();
    assert!(trial.settle().unwrap().contains("already recorded"));
    assert_eq!(trial.seat.read(REPO).unwrap().issue, 19);
    assert_eq!(std::fs::read(receipt).unwrap(), bytes);
    assert!(
        trial.fixture.calls()[before..]
            .iter()
            .all(|call| call["payload"].is_null())
    );
}
